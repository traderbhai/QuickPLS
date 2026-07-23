import type { AnalysisRun } from "../types";

export function completedSamplePlsRun(): AnalysisRun {
  return {
    id: "v11-smoke-completed-pls",
    name: "PLS path modeling core run",
    method: "PLS path modeling core",
    createdAt: "2026-07-19T12:00:00.000Z",
    seed: 20260718,
    status: "completed",
    warnings: ["Validated for the documented QuickPLS supported scope; unsupported shapes remain blocked or explicitly marked."],
    fingerprint: "v11-smoke",
    result: {
      method_version: "pls_pm_v1+v11_smoke_fixture",
      converged: true,
      iterations: 5,
      used_observations: 5,
      omitted_observations: 0,
      outer_estimates: [
        { construct: "competence", indicator: "COMP1", weight: 0.351, loading: 0.842 },
        { construct: "competence", indicator: "COMP2", weight: 0.337, loading: 0.811 },
        { construct: "competence", indicator: "COMP3", weight: 0.329, loading: 0.786 },
        { construct: "likeability", indicator: "LIKE1", weight: 0.511, loading: 0.874 },
        { construct: "likeability", indicator: "LIKE2", weight: 0.497, loading: 0.861 },
        { construct: "satisfaction", indicator: "CUSA1", weight: 0.502, loading: 0.902 },
        { construct: "satisfaction", indicator: "CUSA2", weight: 0.491, loading: 0.888 },
        { construct: "loyalty", indicator: "CUSL1", weight: 0.514, loading: 0.913 },
        { construct: "loyalty", indicator: "CUSL2", weight: 0.486, loading: 0.894 },
      ],
      paths: [
        { source: "competence", target: "satisfaction", coefficient: 0.403 },
        { source: "likeability", target: "satisfaction", coefficient: 0.327 },
        { source: "competence", target: "loyalty", coefficient: 0.116 },
        { source: "likeability", target: "loyalty", coefficient: 0.172 },
        { source: "satisfaction", target: "loyalty", coefficient: 0.544 },
      ],
      effects: [
        { source: "competence", target: "satisfaction", direct: 0.403, indirect: 0, total: 0.403 },
        { source: "likeability", target: "satisfaction", direct: 0.327, indirect: 0, total: 0.327 },
        { source: "competence", target: "loyalty", direct: 0.116, indirect: 0.219, total: 0.335 },
        { source: "likeability", target: "loyalty", direct: 0.172, indirect: 0.178, total: 0.350 },
        { source: "satisfaction", target: "loyalty", direct: 0.544, indirect: 0, total: 0.544 },
      ],
      mediation: {
        method_version: "mediation_v1+smoke",
        tolerance: 1e-7,
        estimates: [
          { source: "competence", target: "loyalty", direct: 0.116, indirect: 0.219, total: 0.335, variance_accounted_for: 0.6537, classification: "complementary_partial", warning: null },
          { source: "likeability", target: "loyalty", direct: 0.172, indirect: 0.178, total: 0.350, variance_accounted_for: 0.5086, classification: "complementary_partial", warning: null },
        ],
        warnings: [],
      },
      r_squared: {
        satisfaction: 0.544,
        loyalty: 0.617,
      },
      warnings: [],
    },
    assessment: {
      method_version: "assessment_v1+smoke",
      rho_a_method_version: "rho_a_v1+smoke",
      construct_quality: [
        { construct: "competence", cronbach_alpha: 0.8966, rho_a: 0.8967, rho_c: 0.9366, ave: 0.8318 },
        { construct: "likeability", cronbach_alpha: 0.8457, rho_a: 0.9719, rho_c: 0.9251, ave: 0.8608, rho_a_warning_codes: ["rho_a.two_indicator_limited_information"] },
        { construct: "satisfaction", cronbach_alpha: 0.6419, rho_a: 0.6897, rho_c: 0.8446, ave: 0.7318, rho_a_warning_codes: ["rho_a.two_indicator_limited_information"] },
        { construct: "loyalty", cronbach_alpha: 0.8350, rho_a: 0.8378, rho_c: 0.9237, ave: 0.8582, rho_a_warning_codes: ["rho_a.two_indicator_limited_information"] },
      ],
      cross_loadings: [
        { indicator: "COMP1", assigned_construct: "competence", construct: "competence", loading: 0.842 },
        { indicator: "LIKE1", assigned_construct: "likeability", construct: "likeability", loading: 0.874 },
        { indicator: "CUSA1", assigned_construct: "satisfaction", construct: "satisfaction", loading: 0.902 },
        { indicator: "CUSL1", assigned_construct: "loyalty", construct: "loyalty", loading: 0.913 },
      ],
      fornell_larcker: {
        constructs: ["competence", "likeability", "satisfaction", "loyalty"],
        values: [
          [0.9120, null, null, null],
          [0.4975, 0.9278, null, null],
          [0.6139, 0.5436, 0.8555, null],
          [0.1333, 0.9580, 1.1202, 0.9264],
        ],
      },
      htmt_plus_method_version: "htmt_plus_v1+smoke",
      htmt_plus: {
        constructs: ["competence", "likeability", "satisfaction", "loyalty"],
        correlation_type: "pearson",
        absolute_correlations: true,
        cells: [
          [{ value: null, status: "not_applicable", reason: "diagonal" }, { value: 0.4975, status: "available", reason: null }, { value: 0.6139, status: "available", reason: null }, { value: 0.1333, status: "available", reason: null }],
          [{ value: 0.4975, status: "available", reason: null }, { value: null, status: "not_applicable", reason: "diagonal" }, { value: 0.5436, status: "available", reason: null }, { value: 0.9580, status: "available", reason: null }],
          [{ value: 0.6139, status: "available", reason: null }, { value: 0.5436, status: "available", reason: null }, { value: null, status: "not_applicable", reason: "diagonal" }, { value: 1.1202, status: "available", reason: null }],
          [{ value: 0.1333, status: "available", reason: null }, { value: 0.9580, status: "available", reason: null }, { value: 1.1202, status: "available", reason: null }, { value: null, status: "not_applicable", reason: "diagonal" }],
        ],
      },
      r_squared: { satisfaction: 0.544, loyalty: 0.617 },
      structural_quality: [
        { construct: "satisfaction", predictor_count: 2, r_squared: 0.544, adjusted_r_squared: 0.5020 },
        { construct: "loyalty", predictor_count: 3, r_squared: 0.617, adjusted_r_squared: 0.5700 },
      ],
      structural_vif: [
        { target_construct: "satisfaction", predictor_construct: "competence", vif: 1.2020 },
        { target_construct: "satisfaction", predictor_construct: "likeability", vif: 1.2020 },
        { target_construct: "loyalty", predictor_construct: "satisfaction", vif: 4.1550 },
      ],
      formative_indicator_vif: [],
      f_squared: [
        { source_construct: "competence", target_construct: "satisfaction", included_r_squared: 0.544, excluded_r_squared: 0.2114, f_squared: 0.7294 },
        { source_construct: "likeability", target_construct: "satisfaction", included_r_squared: 0.544, excluded_r_squared: 0.2368, f_squared: 0.6737 },
        { source_construct: "satisfaction", target_construct: "loyalty", included_r_squared: 0.617, excluded_r_squared: 0.3900, f_squared: 0.5927 },
      ],
      model_fit: {
        saturated: { srmr: 0.0710, d_uls: 0.2140 },
        estimated: { srmr: 0.0810, d_uls: 0.2440 },
      },
      blindfolding: {
        settings: { omission_distance: 7, selection: "endogenous", missing_value_treatment: "listwise" },
        constructs: [
          { construct: "satisfaction", q_squared: 0.4137, prediction_error_sum_squares: 21.753069, observation_sum_squares: 37.105004 },
          { construct: "loyalty", q_squared: 0.6691, prediction_error_sum_squares: 10.908147, observation_sum_squares: 32.966621 },
        ],
      },
      warnings: [],
    },
    bootstrap: {
      method_version: "bootstrap_v1+smoke",
      plan: { replicates: 999, master_seed: 20260718, operation: "pls_pm" },
      usable_replicates: 999,
      failed_replicates: [],
      percentile: {
        confidence_level: 0.95,
        parameters: [
          { parameter: "[\"path\",[\"competence\",\"satisfaction\"]]", original: 0.403, bootstrap_mean: 0.401, bias: -0.002, standard_error: 0.083, lower: 0.214, upper: 0.552, usable_replicates: 999, t_statistic: 4.8554, p_value_two_sided: 0.0008 },
          { parameter: "[\"path\",[\"satisfaction\",\"loyalty\"]]", original: 0.544, bootstrap_mean: 0.541, bias: -0.003, standard_error: 0.091, lower: 0.333, upper: 0.694, usable_replicates: 999, t_statistic: 5.9780, p_value_two_sided: 0.0002 },
          { parameter: "[\"indirect_effect\",[\"competence\",\"loyalty\"]]", original: 0.219, bootstrap_mean: 0.218, bias: -0.001, standard_error: 0.057, lower: 0.101, upper: 0.328, usable_replicates: 999, t_statistic: 3.8421, p_value_two_sided: 0.0040 },
        ],
      },
      bca: {
        confidence_level: 0.95,
        jackknife_case_count: 12,
        parameters: [
          { parameter: "[\"indirect_effect\",[\"competence\",\"loyalty\"]]", bias_correction: 0.02, acceleration: 0.01, lower: 0.096, upper: 0.334, unavailable_reason: null },
        ],
      },
      studentized: {
        method_version: "studentized_v1+smoke",
        confidence_level: 0.95,
        inner_replicates: 99,
        minimum_usable_fraction: 0.8,
        stream_domain: "smoke",
        failure: null,
        parameters: [
          { parameter: "[\"indirect_effect\",[\"competence\",\"loyalty\"]]", original: 0.219, outer_standard_error: 0.057, outer_scale: 1, usable_primary_replicates: 999, lower_pivot: -1.96, upper_pivot: 1.96, lower: 0.107, upper: 0.331, unavailable_reason: null },
        ],
      },
    },
  };
}
