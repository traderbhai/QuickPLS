import type {
  RegressionBootstrapAnalysis,
  RegressionBootstrapValidationWitness,
} from "../types";

export const NATIVE_REGRESSION_BOOTSTRAP_MAX_COEFFICIENT_TERMS = 51;
export const NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS =
  NATIVE_REGRESSION_BOOTSTRAP_MAX_COEFFICIENT_TERMS - 1;

function nonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function finiteCoefficientVector(value: unknown, width: number, logistic: boolean): value is number[] {
  return Array.isArray(value)
    && value.length === width
    && value.every((coefficient) => typeof coefficient === "number"
      && Number.isFinite(coefficient)
      && (!logistic || Number.isFinite(Math.exp(coefficient))));
}

function exactStrings(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function strictlyAscending(indices: readonly number[]): boolean {
  return indices.every((value, index) => index === 0 || value > indices[index - 1]);
}

function exactIndexPartition(
  total: number,
  success: readonly number[],
  failure: readonly number[],
): boolean {
  if (success.length + failure.length !== total
    || !strictlyAscending(success)
    || !strictlyAscending(failure)) return false;
  const seen = new Set([...success, ...failure]);
  return seen.size === total
    && [...seen].every((index) => nonNegativeInteger(index) && index < total);
}

/**
 * Validates the internal archive witness structure used by qpls-project to
 * recompute regression-bootstrap summaries. Aggregate arithmetic is rechecked
 * by qpls-project before an archived result reaches this hydration boundary.
 * The witness is deliberately not projected into user-facing tables or exports.
 */
export function isNativeRegressionBootstrapValidationWitness(
  value: unknown,
  expectedTerms: readonly string[],
  bootstrap: Pick<
    RegressionBootstrapAnalysis,
    "requested_replicates" | "failed_replicates" | "jackknife_cases" | "usable_jackknife_cases"
  >,
  logistic: boolean,
): value is RegressionBootstrapValidationWitness {
  if (!value || typeof value !== "object") return false;
  const witness = value as Partial<RegressionBootstrapValidationWitness>;
  if (!nonNegativeInteger(bootstrap.requested_replicates)
    || !Array.isArray(bootstrap.failed_replicates)
    || !nonNegativeInteger(bootstrap.jackknife_cases)
    || !nonNegativeInteger(bootstrap.usable_jackknife_cases)
    || bootstrap.usable_jackknife_cases > bootstrap.jackknife_cases
    || witness.method_version !== "regression_bootstrap_validation_witness_v1"
    || !Array.isArray(witness.terms)
    || expectedTerms.length < 2
    || expectedTerms.length > NATIVE_REGRESSION_BOOTSTRAP_MAX_COEFFICIENT_TERMS
    || expectedTerms[0] !== "intercept"
    || expectedTerms.some((term) => typeof term !== "string" || !term.trim())
    || new Set(expectedTerms).size !== expectedTerms.length
    || !exactStrings(witness.terms, expectedTerms)
    || !Array.isArray(witness.successful_bootstrap)
    || !Array.isArray(witness.successful_jackknife)
    || !Array.isArray(witness.failed_jackknife)) return false;

  const failedBootstrapIndices: number[] = [];
  for (const failure of bootstrap.failed_replicates) {
    if (!failure || !nonNegativeInteger(failure.replicate_index)
      || typeof failure.reason_code !== "string" || !failure.reason_code.trim()
      || typeof failure.message !== "string" || !failure.message.trim()) return false;
    failedBootstrapIndices.push(failure.replicate_index);
  }
  const successfulBootstrapIndices: number[] = [];
  for (const row of witness.successful_bootstrap) {
    if (!row || !nonNegativeInteger(row.replicate_index)
      || !finiteCoefficientVector(row.coefficients, expectedTerms.length, logistic)) return false;
    successfulBootstrapIndices.push(row.replicate_index);
  }
  if (witness.successful_bootstrap.length !== bootstrap.requested_replicates - bootstrap.failed_replicates.length
    || !exactIndexPartition(bootstrap.requested_replicates, successfulBootstrapIndices, failedBootstrapIndices)) return false;

  const successfulJackknifeIndices: number[] = [];
  for (const row of witness.successful_jackknife) {
    if (!row || !nonNegativeInteger(row.omitted_case)
      || !finiteCoefficientVector(row.coefficients, expectedTerms.length, logistic)) return false;
    successfulJackknifeIndices.push(row.omitted_case);
  }
  const failedJackknifeIndices: number[] = [];
  for (const row of witness.failed_jackknife) {
    if (!row || !nonNegativeInteger(row.omitted_case)
      || typeof row.reason_code !== "string" || !row.reason_code.trim()
      || typeof row.message !== "string" || !row.message.trim()) return false;
    failedJackknifeIndices.push(row.omitted_case);
  }
  return witness.successful_jackknife.length === bootstrap.usable_jackknife_cases
    && witness.failed_jackknife.length === bootstrap.jackknife_cases - bootstrap.usable_jackknife_cases
    && exactIndexPartition(bootstrap.jackknife_cases, successfulJackknifeIndices, failedJackknifeIndices);
}
