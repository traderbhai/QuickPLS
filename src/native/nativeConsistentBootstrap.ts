import type { AnalysisRun, PlsBootstrapRun } from "../types";

export const NATIVE_PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION = "plsc_bootstrap_v1";
export const NATIVE_PLSC_ESTIMATOR_METHOD_VERSION = "plsc_v2";
export const NATIVE_PLSC_RESAMPLING_METHOD_VERSION = "indexed_resampling_v4";
export const NATIVE_PLSC_CONSISTENT_BOOTSTRAP_OPERATION = "plsc_consistent_bootstrap_v1";
export const NATIVE_PLSC_CONSISTENT_BOOTSTRAP_RETRY_POLICY = "no_retry_no_replacement_fixed_indexed_draws_v1";
export const NATIVE_PLSC_CONSISTENT_BOOTSTRAP_MINIMUM_USABLE_FRACTION = 0.9;
export const NATIVE_PLSC_CONSISTENT_BOOTSTRAP_FULL_REFIT_WARNING =
  "Consistent bootstrapping fully re-estimated plsc_v2 for every accepted case resample; ordinary PLS bootstrap estimates were not reused.";
export const NATIVE_PLSC_CONSISTENT_BOOTSTRAP_FAILURE_LEDGER_WARNING =
  "Failed or inadmissible PLSc refits were retained in the fixed replicate ledger without retry, replacement, or clamping.";
export const NATIVE_PLSC_CONSISTENT_BOOTSTRAP_INCOMPLETE_JACKKNIFE_WARNING =
  "BCa intervals are unavailable because at least one required full-PLSc delete-one refit failed; percentile inference remains available.";
export const NATIVE_PLSC_CONSISTENT_BOOTSTRAP_NUMERICAL_BCA_WARNING =
  "One or more BCa intervals are unavailable because the full-PLSc acceleration or adjusted quantiles were numerically undefined; percentile inference remains available.";

const SHA256 = /^[0-9a-f]{64}$/;
const FAILURE_REASONS = new Set([
  "cancelled",
  "inadmissible_rho_a",
  "inadmissible_corrected_correlation",
  "plsc_nonconvergence",
  "singular_plsc_equation",
  "nonfinite_plsc_parameter",
  "parameter_identity_mismatch",
  "plsc_refit_failed",
]);

export interface NativePlscConsistentBootstrapProjection {
  readonly bootstrap: PlsBootstrapRun;
  readonly requestedReplicates: number;
  readonly usableReplicates: number;
  readonly failedReplicates: number;
  readonly minimumUsableReplicates: number;
  readonly successfulReplicateWitnesses: number;
  readonly jackknifeCases: number;
  readonly successfulJackknifeWitnesses: number;
  readonly failedJackknifeCases: number;
  readonly bcaAvailableParameters: number;
  readonly bcaUnavailableParameters: number;
}

function finite(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function nonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
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

function validParameterIdentity(value: string): boolean {
  try {
    const parsed = JSON.parse(value) as unknown;
    return Array.isArray(parsed)
      && parsed.length === 2
      && typeof parsed[0] === "string"
      && parsed[0].startsWith("plsc_")
      && Array.isArray(parsed[1])
      && parsed[1].length > 0
      && parsed[1].every(hasText);
  } catch {
    return false;
  }
}

/**
 * Fail-closed customer projection for the separately attributed PLSc bootstrap.
 * Rust archive validation remains authoritative; this gate prevents malformed
 * or mislabeled live payloads from being rendered or exported by the frontend.
 */
export function nativePlscConsistentBootstrapProjection(
  run: AnalysisRun | null | undefined,
): NativePlscConsistentBootstrapProjection | null {
  const bootstrap = run?.bootstrap;
  const result = run?.result;
  const provenance = run?.provenance;
  if (!run || run.status !== "completed" || !bootstrap || !result || !provenance
    || provenance.method !== "plsc"
    || provenance.settings.method !== "plsc"
    || result.method_version !== NATIVE_PLSC_ESTIMATOR_METHOD_VERSION
    || result.plsc?.method_version !== NATIVE_PLSC_ESTIMATOR_METHOD_VERSION
    || bootstrap.method_version !== NATIVE_PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION
    || bootstrap.estimator_method_version !== NATIVE_PLSC_ESTIMATOR_METHOD_VERSION
    || bootstrap.resampling_method_version !== NATIVE_PLSC_RESAMPLING_METHOD_VERSION
    || bootstrap.plan.operation !== NATIVE_PLSC_CONSISTENT_BOOTSTRAP_OPERATION
    || bootstrap.plan.replicates !== provenance.settings.bootstrap_samples
    || bootstrap.plan.master_seed !== provenance.seed
    || provenance.seed !== provenance.settings.seed
    || bootstrap.plan.replicates < 1_000
    || bootstrap.plan.replicates > 10_000
    || provenance.settings.studentized_inner_samples !== 0
    || provenance.settings.permutation_samples !== 0
    || bootstrap.studentized != null
    || bootstrap.htmt_inference != null
    || run.permutation != null
    || bootstrap.minimum_usable_fraction !== NATIVE_PLSC_CONSISTENT_BOOTSTRAP_MINIMUM_USABLE_FRACTION
    || bootstrap.retry_policy !== NATIVE_PLSC_CONSISTENT_BOOTSTRAP_RETRY_POLICY
    || !SHA256.test(bootstrap.original_parameter_values_sha256 ?? "")) return null;

  const methodVersions = new Set(provenance.method_version.split("+"));
  if (![NATIVE_PLSC_ESTIMATOR_METHOD_VERSION, NATIVE_PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION, NATIVE_PLSC_RESAMPLING_METHOD_VERSION]
    .every((version) => methodVersions.has(version))) return null;

  const requested = bootstrap.plan.replicates;
  const minimumUsable = Math.max(2, Math.ceil(requested * NATIVE_PLSC_CONSISTENT_BOOTSTRAP_MINIMUM_USABLE_FRACTION));
  if (!nonNegativeInteger(bootstrap.usable_replicates)
    || bootstrap.usable_replicates < minimumUsable
    || !Array.isArray(bootstrap.failed_replicates)
    || bootstrap.usable_replicates + bootstrap.failed_replicates.length !== requested
    || !Array.isArray(bootstrap.replicate_ledger)
    || bootstrap.replicate_ledger.length !== requested) return null;

  if (!Array.isArray(bootstrap.successful_replicates)
    || bootstrap.successful_replicates.length !== bootstrap.usable_replicates) return null;
  const successfulWitnesses = new Map<number, NonNullable<typeof bootstrap.successful_replicates>[number]>();
  for (const witness of bootstrap.successful_replicates) {
    if (!nonNegativeInteger(witness.replicate_index)
      || witness.replicate_index >= requested
      || successfulWitnesses.has(witness.replicate_index)
      || !nonNegativeInteger(witness.iterations) || witness.iterations === 0
      || witness.used_observations !== result.used_observations
      || witness.omitted_observations !== 0
      || typeof witness.parameters !== "object" || witness.parameters == null
      || Array.isArray(witness.parameters)) return null;
    successfulWitnesses.set(witness.replicate_index, witness);
  }

  const failures = new Map<number, (typeof bootstrap.failed_replicates)[number]>();
  for (const failure of bootstrap.failed_replicates) {
    if (!nonNegativeInteger(failure.replicate_index)
      || failure.replicate_index >= requested
      || failures.has(failure.replicate_index)
      || !SHA256.test(failure.sample_indices_sha256 ?? "")
      || !FAILURE_REASONS.has(failure.reason_code ?? "")
      || !hasText(failure.message)) return null;
    failures.set(failure.replicate_index, failure);
  }

  for (const [index, entry] of bootstrap.replicate_ledger.entries()) {
    if (entry.replicate_index !== index || !SHA256.test(entry.sample_indices_sha256)) return null;
    const failure = failures.get(index);
    if (entry.status === "success") {
      if (failure || !successfulWitnesses.has(index)
        || !SHA256.test(entry.parameter_values_sha256 ?? "")
        || entry.reason_code != null || entry.message != null) return null;
    } else if (entry.status === "failed") {
      if (!failure || successfulWitnesses.has(index) || entry.parameter_values_sha256 != null
        || entry.reason_code !== failure.reason_code
        || entry.message !== failure.message
        || entry.sample_indices_sha256 !== failure.sample_indices_sha256) return null;
    } else {
      return null;
    }
  }

  const percentile = bootstrap.percentile;
  if (!percentile || percentile.confidence_level !== provenance.settings.confidence_level
    || !Array.isArray(percentile.parameters) || percentile.parameters.length === 0) return null;
  const parameterIds = new Set<string>();
  for (const parameter of percentile.parameters) {
    if (!validParameterIdentity(parameter.parameter) || parameterIds.has(parameter.parameter)
      || !finite(parameter.original) || !finite(parameter.bootstrap_mean)
      || !finite(parameter.bias) || !numbersClose(parameter.bias, parameter.bootstrap_mean - parameter.original)
      || !finite(parameter.standard_error) || parameter.standard_error < 0
      || !finite(parameter.lower) || !finite(parameter.upper) || parameter.lower > parameter.upper
      || parameter.usable_replicates !== bootstrap.usable_replicates
      || (parameter.t_statistic != null && !finite(parameter.t_statistic))
      || (parameter.p_value_two_sided != null
        && (!finite(parameter.p_value_two_sided) || parameter.p_value_two_sided < 0 || parameter.p_value_two_sided > 1))
      || ((parameter.t_statistic == null) !== (parameter.p_value_two_sided == null))) return null;
    parameterIds.add(parameter.parameter);
  }
  const sortedParameterIds = [...parameterIds].sort();
  for (const witness of successfulWitnesses.values()) {
    const witnessParameterIds = Object.keys(witness.parameters).sort();
    if (!exactStrings(witnessParameterIds, sortedParameterIds)
      || Object.values(witness.parameters).some((value) => !finite(value))) return null;
  }

  const bca = bootstrap.bca;
  const failedJackknife = bootstrap.failed_jackknife_cases;
  if (!bca || bca.confidence_level !== provenance.settings.confidence_level
    || bca.jackknife_case_count !== result.used_observations
    || !Array.isArray(bca.parameters) || bca.parameters.length !== parameterIds.size
    || !Array.isArray(failedJackknife)) return null;
  const bcaIds = new Set<string>();
  let bcaAvailable = 0;
  let bcaUnavailable = 0;
  for (const parameter of bca.parameters) {
    if (!parameterIds.has(parameter.parameter) || bcaIds.has(parameter.parameter)) return null;
    bcaIds.add(parameter.parameter);
    const available = finite(parameter.bias_correction)
      && finite(parameter.acceleration)
      && finite(parameter.lower)
      && finite(parameter.upper)
      && parameter.lower <= parameter.upper
      && parameter.unavailable_reason == null;
    const unavailable = parameter.bias_correction == null
      && parameter.acceleration == null
      && parameter.lower == null
      && parameter.upper == null
      && hasText(parameter.unavailable_reason);
    if (available) bcaAvailable += 1;
    else if (unavailable) bcaUnavailable += 1;
    else return null;
  }

  let previousOmitted = -1;
  for (const failure of failedJackknife) {
    if (!nonNegativeInteger(failure.omitted_case)
      || failure.omitted_case >= bca.jackknife_case_count
      || failure.omitted_case <= previousOmitted
      || !FAILURE_REASONS.has(failure.reason_code)
      || !hasText(failure.message)) return null;
    previousOmitted = failure.omitted_case;
  }
  if (!Array.isArray(bootstrap.successful_jackknife_cases)) return null;
  const successfulJackknifeIndices = new Set<number>();
  let previousSuccessfulOmitted = -1;
  for (const witness of bootstrap.successful_jackknife_cases) {
    const witnessParameterIds = typeof witness.parameters === "object" && witness.parameters != null
      && !Array.isArray(witness.parameters)
      ? Object.keys(witness.parameters).sort()
      : [];
    if (!nonNegativeInteger(witness.omitted_case)
      || witness.omitted_case >= bca.jackknife_case_count
      || witness.omitted_case <= previousSuccessfulOmitted
      || successfulJackknifeIndices.has(witness.omitted_case)
      || failedJackknife.some((failure) => failure.omitted_case === witness.omitted_case)
      || !nonNegativeInteger(witness.iterations) || witness.iterations === 0
      || witness.used_observations + 1 !== result.used_observations
      || witness.omitted_observations !== 0
      || !exactStrings(witnessParameterIds, sortedParameterIds)
      || Object.values(witness.parameters).some((value) => !finite(value))) return null;
    successfulJackknifeIndices.add(witness.omitted_case);
    previousSuccessfulOmitted = witness.omitted_case;
  }
  if (bca.jackknife_case_count < 3) {
    if (bootstrap.successful_jackknife_cases.length !== 0 || failedJackknife.length !== 0) return null;
  } else if (bootstrap.successful_jackknife_cases.length + failedJackknife.length !== bca.jackknife_case_count) {
    return null;
  }
  if (failedJackknife.length > 0 && bcaAvailable > 0) return null;

  const expectedWarnings = [
    NATIVE_PLSC_CONSISTENT_BOOTSTRAP_FULL_REFIT_WARNING,
    NATIVE_PLSC_CONSISTENT_BOOTSTRAP_FAILURE_LEDGER_WARNING,
    ...(failedJackknife.length > 0
      ? [NATIVE_PLSC_CONSISTENT_BOOTSTRAP_INCOMPLETE_JACKKNIFE_WARNING]
      : bcaUnavailable > 0
        ? [NATIVE_PLSC_CONSISTENT_BOOTSTRAP_NUMERICAL_BCA_WARNING]
        : []),
  ];
  if (!Array.isArray(bootstrap.warnings) || !exactStrings(bootstrap.warnings, expectedWarnings)) return null;

  return Object.freeze({
    bootstrap,
    requestedReplicates: requested,
    usableReplicates: bootstrap.usable_replicates,
    failedReplicates: bootstrap.failed_replicates.length,
    minimumUsableReplicates: minimumUsable,
    successfulReplicateWitnesses: bootstrap.successful_replicates.length,
    jackknifeCases: bca.jackknife_case_count,
    successfulJackknifeWitnesses: bootstrap.successful_jackknife_cases.length,
    failedJackknifeCases: failedJackknife.length,
    bcaAvailableParameters: bcaAvailable,
    bcaUnavailableParameters: bcaUnavailable,
  });
}
