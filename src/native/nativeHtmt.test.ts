import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import { tablesToCsv } from "../domain/resultTables";
import type {
  AnalysisRun,
  HtmtAssessment,
  HtmtBootstrapInference,
} from "../types";
import { nativeRunProvenanceTable } from "./nativeExportTables";
import { nativeResultTables } from "./nativeResults";


function point(absoluteCorrelations: boolean): HtmtAssessment {
  return {
    constructs: ["competence", "likeability"],
    correlation_type: "pearson",
    absolute_correlations: absoluteCorrelations,
    cells: [
      [
        { value: 1, status: "available", reason: null },
        { value: 0.75, status: "available", reason: null },
      ],
      [
        { value: 0.75, status: "available", reason: null },
        { value: 1, status: "available", reason: null },
      ],
    ],
  };
}


function inference(
  absoluteCorrelations: boolean,
  methodVersion: string,
  pointMethodVersion: string,
): HtmtBootstrapInference {
  const diagonal = {
    status: "not_applicable" as const,
    reason: "htmt.bootstrap.diagonal_not_inferred",
    original: 1,
    bootstrap_mean: null,
    bias: null,
    standard_error: null,
    bias_correction: null,
    lower: null,
    upper: null,
    usable_replicates: 0,
    failed_replicates: 0,
    below_original: 0,
    tied_original: 0,
    replicate_min: null,
    replicate_max: null,
    upper_bound_below_critical_value: null,
    usable_replicate_indices_sha256: null,
    pair_unavailable_replicates: [],
  };
  const pair = {
    status: "available" as const,
    reason: null,
    original: 0.75,
    bootstrap_mean: 0.755,
    bias: 0.005,
    standard_error: 0.05,
    bias_correction: -0.1,
    lower: 0.64,
    upper: 0.84,
    usable_replicates: 10,
    failed_replicates: 0,
    below_original: 4,
    tied_original: 0,
    replicate_min: 0.61,
    replicate_max: 0.91,
    upper_bound_below_critical_value: true,
    usable_replicate_indices_sha256: "01".repeat(32),
    pair_unavailable_replicates: [],
  };
  return {
    method_version: methodVersion,
    point_method_version: pointMethodVersion,
    constructs: ["competence", "likeability"],
    correlation_type: "pearson",
    absolute_correlations: absoluteCorrelations,
    interval_method: "bias_corrected_percentile_type7_v1",
    test_type: "one_tailed_upper",
    significance_level: 0.05,
    equivalent_two_sided_confidence_level: 0.9,
    critical_value: 0.9,
    decision_rule: "bias_corrected_upper_bound_strictly_below_critical_value_v1",
    replicate_index_digest_method: "sha256_u32_le_v1",
    requested_replicates: 10,
    minimum_usable_replicates: 9,
    retry_policy: "no_retry_fixed_preplanned_primary_draws_v1",
    cells: [[diagonal, pair], [pair, diagonal]],
  };
}


function completedHtmtRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  return {
    ...base,
    assessment: {
      ...base.assessment!,
      htmt_plus_method_version: "ringle_et_al_htmt_plus_v1",
      htmt_plus: point(true),
      htmt_original_method_version: "henseler_et_al_htmt_v1",
      htmt_original: point(false),
    },
    bootstrap: {
      ...base.bootstrap!,
      plan: {
        operation: "pls_pm_bootstrap_v1",
        replicates: 10,
        master_seed: 42,
      },
      usable_replicates: 10,
      failed_replicates: [],
      htmt_inference: {
        method_version: "htmt_bias_corrected_bootstrap_inference_v1",
        htmt_plus: inference(
          true,
          "ringle_et_al_htmt_plus_bias_corrected_bootstrap_v1",
          "ringle_et_al_htmt_plus_v1",
        ),
        htmt_original: inference(
          false,
          "henseler_et_al_htmt_bias_corrected_bootstrap_v1",
          "henseler_et_al_htmt_v1",
        ),
      },
    },
    provenance: {
      recipe_id: "htmt-qualification-recipe",
      dataset_fingerprint: "sha256:htmt-qualification-dataset",
      method: "bootstrap",
      method_version:
        "pls_pm_v1+indexed_resampling_v4+htmt_bias_corrected_bootstrap_inference_v1",
      engine_version: "qualification-fixture",
      seed: 42,
      settings: {
        method: "bootstrap",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 300,
        bootstrap_samples: 10,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 42,
        workers: 2,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-14T00:00:00Z",
      completed_at: "2026-08-14T00:00:01Z",
    },
  };
}


describe("HTMT native qualification contract", () => {
  it("retains inference when an ineligible diagonal has no point HTMT value", () => {
    const run = completedHtmtRun();
    const inference = run.bootstrap!.htmt_inference!;
    for (const kind of ["htmt_plus", "htmt_original"] as const) {
      const pointArtifact = run.assessment![kind]!;
      const inferenceArtifact = inference[kind];
      for (let index = 0; index < pointArtifact.constructs.length; index += 1) {
        pointArtifact.cells[index][index] = {
          value: null,
          status: "not_applicable",
          reason: "htmt.single_indicator_not_applicable",
        };
        inferenceArtifact.cells[index][index].original = null;
      }
    }

    expect(nativeResultTables(run).map((table) => table.id)).toEqual(expect.arrayContaining([
      "htmt_plus_bootstrap",
      "htmt_original_bootstrap",
    ]));

    inference.htmt_plus.cells[0][0].original = 1;
    expect(nativeResultTables(run)).toEqual([]);
  });

  it("renders point and BC-not-BCa inference with explicit decision and ledger", () => {
    const run = completedHtmtRun();
    const tables = nativeResultTables(run);
    const plus = tables.find((table) => table.id === "htmt_plus_bootstrap");
    expect(tables.map((table) => table.id)).toEqual(expect.arrayContaining([
      "htmt_plus",
      "htmt_original",
      "htmt_plus_bootstrap",
      "htmt_original_bootstrap",
    ]));
    expect(plus?.status).toBe("experimental");
    expect(plus?.columns).toEqual(expect.arrayContaining([
      "BC 90% lower",
      "BC 90% upper",
      "Decision at 0.90",
      "Usable-index digest",
    ]));
    expect(plus?.rows[0]).toEqual(expect.arrayContaining([
      "Established: upper bound < 0.90",
      "01".repeat(32),
    ]));

    const provenance = nativeRunProvenanceTable(run);
    expect(provenance.rows).toEqual(expect.arrayContaining([
      ["HTMT interval", "Bias-corrected percentile (Type 7); not BCa"],
      ["HTMT test", "One-tailed upper, alpha .05"],
      ["HTMT critical value", "0.9"],
      [
        "HTMT decision rule",
        "Documented inference: bias-corrected upper bound strictly below 0.90",
      ],
    ]));
    const csv = tablesToCsv([...tables, provenance]);
    expect(csv).toContain("HTMT+ bias-corrected bootstrap inference");
    expect(csv).toContain("Bias-corrected percentile (Type 7); not BCa");
    expect(csv).not.toContain("BCa interval");
  });

  it("fails closed on decision, digest, and unavailable-ledger tampering", () => {
    const contradictory = completedHtmtRun();
    contradictory.bootstrap!.htmt_inference!.htmt_plus.cells[0][1]
      .upper_bound_below_critical_value = false;
    contradictory.bootstrap!.htmt_inference!.htmt_plus.cells[1][0]
      .upper_bound_below_critical_value = false;
    expect(nativeResultTables(contradictory)).toEqual([]);

    const missingDigest = completedHtmtRun();
    missingDigest.bootstrap!.htmt_inference!.htmt_plus.cells[0][1]
      .usable_replicate_indices_sha256 = null;
    missingDigest.bootstrap!.htmt_inference!.htmt_plus.cells[1][0]
      .usable_replicate_indices_sha256 = null;
    expect(nativeResultTables(missingDigest)).toEqual([]);

    const duplicateIndex = completedHtmtRun();
    const pair = duplicateIndex.bootstrap!.htmt_inference!.htmt_plus.cells[0][1];
    pair.usable_replicates = 8;
    pair.failed_replicates = 2;
    pair.pair_unavailable_replicates = [
      { replicate_index: 2, reason_code: "htmt.zero_monotrait_denominator" },
      { replicate_index: 2, reason_code: "htmt.zero_monotrait_denominator" },
    ];
    duplicateIndex.bootstrap!.htmt_inference!.htmt_plus.cells[1][0]
      = structuredClone(pair);
    expect(nativeResultTables(duplicateIndex)).toEqual([]);
  });
});
