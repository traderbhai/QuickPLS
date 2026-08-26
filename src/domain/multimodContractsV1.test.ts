import { describe, expect, it } from "vitest";
import {
  MULTIMOD_SIDECAR_MAX_BYTES_V1,
  MULTIMOD_SIDECAR_WARN_BYTES_V1,
  MultiModContractErrorV1,
  multiModSidecarCostStateV1,
  parseGeneralSemConditionalProcessConfigV2,
  parseInterventionalCausalMediationConfigV1,
  parseMultiModAnalysisResultV1,
  parseMgaMultigroupV1,
  parseMultimodIntervalV1,
  parseMultimodResultSidecarDescriptorV1,
  parsePlsUnobservedHeterogeneityConfigV2,
  predictMultiModSidecarBytesV1,
} from "./multimodContractsV1";

const checklist = {
  identical_indicators_and_coding: true,
  identical_data_treatment: true,
  identical_algorithm_settings: true,
  identical_model_specification: true,
  deterministic_sign_orientation_reviewed: true,
  analyst_review_confirmed: true,
};

const identification = {
  temporal_order_declared: true,
  adjustment_set_justified: true,
  consistency_assumption_acknowledged: true,
  no_unmeasured_treatment_outcome_confounding_acknowledged: true,
  no_unmeasured_treatment_mediator_confounding_acknowledged: true,
  no_unmeasured_mediator_outcome_confounding_acknowledged: true,
  no_exposure_induced_mediator_outcome_confounder_confirmed: true,
  no_recanting_witness_confirmed: true,
  linear_model_specification_reviewed: true,
  positivity_reviewed: true,
};

function boundaryMgaResult(compositionalCorrelation: number) {
  return {
    kind: "pls_multigroup_analysis_v1",
    analysis: {
      schema_version: 1,
      provenance: {
        method_version: "qpls.mga.multigroup.v1",
        recipe_id: "recipe-1",
        recipe_analytical_sha256: "a".repeat(64),
        config_sha256: "b".repeat(64),
        model_id: "model-1",
        model_scientific_sha256: "c".repeat(64),
        dataset_id: "dataset-1",
        dataset_fingerprint: "dataset-fingerprint",
        engine_version: "2.56.0",
        seed: 42,
        capability_cell: {
          registry_schema_version: 2,
          capability_id: "quickpls.multimod",
          cell_id: "qpls.multimod.mga.general_sem_pls.v1",
          capability_version: "mga_multigroup_v1",
        },
        qualification: "unqualified_labs",
      },
      profile: "general_sem_pls",
      group_eligibility: [
        {
          group_id: "group_01",
          label: "Group 1",
          complete_cases: 30,
          selected_rows: 30,
          eligible: true,
          warnings: [],
          blockers: [],
        },
        {
          group_id: "group_07",
          label: "Group 7",
          complete_cases: 30,
          selected_rows: 30,
          eligible: true,
          warnings: [],
          blockers: [],
        },
      ],
      group_parameters: [],
      micom_pairs: [
        {
          left_group_id: "group_01",
          right_group_id: "group_07",
          construct_id: "construct:x",
          interpretation: "composite_invariance",
          configural_invariance_confirmed: true,
          compositional_correlation: compositionalCorrelation,
          compositional_lower_quantile: 0.999_999_999_999_999_7,
          compositional_p_value: 1,
          compositional_invariance: true,
          partial_invariance: true,
          equal_mean_p_value: 1,
          equal_variance_p_value: 1,
        },
      ],
      omnibus: [],
      pairwise: [],
      multiplicity: "holm",
      replicate_ledgers: [],
      excluded_rows: [],
      sidecars: [],
    },
  };
}

describe("MultiMod versioned TypeScript contracts", () => {
  it("uses the frozen MGA numerical-tie rule for MICOM result validation", () => {
    const boundary = parseMultiModAnalysisResultV1(
      boundaryMgaResult(0.999_999_999_999_999_6),
      "result-1",
    );
    expect(boundary.kind).toBe("pls_multigroup_analysis_v1");

    expect(() =>
      parseMultiModAnalysisResultV1(
        boundaryMgaResult(0.999_999_999_999_999_7 - 1e-10),
        "result-1",
      ),
    ).toThrowError(
      expect.objectContaining({ code: "multimod_result.micom_pair" }),
    );
  });

  it("preserves typed MGA group identities and complete MICOM review", () => {
    const parsed = parseMgaMultigroupV1({
      schema_version: 1,
      profile: "general_sem_pls",
      grouping_column: "sector",
      groups: [
        {
          group_id: "integer-one",
          label: "Integer 1",
          value: { kind: "integer", value: 1 },
        },
        {
          group_id: "numeric-one",
          label: "Numeric 1",
          value: { kind: "number", value: 1 },
        },
      ],
      comparison_plan: { kind: "all_pairs", heavy_run_confirmed: false },
      procedures: ["micom_pairwise", "pairwise_permutation"],
      permutation_samples: 5_000,
      bootstrap_samples: 5_000,
      seed: 42,
      confidence_level: 0.95,
      alpha: 0.05,
      alternative: "two_sided",
      multiplicity: "holm",
      configural_checklist: checklist,
    });
    expect(parsed.groups.map((group) => group.value.kind)).toEqual([
      "integer",
      "number",
    ]);
    expect(parsed.multiplicity).toBe("holm");
    expect(parsed.selected_parameter_ids).toEqual([]);
  });

  it("fails closed instead of losing an i64 group identity outside JavaScript's safe range", () => {
    expect(() =>
      parseMgaMultigroupV1({
        schema_version: 1,
        profile: "general_sem_pls",
        grouping_column: "cohort",
        groups: [
          {
            group_id: "unsafe",
            label: "Unsafe",
            value: { kind: "integer", value: Number.MAX_SAFE_INTEGER + 1 },
          },
          {
            group_id: "safe",
            label: "Safe",
            value: { kind: "integer", value: 1 },
          },
        ],
        comparison_plan: { kind: "all_pairs", heavy_run_confirmed: false },
        procedures: ["pairwise_permutation"],
        permutation_samples: 5_000,
        bootstrap_samples: 5_000,
        seed: 42,
        confidence_level: 0.95,
        alpha: 0.05,
        alternative: "two_sided",
        multiplicity: "holm",
        configural_checklist: checklist,
      }),
    ).toThrowError(expect.objectContaining({ code: "multimod.value.integer" }));
  });

  it("requires max-spread omnibus inference before three-group pairwise follow-up", () => {
    expect(() =>
      parseMgaMultigroupV1({
        schema_version: 1,
        profile: "general_sem_pls",
        grouping_column: "cohort",
        groups: ["a", "b", "c"].map((value) => ({
          group_id: value,
          label: value.toUpperCase(),
          value: { kind: "text", value },
        })),
        comparison_plan: { kind: "all_pairs", heavy_run_confirmed: false },
        procedures: ["pairwise_permutation"],
        permutation_samples: 5_000,
        bootstrap_samples: 5_000,
        seed: 42,
        confidence_level: 0.95,
        alpha: 0.05,
        alternative: "two_sided",
        multiplicity: "holm",
        configural_checklist: checklist,
      }),
    ).toThrowError(
      expect.objectContaining({ code: "mga_multigroup_v1.omnibus_required" }),
    );
  });

  it("fails closed when publication-faithful PLS-POS is combined with interactions", () => {
    expect(() =>
      parsePlsUnobservedHeterogeneityConfigV2({
        schema_version: 2,
        profile: "p2_multi_two_way",
        phase: {
          kind: "discovery",
          candidate_k: [2, 3],
          algorithms: ["pls_pos_published_v2"],
        },
        seed: 42,
        fimix: {
          starts: 30,
          max_iterations: 5_000,
          relative_log_likelihood_tolerance: 1e-10,
          consecutive_converged_iterations: 3,
          likelihood_decrease_tolerance: 1e-9,
          residual_variance_floor: 1e-8,
          rank_tolerance: 1e-11,
          minimum_class_share: 0.05,
          required_reproducing_starts: 2,
          optimum_relative_log_likelihood_tolerance: 1e-8,
          optimum_maximum_coefficient_difference: 1e-6,
          optimum_mean_posterior_difference: 1e-4,
        },
        pls_pos: {
          starts: 10,
          strict_improvement_tolerance: 1e-10,
          stable_objective_tolerance: 1e-10,
          minimum_reproducing_starts: 2,
        },
      }),
    ).toThrowError(
      expect.objectContaining({
        code: "pls_heterogeneity_v2.published_pos_interaction",
      }),
    );
  });

  it("rejects a FIMIX likelihood-decrease tolerance outside the frozen contract", () => {
    expect(() =>
      parsePlsUnobservedHeterogeneityConfigV2({
        schema_version: 2,
        profile: "p0_structural",
        phase: {
          kind: "discovery",
          candidate_k: [2],
          algorithms: ["fimix_pls_v2"],
        },
        seed: 42,
        fimix: {
          starts: 30,
          max_iterations: 5_000,
          relative_log_likelihood_tolerance: 1e-10,
          consecutive_converged_iterations: 3,
          likelihood_decrease_tolerance: 2e-9,
          residual_variance_floor: 1e-8,
          rank_tolerance: 1e-11,
          minimum_class_share: 0.05,
          required_reproducing_starts: 2,
          optimum_relative_log_likelihood_tolerance: 1e-8,
          optimum_maximum_coefficient_difference: 1e-6,
          optimum_mean_posterior_difference: 1e-4,
        },
        pls_pos: {
          starts: 10,
          strict_improvement_tolerance: 1e-10,
          stable_objective_tolerance: 1e-10,
          minimum_reproducing_starts: 2,
        },
      }),
    ).toThrowError(
      expect.objectContaining({ code: "pls_heterogeneity_v2.fimix_settings" }),
    );
  });

  it("requires inference to carry a confirmed discovery-bound lock receipt", () => {
    const base = {
      schema_version: 2,
      profile: "p0_structural",
      seed: 42,
      fimix: {
        starts: 30,
        max_iterations: 5_000,
        relative_log_likelihood_tolerance: 1e-10,
        consecutive_converged_iterations: 3,
        likelihood_decrease_tolerance: 1e-9,
        residual_variance_floor: 1e-8,
        rank_tolerance: 1e-11,
        minimum_class_share: 0.05,
        required_reproducing_starts: 2,
        optimum_relative_log_likelihood_tolerance: 1e-8,
        optimum_maximum_coefficient_difference: 1e-6,
        optimum_mean_posterior_difference: 1e-4,
      },
      pls_pos: {
        starts: 10,
        strict_improvement_tolerance: 1e-10,
        stable_objective_tolerance: 1e-10,
        minimum_reproducing_starts: 2,
      },
    } as const;
    const parsed = parsePlsUnobservedHeterogeneityConfigV2({
      ...base,
      phase: {
        kind: "inference",
        lock: {
          schema_version: 1,
          discovery_result_identity_sha256: "a".repeat(64),
          discovery_candidate_k: [2, 3],
          discovery_algorithms: ["fimix_pls_v2", "pls_pos_published_v2"],
          selected_algorithm: "pls_pos_published_v2",
          selected_k: 3,
          analyst_lock_confirmed: true,
          tandem_fimix_same_k_start_required: true,
        },
      },
      bootstrap: { resamples: 1_000, seed: 42, confidence_level: 0.95 },
    });
    expect(parsed.phase).toMatchObject({
      kind: "inference",
      lock: {
        selected_algorithm: "pls_pos_published_v2",
        selected_k: 3,
      },
    });
    expect(() =>
      parsePlsUnobservedHeterogeneityConfigV2({
        ...base,
        phase: {
          kind: "inference",
        },
      }),
    ).toThrow(/lock/u);
  });

  it("accepts an explicit both-stage conditional path on one shared studentized ledger", () => {
    const parsed = parseGeneralSemConditionalProcessConfigV2({
      schema_version: 2,
      profile: "multi_two_way_studentized",
      paths: [{ path_id: "x-m-y", ordered_relation_ids: ["x-m", "m-y"] }],
      declared_interaction_ids: ["x-w-m", "m-v-y"],
      hoc_ids: [],
      moderator_ids: ["w", "v"],
      probes: [
        {
          probe_id: "probe-w",
          moderator_id: "w",
          scale: "standardized_score",
          values: [-1, 0, 1],
        },
        {
          probe_id: "probe-v",
          moderator_id: "v",
          scale: "standardized_score",
          values: [-1, 0, 1],
        },
      ],
      explicit_joint_tuples: [],
      probe_contrasts: [],
      groups: [],
      group_contrasts: [],
      estimands: {
        conditional_specific_indirect: true,
        conditional_total_indirect: false,
        conditional_total_effect: false,
        scalar_index_when_affine: false,
        local_first_derivatives: true,
        local_second_and_cross_derivatives: true,
        finite_probe_contrasts: false,
      },
      inference: {
        interval: "studentized",
        alternative: "greater",
        outer_resamples: 1_000,
        inner_resamples: 200,
        seed: 42,
        confidence_level: 0.95,
      },
    });
    expect(parsed.paths[0].ordered_relation_ids).toEqual(["x-m", "m-y"]);
    expect(parsed.inference).toMatchObject({
      interval: "studentized",
      outer_resamples: 1_000,
      inner_resamples: 200,
    });
  });

  it("requires every identification declaration for the separate causal module", () => {
    const parsed = parseInterventionalCausalMediationConfigV1({
      schema_version: 1,
      treatment: "x",
      treatment_contrast: { kind: "binary", control: 0, treated: 1 },
      outcome: "y",
      mediators: ["m"],
      adjustment_covariates: ["c"],
      paths: [
        {
          path_id: "x-m-y",
          ordered_variable_ids: ["x", "m", "y"],
          equations: [
            {
              equation_id: "equation:m",
              outcome_variable_id: "m",
              terms: [
                { term_id: "term:x", factor_variable_ids: ["x"] },
                { term_id: "term:c", factor_variable_ids: ["c"] },
              ],
            },
            {
              equation_id: "equation:y",
              outcome_variable_id: "y",
              terms: [
                { term_id: "term:m", factor_variable_ids: ["m"] },
                { term_id: "term:x", factor_variable_ids: ["x"] },
                { term_id: "term:c", factor_variable_ids: ["c"] },
              ],
            },
          ],
        },
      ],
      positivity_policy: {
        minimum_binary_arm_count: 10,
        maximum_binary_arm_ratio: 10,
        positivity_strata_variable_ids: [],
        minimum_count_per_binary_stratum_arm: 1,
        continuous_neighborhood_fraction_of_range: 0.05,
        minimum_continuous_neighborhood_count: 5,
      },
      identification,
      bootstrap_resamples: 1_000,
      seed: 42,
      confidence_level: 0.95,
    });
    expect(parsed.treatment_contrast).toEqual({
      kind: "binary",
      control: 0,
      treated: 1,
    });
    expect(parsed.baseline_moderators).toEqual([]);
    expect(parsed.adjustment_covariates).toEqual(["c"]);

    expect(() =>
      parseInterventionalCausalMediationConfigV1({
        ...parsed,
        identification: { ...identification, positivity_reviewed: false },
      }),
    ).toThrowError(
      expect.objectContaining({
        code: "interventional_causal_mediation_v1.identification",
      } satisfies Partial<MultiModContractErrorV1>),
    );
    expect(() =>
      parseInterventionalCausalMediationConfigV1({
        ...parsed,
        adjustment_covariates: [],
      }),
    ).toThrowError(
      expect.objectContaining({
        code: "interventional_causal_mediation_v1.adjustment_set_missing",
      } satisfies Partial<MultiModContractErrorV1>),
    );
  });

  it("uses deterministic warning and rejection thresholds for predicted sidecars", () => {
    expect(
      multiModSidecarCostStateV1(
        predictMultiModSidecarBytesV1({
          kind: "causal",
          rows: 100,
          resamples: 500,
          targets: 2,
        }),
      ),
    ).toBe("normal");
    expect(multiModSidecarCostStateV1(MULTIMOD_SIDECAR_WARN_BYTES_V1)).toBe(
      "normal",
    );
    expect(
      multiModSidecarCostStateV1(MULTIMOD_SIDECAR_WARN_BYTES_V1 + 1),
    ).toBe("warning");
    expect(multiModSidecarCostStateV1(MULTIMOD_SIDECAR_MAX_BYTES_V1)).toBe(
      "warning",
    );
    expect(multiModSidecarCostStateV1(MULTIMOD_SIDECAR_MAX_BYTES_V1 + 1)).toBe(
      "blocked",
    );
    expect(
      multiModSidecarCostStateV1(
        predictMultiModSidecarBytesV1({
          kind: "causal",
          rows: Number.MAX_SAFE_INTEGER,
          resamples: Number.MAX_SAFE_INTEGER,
          targets: Number.MAX_SAFE_INTEGER,
        }),
      ),
    ).toBe("blocked");
  });

  it("charges weighted MGA row coordinates in every persisted draw", () => {
    const shared = {
      kind: "mga" as const,
      groupRows: [50, 50],
      procedures: ["pairwise_permutation" as const],
      pairCount: 1,
      permutationSamples: 5_000,
      bootstrapSamples: 5_000,
      targets: 4,
      micomConstructs: 3,
    };
    expect(
      predictMultiModSidecarBytesV1({
        ...shared,
        profile: "case_weighted_pls",
      }),
    ).toBeGreaterThan(
      predictMultiModSidecarBytesV1({
        ...shared,
        profile: "general_sem_pls",
      }),
    );
  });

  it("charges each retained MICOM construct null series exactly once", () => {
    const shared = {
      kind: "mga" as const,
      groupRows: Array.from({ length: 20 }, () => 10),
      profile: "general_sem_pls" as const,
      procedures: [
        "micom_pairwise" as const,
        "pairwise_permutation" as const,
        "henseler_pls_mga" as const,
        "bootstrap_difference_bc" as const,
        "omnibus_max_spread_permutation" as const,
      ],
      pairCount: 190,
      permutationSamples: 5_000,
      bootstrapSamples: 5_000,
      targets: 30,
      maximumTargetIdBytes: 64,
    };
    const oneConstruct = predictMultiModSidecarBytesV1({
      ...shared,
      micomConstructs: 1,
    });
    const twoConstructs = predictMultiModSidecarBytesV1({
      ...shared,
      micomConstructs: 2,
    });
    const threeConstructs = predictMultiModSidecarBytesV1({
      ...shared,
      micomConstructs: 3,
    });
    expect(twoConstructs - oneConstruct).toBe(190 * 5_000 * 18);
    expect(multiModSidecarCostStateV1(twoConstructs)).toBe("warning");
    expect(multiModSidecarCostStateV1(threeConstructs)).toBe("blocked");
  });

  it("charges retained omnibus null vectors independently of other MGA procedures", () => {
    const shared = {
      kind: "mga" as const,
      groupRows: Array.from({ length: 20 }, () => 10),
      profile: "general_sem_pls" as const,
      pairCount: 190,
      permutationSamples: 5_000,
      bootstrapSamples: 5_000,
      targets: 30,
      maximumTargetIdBytes: 64,
      micomConstructs: 2,
    };
    const withOmnibus = predictMultiModSidecarBytesV1({
      ...shared,
      procedures: ["omnibus_max_spread_permutation"],
    });
    const withoutOmnibus = predictMultiModSidecarBytesV1({
      ...shared,
      procedures: [],
    });
    expect(withOmnibus - withoutOmnibus).toBe(
      5_000 * (200 * 8 + 30 * 21 + 128) + 4_096 + 30 * (64 + 4),
    );
  });

  it("requires alternative-specific interval endpoints", () => {
    expect(
      parseMultimodIntervalV1({
        confidence_level: 0.95,
        upper: 0.4,
        family: "percentile_type7",
        alternative: "less",
      }),
    ).toEqual({
      confidence_level: 0.95,
      upper: 0.4,
      family: "percentile_type7",
      alternative: "less",
    });
    expect(() =>
      parseMultimodIntervalV1({
        confidence_level: 0.95,
        lower: -0.1,
        upper: 0.4,
        family: "percentile_type7",
        alternative: "less",
      }),
    ).toThrowError(
      expect.objectContaining({ code: "multimod_result.interval" }),
    );
  });

  it("requires a digest-bound Arrow schema identity on every result sidecar", () => {
    const descriptor = {
      schema_version: 1,
      entry_name: "results/result-1/fimix-candidate-posteriors.arrow",
      evidence_role: "fimix-candidate:posteriors",
      arrow_schema_contract_id: `qpls.multimod.arrow.fimix-candidate:posteriors.v1.${"d".repeat(64)}`,
      arrow_schema_contract_version: 1,
      media_type: "application/vnd.apache.arrow.stream",
      compression: "zip_deflate",
      arrow_schema_sha256: "c".repeat(64),
      row_count: 10,
      column_count: 2,
      uncompressed_bytes: 1_024,
      sha256: "a".repeat(64),
      identity_sha256: "b".repeat(64),
      required_for_scientific_reopen: true,
    };
    expect(
      parseMultimodResultSidecarDescriptorV1(descriptor, "result-1", "sidecar")
        .arrow_schema_sha256,
    ).toBe("c".repeat(64));
    const { arrow_schema_sha256: _omitted, ...withoutSchemaIdentity } =
      descriptor;
    expect(() =>
      parseMultimodResultSidecarDescriptorV1(
        withoutSchemaIdentity,
        "result-1",
        "sidecar",
      ),
    ).toThrowError(
      expect.objectContaining({ code: "multimod.shape.missing_field" }),
    );
  });
});
