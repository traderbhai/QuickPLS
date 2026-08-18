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
  compareUtf8StringsV1,
  compileCbsemPlanV2,
  compilePlsPlanV2,
  SemModelV4OperationError,
  validateSemModelV4,
  type SemModelV4,
  type SemRelationV4,
} from "./semModelV4";
import { sha256HexBytesV1 } from "./sha256V1";

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
  capability_id: "smartpls.pls_bootstrapping",
  cell_id: "qpls3.inference.bootstrap",
  capability_version: "indexed_resampling_v4",
} as const;

const CBSEM_CELL = {
  registry_schema_version: 2,
  capability_id: "smartpls.cbsem",
  cell_id: "qpls3.cbsem.ml",
  capability_version: "cbsem_ml_v1",
} as const;

const PLS_EVIDENCE: readonly SemCapabilityEvidenceV1[] = [
  {
    evidence_id: "capability_registry_v2:smartpls.mediation:qpls3.pls.mediation:pls_mediation_v1",
    description: "Capability Registry V2 exposes the exact mediation option in Experimental Labs.",
  },
  {
    evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_v1",
    description: "The versioned PLS v3 compiler preserves the proven v2 scoring plan and adds stable topology and effect identities.",
  },
];

const PLS_BOOTSTRAP_EVIDENCE: SemCapabilityEvidenceV1 = {
  evidence_id: "capability_registry_v2:smartpls.pls_bootstrapping:qpls3.inference.bootstrap:indexed_resampling_v4",
  description: "Capability Registry V2 exposes the bounded indexed case-resampling primitive used by this General SEM compiler slice.",
};

const PLS_BOOTSTRAP_COMPILER_EVIDENCE: SemCapabilityEvidenceV1 = {
  evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_bootstrap_v1",
  description: "The bootstrap compiler binds exact Recipe V4 inference settings to the General SEM config while retaining the proven point-scoring plan.",
};

const CBSEM_EVIDENCE: readonly SemCapabilityEvidenceV1[] = [
  {
    evidence_id: "capability_registry_v2:smartpls.cbsem:qpls3.cbsem.ml:cbsem_ml_v1",
    description: "Capability Registry V2 is the exact authority for the bounded CB-SEM ML cell.",
  },
  {
    evidence_id: "compiler:compiled_cbsem_plan_v3",
    description: "CB-SEM v3 preserves the complete v2 parameter table and adds SCC and identification evidence without implying execution support.",
  },
];

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
function specificDirectedPathIdentityV1(relationIds: readonly string[]): string {
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

function executionScopeDiagnostics(config: GeneralSemConfigV1): SemCapabilityDiagnosticV1[] {
  const diagnostics: SemCapabilityDiagnosticV1[] = [];
  if (config.conditional_effect_probes.length > 0) {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.conditional_probes_not_executable",
      "Conditional-effect probes are authored but are not executable in the current PLS v3 point-estimation slice.",
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

function plsShapeDiagnostics(model: SemModelV4): SemCapabilityDiagnosticV1[] {
  const diagnostics: SemCapabilityDiagnosticV1[] = [];
  if (model.variables.some((variable) => variable.kind === "common_factor")) {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.common_factor_not_executable",
      "The current General SEM PLS executor requires structural constructs to be composites; the authored model contains common factors.",
      "Use composite constructs for this PLS request, or retain the factor model for a qualified CB-SEM capability cell.",
    ));
  }
  if (model.variables.some((variable) => variable.kind === "derived")
    || model.derived_terms.length > 0
    || model.parameters.some((parameter) => parameter.kind === "derived")) {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.derived_shape_not_executable",
      "The authored model contains derived variables, terms, or parameters that the current PLS v3 executor does not calculate.",
      "Keep the derived semantics on the canvas, but remove them from this calculation or wait for their exact qualified execution cell.",
    ));
  }
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

/**
 * Exact preflight for the General SEM PLS point-estimation and bounded
 * percentile case-bootstrap compiler slices. The inputs are never rewritten.
 */
export function preflightGeneralSemPlsV1(
  model: SemModelV4,
  config: GeneralSemConfigV1,
): SemCapabilityDecisionV1 {
  const validatedConfig = parseGeneralSemConfigV1(config);
  const bootstrapRequested = validatedConfig.inference.kind === "case_bootstrap";
  const capabilityCells = bootstrapRequested
    ? [PLS_CELL, PLS_BOOTSTRAP_CELL]
    : [PLS_CELL];
  const evidence = bootstrapRequested
    ? [...PLS_EVIDENCE, PLS_BOOTSTRAP_COMPILER_EVIDENCE, PLS_BOOTSTRAP_EVIDENCE]
    : PLS_EVIDENCE;
  const diagnostics = executionScopeDiagnostics(validatedConfig);
  diagnostics.push(...plsShapeDiagnostics(model));

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

  let basePlanCompiles = false;
  if (modelIsValid && !hasFeedback && diagnostics.every((item) => (
    item.code !== "sem.capability.pls.common_factor_not_executable"
    && item.code !== "sem.capability.pls.derived_shape_not_executable"
  ))) {
    try {
      compilePlsPlanV2(model);
      basePlanCompiles = true;
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
      diagnostics.push(...requestedEffectDiagnostics(model, validatedConfig, enumeration.paths));
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

  return createSemCapabilityDecisionV1({
    status: "experimental",
    estimator_id: GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
    capability_cells: capabilityCells,
    diagnostics: [{
      code: "sem.capability.pls.experimental_labs",
      severity: "info",
      subject: null,
      message: bootstrapRequested
        ? "General recursive PLS percentile case-bootstrap inference passes the bounded Experimental Labs compiler preflight."
        : "General recursive PLS point estimation and path-specific effects pass the Experimental Labs compiler preflight.",
      corrections: [],
    }],
    evidence,
    summary: "PLS-SEM can compile this exact request in Experimental Labs.",
    explanation: bootstrapRequested
      ? "The compiler binds percentile, two-sided case resampling to both the mediation and indexed-resampling cells. Runtime inference must carry a matching complete-model re-estimation receipt before publication."
      : "The compiler binds the proven PLS scoring plan to stable relation-path identities. Runtime validation remains authoritative before a result can be published.",
  });
}

/**
 * CB-SEM v3 can describe topology and identification, but its General SEM
 * runtime adapter is deliberately unavailable until the exact cell is qualified.
 */
export function preflightGeneralSemCbsemV1(
  model: SemModelV4,
  config: GeneralSemConfigV1,
): SemCapabilityDecisionV1 {
  const validatedConfig = parseGeneralSemConfigV1(config);
  const diagnostics: SemCapabilityDiagnosticV1[] = [];
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
    if (structuralSccs(model).some((component) => component.hasFeedback)) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.cbsem.feedback_execution_blocked",
        "The reciprocal block is preserved, but the current CB-SEM executor is not qualified to estimate feedback systems.",
        "Remove the reciprocal path to create a recursive model, or retain the model until the identified feedback capability is qualified.",
      ));
    } else {
      diagnostics.push(errorDiagnostic(
        "sem.capability.cbsem.general_runtime_not_connected",
        "The CB-SEM v3 parameter and identification plan is available, but the General SEM runtime adapter is not connected.",
        "Use the currently qualified bounded CB-SEM workflow, or keep this request in Labs until the v3 adapter is qualified.",
      ));
    }
  }

  return createSemCapabilityDecisionV1({
    status: "blocked",
    estimator_id: GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
    capability_cells: [CBSEM_CELL],
    diagnostics,
    evidence: CBSEM_EVIDENCE,
    summary: "CB-SEM cannot calculate this exact General SEM request yet.",
    explanation: "Compilation and identification diagnostics remain visible, while execution stays disabled until the exact runtime cell is qualified.",
  });
}
