import type { AnalysisMethodId, AnalysisUiSettings } from "../types";
import {
  capabilityRegistryV2,
  type QuickPlsCellMatchV2,
} from "./capabilityRegistryV2";
import type { CapabilityAvailabilityV2 } from "./capabilitySurfaceV2";
import { establishedMethodContractV1 } from "./generated/establishedMethodContractsV1";

/**
 * Exact option-cell dependency selected by a concrete analysis recipe.
 *
 * `cell_id` alone is intentionally not an identity: PCA, prediction, and
 * group workflows contain cells shared by more than one official catalogue
 * row. The bridge therefore verifies the pair against Capability Registry V2.
 */
export interface MethodCapabilityRequirementV2 {
  readonly capability_id: string;
  readonly cell_id: string;
  readonly option: string;
}

export type MethodProductTierV2 = "standard" | "experimental" | "hidden";

export type MethodCapabilityReasonV2 =
  | "all_required_cells_standard"
  | "all_required_cells_labs_visible"
  | "experimental_labs_disabled"
  | "required_cells_not_executable"
  | "registry_mapping_missing"
  | "unknown_method_mapping"
  | "unknown_option_mapping";

/**
 * Customer-safe aggregate. Scientific governance states remain inside the
 * registry; product code receives only a display tier and actionable IDs.
 */
export interface MethodCapabilityAvailabilityV2 {
  readonly method_id: string;
  readonly tier: MethodProductTierV2;
  readonly selectable: boolean;
  readonly label: "Supported" | "Experimental" | null;
  readonly required_cell_ids: readonly string[];
  readonly required_capability_ids: readonly string[];
  readonly blocked_cell_ids: readonly string[];
  readonly blocked_capability_ids: readonly string[];
  readonly internal_reason: MethodCapabilityReasonV2;
}

export interface MethodCapabilityRegistryReaderV2 {
  quickPlsCell(cellId: string): readonly QuickPlsCellMatchV2[];
  availability(capabilityId: string, cellId: string, experimentalLabsEnabled: boolean): CapabilityAvailabilityV2;
}

export interface MethodCapabilityAvailabilityOptionsV2 {
  readonly experimentalLabsEnabled: boolean;
  readonly registry?: MethodCapabilityRegistryReaderV2;
}

export class MethodCapabilityMappingErrorV2 extends Error {
  readonly code: "unknown_method_mapping" | "unknown_option_mapping";

  constructor(code: MethodCapabilityMappingErrorV2["code"], message: string) {
    super(message);
    this.name = "MethodCapabilityMappingErrorV2";
    this.code = code;
  }
}

const requirement = (
  capability_id: string,
  cell_id: string,
  option: string,
): MethodCapabilityRequirementV2 => Object.freeze({ capability_id, cell_id, option });

const PLS_ALGORITHM = requirement("smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_algorithm");

const SIMPLE_REQUIREMENTS = {
  pls_pm: [PLS_ALGORITHM],
  pls_sample_size_power: [
    requirement("smartpls.pls_power_analysis", "qpls3.pls.sample_size_power", "prospective_sample_size_power"),
  ],
  wpls: [requirement("smartpls.wpls", "qpls3.pls.weighted", "weighted_pls")],
  pca: [
    requirement("smartpls.pca_core", "qpls3.standalone.pca", "pca_core_catalogue_row"),
    requirement("smartpls.pca_cbsem", "qpls3.standalone.pca", "pca_cbsem_catalogue_row"),
  ],
} as const satisfies Partial<Record<AnalysisMethodId, readonly MethodCapabilityRequirementV2[]>>;

const GROUP_OPTION_REQUIREMENTS = {
  micom: requirement("smartpls.micom", "qpls3.groups.micom_permutation_mga", "micom"),
  mga_permutation: requirement("smartpls.mga", "qpls3.groups.micom_permutation_mga", "mga_permutation"),
  pls_pos: requirement("smartpls.pls_pos", "qpls3.segmentation.pls_pos", "pls_pos"),
  fimix: requirement("smartpls.fimix_pls", "qpls3.segmentation.fimix_pls", "fimix_pls"),
} as const;

type GroupOptionV2 = keyof typeof GROUP_OPTION_REQUIREMENTS;

function freezeRequirements(
  values: readonly MethodCapabilityRequirementV2[],
): readonly MethodCapabilityRequirementV2[] {
  const seen = new Set<string>();
  const unique: MethodCapabilityRequirementV2[] = [];
  for (const value of values) {
    const identity = `${value.capability_id}::${value.cell_id}`;
    if (seen.has(identity)) continue;
    seen.add(identity);
    unique.push(Object.freeze({ ...value }));
  }
  return Object.freeze(unique);
}

function establishedMethodRequirementsV1(
  method: string,
): readonly MethodCapabilityRequirementV2[] | null {
  const contract = establishedMethodContractV1(method, method);
  if (!contract) return null;
  return freezeRequirements(contract.capability_requirements.map((item) => requirement(
    item.capability_id,
    item.cell_id,
    item.option,
  )));
}

function nonblank(value: string | null | undefined): boolean {
  return typeof value === "string" && value.trim().length > 0;
}

function groupOptions(value: string | null | undefined): readonly GroupOptionV2[] {
  if (!nonblank(value)) return Object.freeze([]);
  const result: GroupOptionV2[] = [];
  const seen = new Set<string>();
  for (const rawToken of value!.split(",")) {
    const token = rawToken.trim().toLowerCase();
    if (!token || seen.has(token)) continue;
    if (!Object.hasOwn(GROUP_OPTION_REQUIREMENTS, token)) {
      throw new MethodCapabilityMappingErrorV2("unknown_option_mapping", `Unknown group-method option: ${rawToken.trim()}`);
    }
    seen.add(token);
    result.push(token as GroupOptionV2);
  }
  return Object.freeze(result);
}

function plsDependent(
  ...specific: readonly MethodCapabilityRequirementV2[]
): readonly MethodCapabilityRequirementV2[] {
  return freezeRequirements([PLS_ALGORITHM, ...specific]);
}

function predictionRequirements(settings: Readonly<AnalysisUiSettings>): readonly MethodCapabilityRequirementV2[] {
  const segmentation = groupOptions(settings.groupMethods)
    .filter((option) => option === "pls_pos" || option === "fimix")
    .map((option) => GROUP_OPTION_REQUIREMENTS[option]);
  return plsDependent(
    requirement("smartpls.plspredict", "qpls3.prediction.plspredict_cvpat", "plspredict"),
    requirement("smartpls.cvpat", "qpls3.prediction.plspredict_cvpat", "cvpat"),
    ...segmentation,
  );
}

function groupRequirements(settings: Readonly<AnalysisUiSettings>): readonly MethodCapabilityRequirementV2[] {
  const selected = groupOptions(settings.groupMethods);
  const effective = selected.length > 0 ? selected : (["micom", "mga_permutation"] as const);
  return plsDependent(...effective.map((option) => GROUP_OPTION_REQUIREMENTS[option]));
}

function cbsemRequirements(settings: Readonly<AnalysisUiSettings>): readonly MethodCapabilityRequirementV2[] {
  const selectedModel = settings.cbsemModelType === "cfa"
    ? requirement("smartpls.cfa", "qpls3.cbsem.ml", "cfa_ml")
    : requirement("smartpls.cbsem", "qpls3.cbsem.ml", "sem_ml");
  const values: MethodCapabilityRequirementV2[] = [selectedModel];
  if ((settings.cbsemBootstrapSamples ?? 0) > 0) {
    values.push(requirement("smartpls.cbsem_bootstrapping", "qpls3.cbsem.bootstrap", "cbsem_bootstrap"));
  }
  if (nonblank(settings.cbsemGroupColumn)) {
    values.push(requirement("smartpls.cbsem_mga", "qpls3.cbsem.multigroup", "cbsem_multigroup"));
    if (nonblank(settings.cbsemInvarianceSteps)) {
      values.push(requirement(
        "smartpls.cbsem_measurement_invariance",
        "qpls3.cbsem.measurement_invariance",
        "cbsem_measurement_invariance",
      ));
    }
  }
  return freezeRequirements(values);
}

function regressionRequirements(settings: Readonly<AnalysisUiSettings>): readonly MethodCapabilityRequirementV2[] {
  const type = settings.regressionType ?? "ols";
  const values: MethodCapabilityRequirementV2[] = [];
  if (type === "ols") {
    values.push(requirement("smartpls.regression", "qpls3.standalone.ols", "ols"));
  } else if (type === "logistic") {
    values.push(requirement("smartpls.logistic_regression", "qpls3.standalone.logistic", "logistic"));
  } else if (type === "process") {
    values.push(requirement("smartpls.process", "qpls3.standalone.process", "process"));
  } else {
    throw new MethodCapabilityMappingErrorV2("unknown_option_mapping", `Unknown regression type: ${String(type)}`);
  }

  if (settings.regressionBootstrap === true) {
    values.push(type === "process"
      ? requirement("smartpls.process_bootstrapping", "qpls3.standalone.process", "process_bootstrap")
      : requirement(
        "smartpls.regression_bootstrapping",
        "qpls3.standalone.regression_bootstrap",
        "regression_bootstrap",
      ));
  }
  return freezeRequirements(values);
}

/** Resolve a concrete method/settings request into exact registry identities. */
export function methodCapabilityRequirementsV2(
  settings: Readonly<AnalysisUiSettings>,
): readonly MethodCapabilityRequirementV2[] {
  const method = settings.method;
  if (method === "pls_pm" && settings.posthocTechnicalMinimumSampleSize === true) {
    return plsDependent(
      requirement("smartpls.pls_bootstrapping", "qpls3.inference.bootstrap", "pls_bootstrap"),
      requirement(
        "smartpls.pls_power_analysis",
        "qpls3.pls.posthoc_technical_minimum_sample_size",
        "post_hoc_technical_minimum_sample_size",
      ),
    );
  }
  if (method === "plsc") {
    const values = [requirement("smartpls.plsc", "qpls3.pls.consistent", "consistent_pls")];
    if (settings.bootstrapSamples > 0) {
      values.push(requirement(
        "smartpls.consistent_bootstrapping",
        "qpls3.inference.consistent_bootstrap",
        "consistent_bootstrap",
      ));
    }
    if (settings.permutationSamples > 0) {
      values.push(requirement(
        "smartpls.consistent_permutation",
        "qpls3.inference.consistent_permutation",
        "consistent_permutation",
      ));
    }
    return freezeRequirements(values);
  }
  const established = establishedMethodRequirementsV1(method);
  if (established) return established;
  if (Object.hasOwn(SIMPLE_REQUIREMENTS, method)) {
    return freezeRequirements(SIMPLE_REQUIREMENTS[method as keyof typeof SIMPLE_REQUIREMENTS]);
  }

  switch (method) {
    case "bootstrap":
      return plsDependent(requirement("smartpls.pls_bootstrapping", "qpls3.inference.bootstrap", "pls_bootstrap"));
    case "permutation":
      return plsDependent(requirement(
        "smartpls.permutation",
        "qpls3.inference.structural_path_randomization",
        "structural_path_randomization",
      ));
    case "cta_pls":
      return plsDependent(requirement("smartpls.cta_pls", "qpls3.assessment.cta_pls", "cta_pls"));
    case "endogeneity":
      return plsDependent(requirement(
        "smartpls.endogeneity_gaussian_copulas",
        "qpls3.pls.gaussian_copula_endogeneity",
        "gaussian_copula_endogeneity",
      ));
    case "nonlinear_effects":
      return plsDependent(requirement(
        "smartpls.nonlinear_relationships",
        "qpls3.pls.nonlinear_quadratic",
        "nonlinear_relationships",
      ));
    case "moderated_mediation":
      return plsDependent(
        requirement("smartpls.moderation", "qpls3.pls.moderation", "moderation"),
        requirement("smartpls.mediation", "qpls3.pls.mediation", "mediation"),
      );
    case "predict":
      return predictionRequirements(settings);
    case "mga":
      return groupRequirements(settings);
    case "cbsem":
      return cbsemRequirements(settings);
    case "regression":
      return regressionRequirements(settings);
    default:
      throw new MethodCapabilityMappingErrorV2(
        "unknown_method_mapping",
        `Unknown analysis-method mapping: ${String(method)}`,
      );
  }
}

function exactRegistryMatch(
  registry: MethodCapabilityRegistryReaderV2,
  required: MethodCapabilityRequirementV2,
): QuickPlsCellMatchV2 | undefined {
  return registry.quickPlsCell(required.cell_id).find((match) => (
    match.row.capability_id === required.capability_id
    && match.cell.capability_id === required.capability_id
    && match.cell.cell_id === required.cell_id
    && match.link.capability_id === required.capability_id
    && match.link.cell_id === required.cell_id
  ));
}

function uniqueStrings(values: readonly string[]): readonly string[] {
  return Object.freeze([...new Set(values)]);
}

function hiddenProjection(
  methodId: string,
  reason: MethodCapabilityReasonV2,
  requirements: readonly MethodCapabilityRequirementV2[] = [],
  blocked: readonly MethodCapabilityRequirementV2[] = requirements,
): MethodCapabilityAvailabilityV2 {
  return Object.freeze({
    method_id: methodId,
    tier: "hidden",
    selectable: false,
    label: null,
    required_cell_ids: uniqueStrings(requirements.map((item) => item.cell_id)),
    required_capability_ids: uniqueStrings(requirements.map((item) => item.capability_id)),
    blocked_cell_ids: uniqueStrings(blocked.map((item) => item.cell_id)),
    blocked_capability_ids: uniqueStrings(blocked.map((item) => item.capability_id)),
    internal_reason: reason,
  });
}

/**
 * Aggregate exact option cells into one method surface decision.
 *
 * Standard requires every selected option to be Standard-ready. Experimental
 * requires every selected option to be visible with Labs enabled and at least
 * one experimental option. Any absent add-on hides the combined request.
 */
export function methodCapabilityAvailabilityV2(
  settings: Readonly<AnalysisUiSettings>,
  options: MethodCapabilityAvailabilityOptionsV2,
): MethodCapabilityAvailabilityV2 {
  const methodId = String((settings as { method?: unknown }).method ?? "");
  let requirements: readonly MethodCapabilityRequirementV2[];
  try {
    requirements = methodCapabilityRequirementsV2(settings);
  } catch (error) {
    if (error instanceof MethodCapabilityMappingErrorV2) {
      return hiddenProjection(methodId, error.code);
    }
    return hiddenProjection(methodId, "unknown_method_mapping");
  }

  const registry = options.registry ?? capabilityRegistryV2;
  const missing: MethodCapabilityRequirementV2[] = [];
  const resolved: Array<{
    requirement: MethodCapabilityRequirementV2;
    availability: CapabilityAvailabilityV2;
  }> = [];
  for (const required of requirements) {
    if (!exactRegistryMatch(registry, required)) {
      missing.push(required);
      continue;
    }
    try {
      resolved.push({
        requirement: required,
        availability: registry.availability(
          required.capability_id,
          required.cell_id,
          options.experimentalLabsEnabled,
        ),
      });
    } catch {
      missing.push(required);
    }
  }
  if (missing.length > 0) {
    return hiddenProjection(methodId, "registry_mapping_missing", requirements, missing);
  }

  if (resolved.every((item) => item.availability.visibility === "supported")) {
    return Object.freeze({
      method_id: methodId,
      tier: "standard",
      selectable: true,
      label: "Supported",
      required_cell_ids: uniqueStrings(requirements.map((item) => item.cell_id)),
      required_capability_ids: uniqueStrings(requirements.map((item) => item.capability_id)),
      blocked_cell_ids: Object.freeze([]),
      blocked_capability_ids: Object.freeze([]),
      internal_reason: "all_required_cells_standard",
    });
  }

  if (resolved.every((item) => item.availability.visibility !== "hidden")
      && resolved.some((item) => item.availability.visibility === "experimental")) {
    return Object.freeze({
      method_id: methodId,
      tier: "experimental",
      selectable: true,
      label: "Experimental",
      required_cell_ids: uniqueStrings(requirements.map((item) => item.cell_id)),
      required_capability_ids: uniqueStrings(requirements.map((item) => item.capability_id)),
      blocked_cell_ids: Object.freeze([]),
      blocked_capability_ids: Object.freeze([]),
      internal_reason: "all_required_cells_labs_visible",
    });
  }

  const blocked = resolved
    .filter((item) => item.availability.visibility === "hidden")
    .map((item) => item.requirement);
  const allLabsDisabled = blocked.length > 0 && resolved
    .filter((item) => item.availability.visibility === "hidden")
    .every((item) => item.availability.reason === "labs_disabled");
  return hiddenProjection(
    methodId,
    allLabsDisabled ? "experimental_labs_disabled" : "required_cells_not_executable",
    requirements,
    blocked,
  );
}
