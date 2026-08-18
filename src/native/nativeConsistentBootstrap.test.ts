import { describe, expect, it } from "vitest";
import type { AnalysisRun } from "../types";
import {
  NATIVE_PLSC_CONSISTENT_BOOTSTRAP_FAILURE_LEDGER_WARNING,
  NATIVE_PLSC_CONSISTENT_BOOTSTRAP_FULL_REFIT_WARNING,
  NATIVE_PLSC_CONSISTENT_BOOTSTRAP_INCOMPLETE_JACKKNIFE_WARNING,
  nativePlscConsistentBootstrapProjection,
} from "./nativeConsistentBootstrap";
import { nativeRunProvenanceTable } from "./nativeExportTables";
import { nativeResultTables } from "./nativeResults";

const PARAMETER = "[\"plsc_path\",[\"x\",\"y\"]]";
const DIGEST = "ab".repeat(32);
const SAMPLE_DIGEST = "cd".repeat(32);

function completedConsistentBootstrapRun(): AnalysisRun {
  return {
    id: "plsc-bootstrap-run",
    name: "PLSc consistent bootstrapping",
    method: "PLSc Consistent Bootstrapping",
    createdAt: "2026-08-14T08:00:00.000Z",
    seed: 42,
    status: "completed",
    warnings: [],
    fingerprint: "sha256:plsc-bootstrap-fixture",
    modelSnapshot: {
      nodes: [
        { id: "x", type: "construct", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
        { id: "y", type: "construct", position: { x: 240, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
      ],
      edges: [{ id: "x-y", source: "x", target: "y" }],
    },
    result: {
      method_version: "plsc_v2",
      converged: true,
      iterations: 8,
      used_observations: 5,
      omitted_observations: 0,
      outer_estimates: [
        { construct: "x", indicator: "x1", weight: 0.5, loading: 0.8 },
        { construct: "x", indicator: "x2", weight: 0.5, loading: 0.8 },
        { construct: "y", indicator: "y1", weight: 0.5, loading: 0.8 },
        { construct: "y", indicator: "y2", weight: 0.5, loading: 0.8 },
      ],
      paths: [{ source: "x", target: "y", coefficient: 0.4 }],
      effects: [{ source: "x", target: "y", direct: 0.4, indirect: 0, total: 0.4 }],
      plsc: {
        method_version: "plsc_v2",
        reliability_method_version: "rho_a_v1",
        tolerance: 1e-7,
        reliabilities: [{ construct: "x", rho_a: 0.8 }, { construct: "y", rho_a: 0.8 }],
        construct_correlations: [{ left: "x", right: "y", original: 0.3, corrected: 0.4 }],
        corrected_paths: [{ source: "x", target: "y", coefficient: 0.4 }],
        corrected_outer_loadings: [
          { construct: "x", indicator: "x1", weight: 0.5, loading: 0.8 },
          { construct: "x", indicator: "x2", weight: 0.5, loading: 0.8 },
          { construct: "y", indicator: "y1", weight: 0.5, loading: 0.8 },
          { construct: "y", indicator: "y2", weight: 0.5, loading: 0.8 },
        ],
        corrected_r_squared: { y: 0.16 },
        warnings: [],
      },
      r_squared: { y: 0.16 },
      warnings: [],
    },
    bootstrap: {
      method_version: "plsc_bootstrap_v1",
      estimator_method_version: "plsc_v2",
      resampling_method_version: "indexed_resampling_v4",
      plan: { replicates: 1_000, master_seed: 42, operation: "plsc_consistent_bootstrap_v1" },
      minimum_usable_fraction: 0.9,
      retry_policy: "no_retry_no_replacement_fixed_indexed_draws_v1",
      original_parameter_values_sha256: DIGEST,
      usable_replicates: 1_000,
      failed_replicates: [],
      replicate_ledger: Array.from({ length: 1_000 }, (_, replicate_index) => ({
        replicate_index,
        sample_indices_sha256: SAMPLE_DIGEST,
        status: "success" as const,
        parameter_values_sha256: DIGEST,
        reason_code: null,
        message: null,
      })),
      successful_replicates: Array.from({ length: 1_000 }, (_, replicate_index) => ({
        replicate_index,
        iterations: 8,
        used_observations: 5,
        omitted_observations: 0,
        parameters: { [PARAMETER]: 0.41 },
      })),
      percentile: {
        confidence_level: 0.95,
        parameters: [{
          parameter: PARAMETER,
          original: 0.4,
          bootstrap_mean: 0.41,
          bias: 0.01,
          standard_error: 0.05,
          lower: 0.30,
          upper: 0.50,
          usable_replicates: 1_000,
          t_statistic: 8,
          p_value_two_sided: 0,
        }],
      },
      bca: {
        confidence_level: 0.95,
        jackknife_case_count: 5,
        parameters: [{
          parameter: PARAMETER,
          bias_correction: 0.01,
          acceleration: 0.02,
          lower: 0.29,
          upper: 0.51,
          unavailable_reason: null,
        }],
      },
      successful_jackknife_cases: Array.from({ length: 5 }, (_, omitted_case) => ({
        omitted_case,
        iterations: 8,
        used_observations: 4,
        omitted_observations: 0,
        parameters: { [PARAMETER]: 0.4 },
      })),
      failed_jackknife_cases: [],
      warnings: [
        NATIVE_PLSC_CONSISTENT_BOOTSTRAP_FULL_REFIT_WARNING,
        NATIVE_PLSC_CONSISTENT_BOOTSTRAP_FAILURE_LEDGER_WARNING,
      ],
    },
    provenance: {
      recipe_id: "plsc-bootstrap-recipe",
      dataset_fingerprint: "sha256:plsc-bootstrap-fixture",
      method: "plsc",
      method_version: "pls_pm_v1+plsc_v2+plsc_bootstrap_v1+indexed_resampling_v4",
      engine_version: "test",
      seed: 42,
      settings: {
        method: "plsc",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        bootstrap_samples: 1_000,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 42,
        workers: 4,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-14T07:59:00.000Z",
      completed_at: "2026-08-14T08:00:00.000Z",
    },
  };
}

describe("native PLSc consistent-bootstrap projection", () => {
  it("accepts the exact separately attributed Standard contract and renders validated results and provenance", () => {
    const run = completedConsistentBootstrapRun();
    expect(nativePlscConsistentBootstrapProjection(run)).toMatchObject({
      requestedReplicates: 1_000,
      usableReplicates: 1_000,
      failedReplicates: 0,
      minimumUsableReplicates: 900,
      successfulReplicateWitnesses: 1_000,
      jackknifeCases: 5,
      successfulJackknifeWitnesses: 5,
      failedJackknifeCases: 0,
      bcaAvailableParameters: 1,
      bcaUnavailableParameters: 0,
    });
    const tables = nativeResultTables(run);
    expect(tables.find((table) => table.id === "plsc_bootstrap_accounting")?.status).toBe("validated");
    const percentile = tables.find((table) => table.id === "bootstrap_percentile");
    expect(percentile?.status).toBe("validated");
    expect(percentile?.rows[0][0]).toBe("Plsc path: Predictor → Outcome");
    expect(tables.find((table) => table.id === "bootstrap_bca")?.status).toBe("validated");
    const provenance = nativeRunProvenanceTable(run);
    expect(provenance.status).toBe("validated");
    expect(provenance.rows).toContainEqual(["Requested PLSc bootstrap refits", "1000"]);
    expect(provenance.rows).toContainEqual(["Replayable successful PLSc bootstrap witnesses", "1000"]);
    expect(provenance.rows).toContainEqual(["Replayable successful PLSc delete-one witnesses", "5"]);
    expect(provenance.rows).toContainEqual(["PLSc bootstrap failure policy", "No retry or replacement draw"]);
  });

  it("renders typed delete-one failures and explicit BCa unavailable rows", () => {
    const run = completedConsistentBootstrapRun();
    run.bootstrap!.successful_jackknife_cases!.pop();
    run.bootstrap!.failed_jackknife_cases = [{
      omitted_case: 4,
      reason_code: "plsc_nonconvergence",
      message: "PLSc did not converge for the delete-one sample",
    }];
    run.bootstrap!.bca!.parameters[0] = {
      parameter: PARAMETER,
      bias_correction: null,
      acceleration: null,
      lower: null,
      upper: null,
      unavailable_reason: "one or more required full-PLSc delete-one refits failed",
    };
    run.bootstrap!.warnings!.push(NATIVE_PLSC_CONSISTENT_BOOTSTRAP_INCOMPLETE_JACKKNIFE_WARNING);

    expect(nativePlscConsistentBootstrapProjection(run)).toMatchObject({
      successfulJackknifeWitnesses: 4,
      failedJackknifeCases: 1,
      bcaAvailableParameters: 0,
      bcaUnavailableParameters: 1,
    });
    const tables = nativeResultTables(run);
    expect(tables.find((table) => table.id === "plsc_bootstrap_jackknife_failures")).toMatchObject({
      status: "validated",
      rows: [["5", "plsc_nonconvergence", "PLSc did not converge for the delete-one sample"]],
    });
    expect(tables.find((table) => table.id === "plsc_bootstrap_bca_unavailable")).toMatchObject({
      status: "validated",
      rows: [["Plsc path: Predictor → Outcome", "one or more required full-PLSc delete-one refits failed"]],
    });
  });

  it("fails closed on malformed attribution, ledger integrity, digests, warnings, and unsupported add-ons", () => {
    const cases: AnalysisRun[] = [];

    const wrongVersion = structuredClone(completedConsistentBootstrapRun());
    wrongVersion.bootstrap!.method_version = "indexed_resampling_v4";
    cases.push(wrongVersion);

    const wrongIndex = structuredClone(completedConsistentBootstrapRun());
    wrongIndex.bootstrap!.replicate_ledger![1].replicate_index = 2;
    cases.push(wrongIndex);

    const badDigest = structuredClone(completedConsistentBootstrapRun());
    badDigest.bootstrap!.replicate_ledger![0].parameter_values_sha256 = "not-a-digest";
    cases.push(badDigest);

    const missingWitness = structuredClone(completedConsistentBootstrapRun());
    missingWitness.bootstrap!.successful_replicates!.pop();
    cases.push(missingWitness);

    const wrongWitnessIdentity = structuredClone(completedConsistentBootstrapRun());
    wrongWitnessIdentity.bootstrap!.successful_replicates![0].parameters = { "[\"plsc_path\",[\"x\",\"z\"]]": 0.41 };
    cases.push(wrongWitnessIdentity);

    const missingDeleteOneWitness = structuredClone(completedConsistentBootstrapRun());
    missingDeleteOneWitness.bootstrap!.successful_jackknife_cases!.pop();
    cases.push(missingDeleteOneWitness);

    const missingWarning = structuredClone(completedConsistentBootstrapRun());
    missingWarning.bootstrap!.warnings = [];
    cases.push(missingWarning);

    const ordinaryAddOn = structuredClone(completedConsistentBootstrapRun());
    ordinaryAddOn.bootstrap!.studentized = {
      method_version: "studentized_v1",
      confidence_level: 0.95,
      inner_replicates: 99,
      minimum_usable_fraction: 0.9,
      stream_domain: "invalid",
      failure: null,
      parameters: [],
    };
    cases.push(ordinaryAddOn);

    for (const run of cases) {
      expect(nativePlscConsistentBootstrapProjection(run)).toBeNull();
      expect(nativeResultTables(run)).toEqual([]);
    }
  });
});
