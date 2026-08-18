import type {
  AnalysisResultEnvelope,
  AnalysisRun,
  NativeCanonicalAnalysisRecipe,
  PlsPermutationRun,
} from "../types";

export const NATIVE_PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION = "plsc_permutation_v1";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION = "indexed_group_label_permutation_v1";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_OPERATION = "plsc_group_label_permutation_v1";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_TEST_V1 = "two_tailed_absolute_difference_plus_one_v1";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_TEST = "two_tailed_and_directional_difference_plus_one_v2";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_METHOD_VERSION = "plsc_directional_permutation_v1";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_TEST = "directed_greater_less_plus_one_v1";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION = "plsc_permutation_selected_tail_v1";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_ORIENTATION = "group_a_minus_group_b";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_RETRY_POLICY = "no_retry_no_replacement_fixed_indexed_labels_v1";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_MINIMUM_USABLE_FRACTION = 0.9;
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_SIGNIFICANCE_LEVEL = 0.05;
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_FULL_REFIT_WARNING =
  "Consistent permutation re-estimated complete plsc_v2 models for both original groups and for both groups in every fixed label assignment; ordinary PLS permutation estimates were not reused.";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_FAILURE_LEDGER_WARNING =
  "Failed or inadmissible PLSc group refits were retained in the fixed permutation ledger without retry, replacement, clamping, or ordinary-PLS fallback.";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING_V1 =
  "This internal v1 result reports two-tailed PLSc group-parameter differences only; MICOM, one-tailed inference, outer-weight/effect breadth, and more than two groups are not implemented.";
export const NATIVE_PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING =
  "This internal v1 result reports two-tailed and directed greater/less PLSc group-parameter differences; MICOM, outer-weight/effect breadth, and more than two groups are not implemented.";

const SHA256 = /^[0-9a-f]{64}$/;
const FAILURE_REASONS = new Set([
  "cancelled",
  "inadmissible_rho_a",
  "inadmissible_corrected_correlation",
  "plsc_nonconvergence",
  "singular_plsc_equation",
  "nonfinite_plsc_parameter",
  "parameter_identity_mismatch",
  "assignment_length_mismatch",
  "assignment_label_invalid",
  "plsc_group_refit_failed",
]);

type ParameterFamily = NonNullable<PlsPermutationRun["parameters"][number]["family"]>;

export interface NativePlscConsistentPermutationProjection {
  readonly permutation: PlsPermutationRun;
  readonly selectedTailInference: NonNullable<PlsPermutationRun["selected_tail_inference"]> | null;
  readonly requestedPermutations: number;
  readonly usablePermutations: number;
  readonly failedPermutations: number;
  readonly minimumUsablePermutations: number;
  readonly successfulLedgerEntries: number;
  readonly parameterCounts: Readonly<Record<ParameterFamily, number>>;
}

export function isNativePlscConsistentPermutationIdentityPresent(
  recipe: NativeCanonicalAnalysisRecipe,
  envelope: AnalysisResultEnvelope,
): boolean {
  const versions = new Set(envelope.provenance.method_version.split("+"));
  const artifactMethod = envelope.payload.kind === "pls_pm_v3"
    ? envelope.payload.permutation?.method_version
    : undefined;
  return (recipe.settings.method === "plsc" && recipe.settings.permutation_samples > 0)
    || versions.has(NATIVE_PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION)
    || versions.has(NATIVE_PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION)
    || artifactMethod === NATIVE_PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION;
}

export function nativePlscConsistentPermutationRecipeMatches(
  recipe: NativeCanonicalAnalysisRecipe,
  envelope: AnalysisResultEnvelope,
  projection: NativePlscConsistentPermutationProjection,
): boolean {
  const settings = recipe.settings;
  const provenance = envelope.provenance;
  const provenanceSettings = provenance.settings;
  const methodConfig = recipe.method_config;
  const groupColumn = methodConfig?.kind === "plsc_permutation"
    ? methodConfig.group_column.trim()
    : undefined;
  const groupA = methodConfig?.kind === "plsc_permutation"
    ? methodConfig.group_a.trim()
    : undefined;
  const groupB = methodConfig?.kind === "plsc_permutation"
    ? methodConfig.group_b.trim()
    : undefined;
  const selectedTestTail = methodConfig?.kind === "plsc_permutation"
    ? methodConfig.test_tail ?? "two_sided"
    : undefined;
  const selectedTailInference = projection.selectedTailInference;
  const indicators = recipe.model.constructs.flatMap((construct) => construct.indicators);
  return recipe.schema_version === 3
    && methodConfig?.kind === "plsc_permutation"
    && provenance.recipe_id === recipe.id
    && provenance.dataset_fingerprint === recipe.dataset_fingerprint
    && provenance.method === "plsc"
    && settings.method === "plsc"
    && settings.weighting_scheme !== "pca"
    && settings.preprocessing === "standardized"
    && settings.missing_data === "listwise_deletion"
    && settings.case_weight_column == null
    && settings.bootstrap_samples === 0
    && settings.studentized_inner_samples === 0
    && settings.permutation_samples === projection.requestedPermutations
    && settings.confidence_level === 0.95
    && Number.isFinite(settings.tolerance)
    && settings.tolerance > 0
    && Number.isSafeInteger(settings.max_iterations)
    && settings.max_iterations > 0
    && Number.isSafeInteger(settings.workers)
    && settings.workers >= 1
    && settings.workers <= 64
    && settings.seed === provenance.seed
    && provenanceSettings.method === settings.method
    && provenanceSettings.weighting_scheme === settings.weighting_scheme
    && provenanceSettings.tolerance === settings.tolerance
    && provenanceSettings.max_iterations === settings.max_iterations
    && provenanceSettings.bootstrap_samples === settings.bootstrap_samples
    && provenanceSettings.studentized_inner_samples === settings.studentized_inner_samples
    && provenanceSettings.permutation_samples === settings.permutation_samples
    && provenanceSettings.seed === settings.seed
    && provenanceSettings.workers === settings.workers
    && provenanceSettings.confidence_level === settings.confidence_level
    && provenanceSettings.preprocessing === settings.preprocessing
    && provenanceSettings.missing_data === settings.missing_data
    && provenanceSettings.case_weight_column === settings.case_weight_column
    && envelope.payload.kind === "pls_pm_v3"
    && envelope.payload.bootstrap == null
    && groupColumn === projection.permutation.group_column
    && groupA === projection.permutation.group_a?.group
    && groupB === projection.permutation.group_b?.group
    && groupA !== groupB
    && (selectedTestTail === "two_sided"
      ? selectedTailInference == null
      : (selectedTestTail === "group_a_greater" || selectedTestTail === "group_a_less")
        && selectedTailInference?.selected_test_tail === selectedTestTail)
    && !indicators.includes(groupColumn ?? "")
    && recipe.model.constructs.length > 0
    && recipe.model.constructs.every((construct) => (
      construct.mode === "reflective"
      && construct.indicators.length >= 2
    ))
    && recipe.model.paths.length > 0
    && (recipe.model.controls ?? []).length === 0
    && (recipe.model.interactions ?? []).length === 0
    && (recipe.model.higher_order_constructs ?? []).length === 0;
}

function finite(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function nonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function hasText(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function numbersClose(left: number, right: number): boolean {
  return Math.abs(left - right) <= 1e-10 * Math.max(1, Math.abs(left), Math.abs(right));
}

function exactStrings(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function parameterKey(kind: string, ...parts: string[]): string {
  return JSON.stringify([kind, parts]);
}

function expectedParameterFamilies(run: AnalysisRun): Map<string, ParameterFamily> | null {
  const result = run.result;
  const plsc = result?.plsc;
  if (!result || !plsc || result.method_version !== "plsc_v2" || plsc.method_version !== "plsc_v2") return null;
  if (plsc.corrected_paths.length !== result.paths.length
    || plsc.corrected_paths.some((path, index) => {
      const root = result.paths[index];
      return !root || path.source !== root.source || path.target !== root.target
        || path.coefficient !== root.coefficient;
    })) return null;

  const expected = new Map<string, ParameterFamily>();
  const add = (identity: string, family: ParameterFamily, value: number): boolean => {
    if (expected.has(identity) || !finite(value)) return false;
    expected.set(identity, family);
    return true;
  };
  for (const row of plsc.reliabilities) {
    if (!hasText(row.construct) || !add(parameterKey("plsc_rho_a", row.construct), "rho_a", row.rho_a)) return null;
  }
  for (const row of plsc.construct_correlations) {
    if (!hasText(row.left) || !hasText(row.right)
      || !add(parameterKey("plsc_construct_correlation", row.left, row.right), "construct_correlation", row.corrected)) return null;
  }
  for (const row of plsc.corrected_outer_loadings) {
    if (!hasText(row.construct) || !hasText(row.indicator)
      || !add(parameterKey("plsc_outer_loading", row.construct, row.indicator), "outer_loading", row.loading)) return null;
  }
  for (const row of plsc.corrected_paths) {
    if (!hasText(row.source) || !hasText(row.target)
      || !add(parameterKey("plsc_path", row.source, row.target), "path", row.coefficient)) return null;
  }
  for (const [construct, value] of Object.entries(plsc.corrected_r_squared)) {
    if (!hasText(construct) || !add(parameterKey("plsc_r_squared", construct), "r_squared", value)) return null;
  }
  return expected.size > 0 ? expected : null;
}

/**
 * Fail-closed projection for the separately attributed, internal two-group
 * PLSc label-permutation result. Archive validation remains authoritative;
 * this prevents a malformed or ordinary-PLS permutation payload from being
 * rendered or semantically exported as consistent permutation.
 */
export function nativePlscConsistentPermutationProjection(
  run: AnalysisRun | null | undefined,
): NativePlscConsistentPermutationProjection | null {
  const permutation = run?.permutation;
  const result = run?.result;
  const provenance = run?.provenance;
  const currentTest = permutation?.test_method === NATIVE_PLSC_CONSISTENT_PERMUTATION_TEST;
  const historicalTest = permutation?.test_method === NATIVE_PLSC_CONSISTENT_PERMUTATION_TEST_V1;
  if (!run || run.status !== "completed" || !permutation || !result || !provenance
    || run.bootstrap != null
    || provenance.method !== "plsc"
    || provenance.settings.method !== "plsc"
    || !result.converged
    || result.method_version !== "plsc_v2"
    || result.plsc?.method_version !== "plsc_v2"
    || result.plsc.reliability_method_version !== "dijkstra_henseler_rho_a_v1"
    || permutation.method_version !== NATIVE_PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION
    || permutation.estimator_method_version !== "plsc_v2"
    || permutation.scheduler_method_version !== NATIVE_PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION
    || permutation.plan.operation !== NATIVE_PLSC_CONSISTENT_PERMUTATION_OPERATION
    || permutation.plan.permutations !== provenance.settings.permutation_samples
    || permutation.plan.master_seed !== provenance.settings.seed
    || provenance.seed !== provenance.settings.seed
    || permutation.plan.permutations < 99
    || permutation.plan.permutations > 10_000
    || provenance.settings.bootstrap_samples !== 0
    || provenance.settings.studentized_inner_samples !== 0
    || provenance.settings.confidence_level !== 0.95
    || provenance.settings.preprocessing !== "standardized"
    || provenance.settings.missing_data !== "listwise_deletion"
    || provenance.settings.case_weight_column != null
    || (!currentTest && !historicalTest)
    || permutation.significance_level !== NATIVE_PLSC_CONSISTENT_PERMUTATION_SIGNIFICANCE_LEVEL
    || permutation.minimum_usable_fraction !== NATIVE_PLSC_CONSISTENT_PERMUTATION_MINIMUM_USABLE_FRACTION
    || permutation.retry_policy !== NATIVE_PLSC_CONSISTENT_PERMUTATION_RETRY_POLICY
    || !hasText(permutation.group_column)
    || !permutation.group_a || !permutation.group_b
    || !hasText(permutation.group_a.group) || !hasText(permutation.group_b.group)
    || permutation.group_a.group === permutation.group_b.group
    || !nonNegativeInteger(permutation.group_a.observations) || permutation.group_a.observations < 10
    || !nonNegativeInteger(permutation.group_b.observations) || permutation.group_b.observations < 10
    || permutation.group_a.observations + permutation.group_b.observations !== result.used_observations
    || !SHA256.test(permutation.group_a.parameter_values_sha256)
    || !SHA256.test(permutation.group_b.parameter_values_sha256)
    || !SHA256.test(permutation.pooled_parameter_values_sha256 ?? "")) return null;

  const methodVersionTokens = provenance.method_version.split("+");
  const methodVersions = new Set(methodVersionTokens);
  if (!["plsc_v2", NATIVE_PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION, NATIVE_PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION]
    .every((version) => methodVersions.has(version))
    || methodVersions.has("freedman_lane_permutation_v1")) return null;

  const requested = permutation.plan.permutations;
  const minimumUsable = Math.max(2, Math.ceil(requested * NATIVE_PLSC_CONSISTENT_PERMUTATION_MINIMUM_USABLE_FRACTION));
  if (!nonNegativeInteger(permutation.usable_permutations)
    || permutation.usable_permutations < minimumUsable
    || !Array.isArray(permutation.failed_permutations)
    || permutation.usable_permutations + permutation.failed_permutations.length !== requested
    || !Array.isArray(permutation.permutation_ledger)
    || permutation.permutation_ledger.length !== requested) return null;

  const failures = new Map<number, NonNullable<PlsPermutationRun["failed_permutations"]>[number]>();
  for (const failure of permutation.failed_permutations) {
    if (!nonNegativeInteger(failure.permutation_index)
      || failure.permutation_index >= requested
      || failures.has(failure.permutation_index)
      || !SHA256.test(failure.label_assignment_sha256)
      || !FAILURE_REASONS.has(failure.reason_code)
      || !hasText(failure.message)) return null;
    failures.set(failure.permutation_index, failure);
  }
  let successfulLedgerEntries = 0;
  for (const [index, entry] of permutation.permutation_ledger.entries()) {
    if (entry.permutation_index !== index || !SHA256.test(entry.label_assignment_sha256)) return null;
    const failure = failures.get(index);
    if (entry.status === "success") {
      if (failure || !SHA256.test(entry.parameter_values_sha256 ?? "")
        || entry.reason_code != null || entry.message != null) return null;
      successfulLedgerEntries += 1;
    } else if (entry.status === "failed") {
      if (!failure || entry.parameter_values_sha256 != null
        || entry.reason_code !== failure.reason_code
        || entry.message !== failure.message
        || entry.label_assignment_sha256 !== failure.label_assignment_sha256) return null;
    } else {
      return null;
    }
  }
  if (successfulLedgerEntries !== permutation.usable_permutations) return null;

  const expected = expectedParameterFamilies(run);
  if (!expected || !Array.isArray(permutation.parameters)
    || permutation.parameters.length !== expected.size) return null;
  const expectedOrder = [...expected.keys()].sort();
  const parameterCounts: Record<ParameterFamily, number> = {
    path: 0,
    outer_loading: 0,
    rho_a: 0,
    construct_correlation: 0,
    r_squared: 0,
  };
  const seen = new Set<string>();
  for (const [index, parameter] of permutation.parameters.entries()) {
    const expectedFamily = expected.get(parameter.parameter);
    const probability = (parameter.exceedances + 1) / (permutation.usable_permutations + 1);
    if (expectedFamily == null
      || parameter.parameter !== expectedOrder[index]
      || seen.has(parameter.parameter)
      || parameter.family !== expectedFamily
      || !finite(parameter.estimate_a)
      || !finite(parameter.estimate_b)
      || !finite(parameter.original)
      || !numbersClose(parameter.original, parameter.estimate_a - parameter.estimate_b)
      || !nonNegativeInteger(parameter.exceedances)
      || parameter.exceedances > permutation.usable_permutations
      || parameter.permutations !== permutation.usable_permutations
      || !finite(parameter.p_value_two_sided)
      || parameter.p_value_two_sided <= 0
      || parameter.p_value_two_sided > 1
      || (currentTest
        ? parameter.p_value_two_sided !== probability
        : !numbersClose(parameter.p_value_two_sided, probability))) return null;
    seen.add(parameter.parameter);
    parameterCounts[expectedFamily] += 1;
  }

  const directional = permutation.directional_inference;
  if (currentTest) {
    if (!directional
      || directional.method_version !== NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_METHOD_VERSION
      || directional.test_method !== NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_TEST
      || !Array.isArray(directional.parameters)
      || directional.parameters.length !== permutation.parameters.length) return null;
    for (const [index, directed] of directional.parameters.entries()) {
      const twoSided = permutation.parameters[index];
      if (!twoSided) return null;
      const expectedGreater = (directed.greater_or_equal + 1) / (permutation.usable_permutations + 1);
      const expectedLess = (directed.less_or_equal + 1) / (permutation.usable_permutations + 1);
      const twoSidedDominatesObservedTail = twoSided.original > 0
        ? twoSided.exceedances >= directed.greater_or_equal
        : twoSided.original < 0
          ? twoSided.exceedances >= directed.less_or_equal
          : twoSided.exceedances === permutation.usable_permutations;
      if (directed.parameter !== twoSided.parameter
        || directed.permutations !== permutation.usable_permutations
        || !nonNegativeInteger(directed.greater_or_equal)
        || directed.greater_or_equal > permutation.usable_permutations
        || !nonNegativeInteger(directed.less_or_equal)
        || directed.less_or_equal > permutation.usable_permutations
        || directed.greater_or_equal + directed.less_or_equal < permutation.usable_permutations
        || !twoSidedDominatesObservedTail
        || !finite(directed.p_value_greater)
        || directed.p_value_greater <= 0
        || directed.p_value_greater > 1
        || directed.p_value_greater !== expectedGreater
        || !finite(directed.p_value_less)
        || directed.p_value_less <= 0
        || directed.p_value_less > 1
        || directed.p_value_less !== expectedLess) return null;
    }
  } else if (directional != null) {
    return null;
  }

  const selectedTailInference = permutation.selected_tail_inference;
  const selectedTailMarkerCount = methodVersionTokens.filter(
    (version) => version === NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION,
  ).length;
  if (selectedTailMarkerCount !== (selectedTailInference == null ? 0 : 1)) return null;
  if (!currentTest) {
    if (selectedTailInference != null) return null;
  } else if (selectedTailInference != null) {
    if (!directional
      || selectedTailInference.method_version !== NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION
      || selectedTailInference.orientation !== NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_ORIENTATION
      || (selectedTailInference.selected_test_tail !== "group_a_greater"
        && selectedTailInference.selected_test_tail !== "group_a_less")
      || !Array.isArray(selectedTailInference.parameters)
      || selectedTailInference.parameters.length !== permutation.parameters.length
      || selectedTailInference.parameters.length !== directional.parameters.length) return null;
    for (const [index, selected] of selectedTailInference.parameters.entries()) {
      const twoSided = permutation.parameters[index];
      const directed = directional.parameters[index];
      if (!twoSided || !directed) return null;
      const selectedExceedances = selectedTailInference.selected_test_tail === "group_a_greater"
        ? directed.greater_or_equal
        : directed.less_or_equal;
      const selectedPValue = selectedTailInference.selected_test_tail === "group_a_greater"
        ? directed.p_value_greater
        : directed.p_value_less;
      if (selected.parameter !== twoSided.parameter
        || selected.parameter !== directed.parameter
        || selected.permutations !== permutation.usable_permutations
        || selected.permutations !== directed.permutations
        || !nonNegativeInteger(selected.selected_exceedances)
        || selected.selected_exceedances !== selectedExceedances
        || !finite(selected.selected_p_value)
        || selected.selected_p_value !== selectedPValue) return null;
    }
  }

  const expectedWarnings = [
    NATIVE_PLSC_CONSISTENT_PERMUTATION_FULL_REFIT_WARNING,
    NATIVE_PLSC_CONSISTENT_PERMUTATION_FAILURE_LEDGER_WARNING,
    currentTest
      ? NATIVE_PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING
      : NATIVE_PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING_V1,
  ];
  if (!Array.isArray(permutation.warnings)
    || !exactStrings(permutation.warnings, expectedWarnings)) return null;

  return Object.freeze({
    permutation,
    selectedTailInference: selectedTailInference ?? null,
    requestedPermutations: requested,
    usablePermutations: permutation.usable_permutations,
    failedPermutations: permutation.failed_permutations.length,
    minimumUsablePermutations: minimumUsable,
    successfulLedgerEntries,
    parameterCounts: Object.freeze(parameterCounts),
  });
}
