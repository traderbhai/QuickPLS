import { describe, expect, it } from "vitest";
import type {
  RegressionBootstrapAnalysis,
  RegressionBootstrapValidationWitness,
} from "../types";
import {
  isNativeRegressionBootstrapValidationWitness,
  NATIVE_REGRESSION_BOOTSTRAP_MAX_COEFFICIENT_TERMS,
  NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS,
} from "./nativeRegressionBootstrapWitness";

const expectedTerms = ["intercept", "x"] as const;

function summary(): Pick<
  RegressionBootstrapAnalysis,
  "requested_replicates" | "failed_replicates" | "jackknife_cases" | "usable_jackknife_cases"
> {
  return {
    requested_replicates: 99,
    failed_replicates: [{
      replicate_index: 3,
      reason_code: "replicate_fit_failed",
      message: "The resampled fit was singular.",
    }],
    jackknife_cases: 3,
    usable_jackknife_cases: 2,
  };
}

function witness(): RegressionBootstrapValidationWitness {
  return {
    method_version: "regression_bootstrap_validation_witness_v1",
    terms: [...expectedTerms],
    successful_bootstrap: Array.from({ length: 99 }, (_, replicateIndex) => replicateIndex)
      .filter((replicateIndex) => replicateIndex !== 3)
      .map((replicate_index) => ({ replicate_index, coefficients: [1, 0.5] })),
    successful_jackknife: [
      { omitted_case: 0, coefficients: [1, 0.5] },
      { omitted_case: 2, coefficients: [1, 0.5] },
    ],
    failed_jackknife: [{
      omitted_case: 1,
      reason_code: "delete_one_fit_failed",
      message: "The delete-one fit was singular.",
    }],
  };
}

function cloneWitness(value = witness()): RegressionBootstrapValidationWitness {
  return JSON.parse(JSON.stringify(value)) as RegressionBootstrapValidationWitness;
}

describe("native regression-bootstrap structural validation witness boundary", () => {
  it("accepts exact ordered bootstrap and jackknife index complements", () => {
    expect(NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS).toBe(50);
    expect(NATIVE_REGRESSION_BOOTSTRAP_MAX_COEFFICIENT_TERMS).toBe(51);
    expect(isNativeRegressionBootstrapValidationWitness(
      witness(),
      expectedTerms,
      summary(),
      false,
    )).toBe(true);
  });

  it("rejects term, index, vector, failure, and logistic-overflow tampering", () => {
    const wrongTerms = cloneWitness();
    wrongTerms.terms.reverse();
    expect(isNativeRegressionBootstrapValidationWitness(wrongTerms, expectedTerms, summary(), false)).toBe(false);

    const duplicateBootstrapIndex = cloneWitness();
    duplicateBootstrapIndex.successful_bootstrap[1].replicate_index = 0;
    expect(isNativeRegressionBootstrapValidationWitness(duplicateBootstrapIndex, expectedTerms, summary(), false)).toBe(false);

    const incompletePartition = cloneWitness();
    incompletePartition.successful_bootstrap.pop();
    expect(isNativeRegressionBootstrapValidationWitness(incompletePartition, expectedTerms, summary(), false)).toBe(false);

    const wrongVectorWidth = cloneWitness();
    wrongVectorWidth.successful_jackknife[0].coefficients.pop();
    expect(isNativeRegressionBootstrapValidationWitness(wrongVectorWidth, expectedTerms, summary(), false)).toBe(false);

    const emptyFailureReason = summary();
    emptyFailureReason.failed_replicates[0].reason_code = "";
    expect(isNativeRegressionBootstrapValidationWitness(witness(), expectedTerms, emptyFailureReason, false)).toBe(false);

    const logisticOverflow = cloneWitness();
    logisticOverflow.successful_bootstrap[0].coefficients[0] = 1_000;
    expect(isNativeRegressionBootstrapValidationWitness(logisticOverflow, expectedTerms, summary(), true)).toBe(false);
  });

  it("rejects more than 51 coefficient terms", () => {
    const tooManyTerms = [
      "intercept",
      ...Array.from({ length: 51 }, (_, index) => `x${index + 1}`),
    ];
    const tooWide = cloneWitness();
    tooWide.terms = tooManyTerms;
    tooWide.successful_bootstrap = tooWide.successful_bootstrap.map((row) => ({
      ...row,
      coefficients: tooManyTerms.map(() => 0),
    }));
    tooWide.successful_jackknife = tooWide.successful_jackknife.map((row) => ({
      ...row,
      coefficients: tooManyTerms.map(() => 0),
    }));
    expect(isNativeRegressionBootstrapValidationWitness(
      tooWide,
      tooManyTerms,
      summary(),
      false,
    )).toBe(false);
  });
});
