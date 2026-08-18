use serde::{Deserialize, Serialize};

use qpls_core::{CbsemBootstrapTestTail, SemCovarianceDenominatorV4};
use std::collections::BTreeSet;

/// First publication-eligible CB-SEM bootstrap contract. The historical
/// `cbsem_bootstrap_v1` payload is deliberately a different type and is never
/// upgraded or interpreted as this result.
pub const CBSEM_BOOTSTRAP_METHOD_VERSION_V2: &str = "cbsem_bootstrap_v2";
pub const CBSEM_BOOTSTRAP_ALGORITHM_V2: &str = "indexed_raw_case_refit_ml_v2";
pub const CBSEM_BOOTSTRAP_STREAM_TOKEN_V2: &str = "quickpls_cbsem_ml_case_bootstrap_v2";
pub const CBSEM_BOOTSTRAP_PRIMARY_OPERATION_V2: &str =
    "quickpls_cbsem_ml_case_bootstrap_v2:primary";
pub const CBSEM_BOOTSTRAP_INTERVAL_METHOD_V2: &str = "percentile_type7_v1";
pub const CBSEM_BOOTSTRAP_RETRY_POLICY_V2: &str = "no_retry_fixed_preplanned_primary_draws_v1";
pub const CBSEM_BOOTSTRAP_VALIDATION_WITNESS_V2: &str = "cbsem_bootstrap_validation_witness_v2";
pub const CBSEM_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V2: u8 = 1;
pub const CBSEM_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V2: f64 = 0.90;
pub const CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_METHOD_VERSION_V1: &str =
    "cbsem_exact_case_bootstrap_null_centered_test_tail_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_NULL_HYPOTHESIS_V1: &str =
    "compiled_free_parameter_equals_zero_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_STATISTIC_V1: &str =
    "unstudentized_null_centered_parameter_estimate_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_TIE_POLICY_V1: &str = "inclusive_ieee_comparison_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_PROBABILITY_METHOD_V1: &str =
    "plus_one_over_usable_plus_one_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_DECISION_RULE_V1: &str =
    "selected_p_value_less_than_or_equal_alpha_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_SIGNIFICANCE_LEVEL_V1: f64 = 0.05;
pub const CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1: &str =
    "cbsem_exact_case_bootstrap_refit_standard_errors_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1: &str =
    "cbsem_ml_expected_information_delta_method_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_METHOD_VERSION_V1: &str =
    "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_PIVOT_METHOD_V1: &str =
    "outer_estimate_minus_point_estimate_over_outer_analytic_standard_error_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_QUANTILE_METHOD_V1: &str = "percentile_type7_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_INTERVAL_METHOD_V1: &str =
    "reversed_type7_studentized_pivot_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_ARCHIVE_VALIDATION_SCOPE_V1: &str =
    "ledger_and_arithmetic_only_no_raw_refit_or_expected_information_replay_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1: &str =
    "cbsem_exact_case_bootstrap_delete_one_refit_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_BCA_METHOD_VERSION_V1: &str =
    "cbsem_exact_case_bootstrap_bca_interval_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_BCA_BIAS_CORRECTION_METHOD_V1: &str =
    "midrank_less_plus_half_ties_no_clamp_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ACCELERATION_METHOD_V1: &str =
    "complete_delete_one_jackknife_acceleration_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ADJUSTMENT_METHOD_V1: &str =
    "efron_bca_normal_cdf_adjustment_v1";
/// Validation-repaired acceleration submethod. V1 remains available so
/// append-only historical evidence can still be identified without being
/// reinterpreted as a V2 result.
pub const CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ACCELERATION_METHOD_V2: &str =
    "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2";
/// Validation-repaired adjusted-probability submethod. The inverse-normal
/// step remains statrs; only the final normal CDF is evaluated by libm erfc.
pub const CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ADJUSTMENT_METHOD_V2: &str =
    "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_BCA_QUANTILE_METHOD_V1: &str = "percentile_type7_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_BCA_RETRY_POLICY_V1: &str =
    "no_retry_exactly_one_fit_per_omitted_case_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemBootstrapAnalysisV2 {
    pub method_version: String,
    pub algorithm: String,
    pub interval_method: String,
    pub retry_policy: String,
    pub confidence_level: f64,
    pub requested_replicates: u32,
    pub attempted_fits: u32,
    pub usable_replicates: u32,
    pub failed_replicates: u32,
    pub minimum_usable_fraction: f64,
    pub minimum_usable_replicates: u32,
    pub max_attempts_per_replicate: u8,
    pub complete_case_sample_size: usize,
    pub seed: u64,
    pub stream_token: String,
    pub inference: CbsemBootstrapInferenceV2,
    pub intervals: Vec<CbsemBootstrapParameterIntervalV2>,
    pub failures: Vec<CbsemBootstrapFailedReplicateV2>,
    pub validation_witness: CbsemBootstrapValidationWitnessV2,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemBootstrapInferenceV2 {
    Available,
    Unavailable {
        reason_code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemBootstrapParameterIntervalV2 {
    pub parameter: String,
    pub original: f64,
    pub bootstrap_mean: f64,
    pub bias: f64,
    pub standard_error: f64,
    pub percentile_lower: f64,
    pub percentile_upper: f64,
    pub usable_replicates: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemBootstrapFailedReplicateV2 {
    pub replicate_index: u32,
    pub sample_indices_sha256: String,
    pub reason_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemBootstrapValidationWitnessV2 {
    pub method_version: String,
    pub dataset_fingerprint: String,
    pub recipe_sha256: String,
    pub base_result_sha256: String,
    pub parameter_names: Vec<String>,
    pub successful_replicates: Vec<CbsemBootstrapWitnessReplicateV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemBootstrapWitnessReplicateV2 {
    pub replicate_index: u32,
    pub sample_indices_sha256: String,
    pub iterations: u32,
    pub objective: f64,
    pub parameter_estimates: Vec<f64>,
}

/// Exact compiled-parameter-table case-bootstrap family. This identity is
/// deliberately distinct from `cbsem_bootstrap_v2`, whose historical bounded
/// estimator and result bytes remain unchanged.
pub const CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1: &str = "cbsem_exact_case_bootstrap_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1: &str =
    "sha256_source_fingerprint_and_ordered_u64_indices_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1: &str =
    "sha256_complete_case_n_and_ordered_sampling_positions_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1: &str =
    "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_INTERVAL_METHOD_V1: &str = "percentile_type7_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1: &str =
    "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1: &str =
    "no_retry_fixed_preplanned_primary_draws_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1: u8 = 1;
pub const CBSEM_EXACT_CASE_BOOTSTRAP_BASE_POINT_DIGEST_METHOD_V1: &str =
    "sha256_recipe_v4_cbsem_base_point_projection_v1";

/// Canonical parameter row used by the exact case-bootstrap base-point digest.
/// Its fields map one-for-one to the current CB-SEM `parameters` canonical
/// table and deliberately retain row order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapBasePointParameterV1 {
    pub parameter_id: String,
    pub name: String,
    pub kind: String,
    pub lhs: String,
    pub rhs: String,
    pub estimate: f64,
    pub standard_error: Option<f64>,
    pub z_statistic: Option<f64>,
    pub p_value_two_sided: Option<f64>,
    pub fixed: bool,
}

/// Stable scientific projection shared by runner and archive validation. Every
/// field is available in the current v8 estimation-summary, parameters, fit,
/// or score/LM canonical tables; runtime-only objects and the bootstrap
/// aggregate itself are intentionally excluded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapBasePointDigestProjectionV1 {
    pub digest_method: String,
    pub compiled_moment_schema_version: u32,
    pub moment_input_method_version: String,
    pub estimator_method_version: String,
    pub model_type: String,
    pub estimator: String,
    pub mean_structure: bool,
    pub input: String,
    pub converged: bool,
    pub iterations: u32,
    pub objective: f64,
    pub gradient_norm: f64,
    pub sample_size: usize,
    pub declared_sample_size: Option<usize>,
    pub omitted_observations: usize,
    pub covariance_denominator: SemCovarianceDenominatorV4,
    pub canonical_covariance_sha256: String,
    pub canonical_observed_means_sha256: Option<String>,
    pub parameters: Vec<CbsemExactCaseBootstrapBasePointParameterV1>,
    pub fit_method_version: String,
    pub chi_square: f64,
    pub degrees_of_freedom: i64,
    pub fit_p_value: Option<f64>,
    pub cfi: Option<f64>,
    pub tli: Option<f64>,
    pub rmsea: Option<f64>,
    pub rmsea_interval_method_version: String,
    pub rmsea_interval_confidence_level: f64,
    pub rmsea_ci_lower: Option<f64>,
    pub rmsea_ci_upper: Option<f64>,
    pub srmr: f64,
    pub aic: f64,
    pub bic: f64,
    pub score_lm: crate::CbsemCfaScoreLmBundleV1,
}

pub fn cbsem_exact_case_bootstrap_base_point_digest_projection_v1(
    result: &crate::CbsemCompiledMomentResultV2,
) -> Result<CbsemExactCaseBootstrapBasePointDigestProjectionV1, String> {
    let analysis = &result.analysis;
    if analysis.bootstrap.is_some()
        || analysis.bootstrap_v2.is_some()
        || analysis.exact_case_bootstrap.is_some()
        || analysis.exact_case_bootstrap_studentized.is_some()
        || analysis.exact_case_bootstrap_bca.is_some()
        || analysis.multigroup.is_some()
        || !analysis.modification_indices.is_empty()
    {
        return Err(
            "exact case-bootstrap base point must not contain nested, legacy, multigroup, or heuristic-MI artifacts"
                .into(),
        );
    }
    let score_lm = analysis.score_lm.clone().ok_or_else(|| {
        "exact case-bootstrap base point requires the current score/LM bundle".to_owned()
    })?;
    let rmsea = analysis
        .fit
        .rmsea_interval_attribution
        .as_ref()
        .ok_or_else(|| {
            "exact case-bootstrap base point requires RMSEA interval attribution".to_owned()
        })?;
    let input = match result.input.kind {
        crate::CbsemMomentInputKindV2::Raw => "raw",
        crate::CbsemMomentInputKindV2::Covariance => "covariance",
        crate::CbsemMomentInputKindV2::Correlation => "correlation",
    };
    let mut parameter_ids = BTreeSet::new();
    let mut parameter_names = BTreeSet::new();
    let parameters = analysis
        .parameters
        .iter()
        .map(|parameter| {
            let parameter_id = result.parameter_ids.get(&parameter.name).ok_or_else(|| {
                format!(
                    "exact case-bootstrap base parameter {} has no stable parameter id",
                    parameter.name
                )
            })?;
            if !parameter_ids.insert(parameter_id.clone())
                || !parameter_names.insert(parameter.name.clone())
            {
                return Err(
                    "exact case-bootstrap base parameter ids and names must be unique".into(),
                );
            }
            Ok(CbsemExactCaseBootstrapBasePointParameterV1 {
                parameter_id: parameter_id.clone(),
                name: parameter.name.clone(),
                kind: parameter.kind.clone(),
                lhs: parameter.lhs.clone(),
                rhs: parameter.rhs.clone(),
                estimate: parameter.estimate,
                standard_error: parameter.standard_error,
                z_statistic: parameter.z_statistic,
                p_value_two_sided: parameter.p_value_two_sided,
                fixed: parameter.fixed,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if parameters.len() != result.parameter_ids.len() {
        return Err(
            "exact case-bootstrap base parameter map cardinality differs from its rows".into(),
        );
    }
    let projection = CbsemExactCaseBootstrapBasePointDigestProjectionV1 {
        digest_method: CBSEM_EXACT_CASE_BOOTSTRAP_BASE_POINT_DIGEST_METHOD_V1.into(),
        compiled_moment_schema_version: result.schema_version,
        moment_input_method_version: result.method_version.clone(),
        estimator_method_version: analysis.method_version.clone(),
        model_type: analysis.model_type.clone(),
        estimator: analysis.estimator.clone(),
        mean_structure: analysis.mean_structure,
        input: input.into(),
        converged: analysis.converged,
        iterations: analysis.iterations,
        objective: analysis.objective,
        gradient_norm: analysis.gradient_norm,
        sample_size: analysis.sample_size,
        declared_sample_size: result.input.declared_sample_size,
        omitted_observations: result.input.omitted_observations,
        covariance_denominator: result.input.covariance_denominator,
        canonical_covariance_sha256: result.input.canonical_ml_covariance_sha256.clone(),
        canonical_observed_means_sha256: result.input.canonical_observed_means_sha256.clone(),
        parameters,
        fit_method_version: analysis.fit.method_version.clone(),
        chi_square: analysis.fit.chi_square,
        degrees_of_freedom: analysis.fit.degrees_of_freedom,
        fit_p_value: analysis.fit.p_value,
        cfi: analysis.fit.cfi,
        tli: analysis.fit.tli,
        rmsea: analysis.fit.rmsea,
        rmsea_interval_method_version: rmsea.method_version.clone(),
        rmsea_interval_confidence_level: rmsea.confidence_level,
        rmsea_ci_lower: analysis.fit.rmsea_ci_lower,
        rmsea_ci_upper: analysis.fit.rmsea_ci_upper,
        srmr: analysis.fit.srmr,
        aic: analysis.fit.aic,
        bic: analysis.fit.bic,
        score_lm,
    };
    validate_cbsem_exact_case_bootstrap_base_point_digest_projection_v1(&projection)?;
    Ok(projection)
}

pub fn cbsem_exact_case_bootstrap_base_point_sha256_v1(
    projection: &CbsemExactCaseBootstrapBasePointDigestProjectionV1,
) -> Result<String, String> {
    validate_cbsem_exact_case_bootstrap_base_point_digest_projection_v1(projection)?;
    Ok(qpls_core::sha256_serialized(projection))
}

fn validate_cbsem_exact_case_bootstrap_base_point_digest_projection_v1(
    projection: &CbsemExactCaseBootstrapBasePointDigestProjectionV1,
) -> Result<(), String> {
    let finite_optional = |value: Option<f64>| value.is_none_or(f64::is_finite);
    let digest = &projection.canonical_covariance_sha256;
    if projection.digest_method != CBSEM_EXACT_CASE_BOOTSTRAP_BASE_POINT_DIGEST_METHOD_V1
        || projection.compiled_moment_schema_version
            != crate::CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2
        || projection.moment_input_method_version
            != crate::CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3
        || projection.estimator_method_version
            != crate::CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        || projection.model_type != "cfa"
        || projection.estimator != "ml"
        || projection.mean_structure
        || projection.input != "raw"
        || !projection.converged
        || !projection.objective.is_finite()
        || projection.objective < 0.0
        || !projection.gradient_norm.is_finite()
        || projection.gradient_norm < 0.0
        || projection.sample_size < 10
        || projection.declared_sample_size.is_some()
        || projection.covariance_denominator != SemCovarianceDenominatorV4::MaximumLikelihoodN
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || projection.canonical_observed_means_sha256.is_some()
        || projection.parameters.is_empty()
        || !projection.chi_square.is_finite()
        || projection.degrees_of_freedom < 0
        || !finite_optional(projection.fit_p_value)
        || !finite_optional(projection.cfi)
        || !finite_optional(projection.tli)
        || !finite_optional(projection.rmsea)
        || !projection.rmsea_interval_confidence_level.is_finite()
        || projection.fit_method_version != crate::CBSEM_FIT_METHOD_VERSION
        || projection.rmsea_interval_method_version
            != crate::CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1
        || projection.rmsea_interval_confidence_level.to_bits() != 0.90_f64.to_bits()
        || !finite_optional(projection.rmsea_ci_lower)
        || !finite_optional(projection.rmsea_ci_upper)
        || !projection.srmr.is_finite()
        || !projection.aic.is_finite()
        || !projection.bic.is_finite()
        || projection.score_lm.method_version != crate::CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1
        || projection.score_lm.scope != crate::CBSEM_CFA_SCORE_LM_SCOPE_V1
    {
        return Err("exact case-bootstrap base-point digest projection is outside the canonical v8 CFA point scope".into());
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    if projection.parameters.iter().any(|parameter| {
        parameter.parameter_id.trim().is_empty()
            || parameter.name.trim().is_empty()
            || !ids.insert(parameter.parameter_id.as_str())
            || !names.insert(parameter.name.as_str())
            || !parameter.estimate.is_finite()
            || !finite_optional(parameter.standard_error)
            || !finite_optional(parameter.z_statistic)
            || !finite_optional(parameter.p_value_two_sided)
            || parameter
                .p_value_two_sided
                .is_some_and(|value| !(0.0..=1.0).contains(&value))
    }) {
        return Err("exact case-bootstrap base-point parameter projection is invalid".into());
    }
    let parameter_by_id = projection
        .parameters
        .iter()
        .map(|parameter| (parameter.parameter_id.as_str(), parameter))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut score_ids = BTreeSet::new();
    for row in &projection.score_lm.rows {
        let Some(parameter) = parameter_by_id.get(row.parameter_id.as_str()) else {
            return Err(
                "exact case-bootstrap score/LM row is not bound to a canonical parameter row"
                    .into(),
            );
        };
        if !score_ids.insert(row.parameter_id.as_str())
            || !parameter.fixed
            || parameter.estimate.to_bits() != 0.0_f64.to_bits()
            || row.kind != "residual_covariance"
            || parameter.kind != row.kind
            || parameter.lhs != row.lhs
            || parameter.rhs != row.rhs
        {
            return Err(
                "exact case-bootstrap score/LM identity, order, or parameter binding is invalid"
                    .into(),
            );
        }
        if let crate::CbsemCfaScoreLmOutcomeV1::Available {
            score,
            efficient_score,
            candidate_information,
            efficient_information,
            modification_index,
            expected_parameter_change,
            p_value,
        } = &row.outcome
        {
            if !score.is_finite()
                || !efficient_score.is_finite()
                || !candidate_information.is_finite()
                || *candidate_information <= 0.0
                || !efficient_information.is_finite()
                || *efficient_information <= 0.0
                || !modification_index.is_finite()
                || *modification_index < 0.0
                || !expected_parameter_change.is_finite()
                || !p_value.is_finite()
                || !(0.0..=1.0).contains(p_value)
            {
                return Err("exact case-bootstrap score/LM numeric projection is invalid".into());
            }
        }
    }
    Ok(())
}

/// Additive aggregate contract reserved for the exact case-bootstrap
/// scheduler. Estimation only produces [`CbsemExactCaseBootstrapRefitV1`]; it
/// never constructs or attaches this aggregate by itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapResultV1 {
    pub method_version: String,
    pub estimator_method_version: String,
    pub source_dataset_id: String,
    pub source_dataset_fingerprint: String,
    /// Analytical identity of the outer bootstrap RecipeV4. This is distinct
    /// from the point-estimator compiler identity below.
    pub outer_recipe_analytical_identity_sha256: String,
    /// Canonical digest of the exact point result to which this inference is
    /// attached. A bootstrap aggregate may never float between point results.
    pub base_point_result_sha256: String,
    pub compiler_analytical_identity_sha256: String,
    pub plan_sha256: String,
    pub model_scientific_sha256: String,
    pub complete_case_sample_size: usize,
    pub complete_case_universe_digest_method: String,
    pub complete_case_universe_sha256: String,
    pub covariance_denominator: SemCovarianceDenominatorV4,
    pub sample_indices_digest_method: String,
    pub sampling_positions_digest_method: String,
    pub interval_method: String,
    pub confidence_level: f64,
    pub requested_replicates: u32,
    pub attempted_refits: u32,
    pub usable_replicates: u32,
    pub failed_replicates: u32,
    pub minimum_usable_fraction: f64,
    pub minimum_usable_replicates: u32,
    pub seed: u64,
    pub stream_token: String,
    pub retry_policy: String,
    pub max_attempts_per_replicate: u8,
    pub parameter_ids: Vec<String>,
    pub inference: CbsemExactCaseBootstrapInferenceV1,
    pub intervals: Vec<CbsemExactCaseBootstrapParameterIntervalV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hypothesis_tests: Option<CbsemExactCaseBootstrapHypothesisTestsV1>,
    pub successful_refits: Vec<CbsemExactCaseBootstrapWitnessV1>,
    pub failed_refits: Vec<CbsemExactCaseBootstrapFailureV1>,
}

/// Exact null-centered tests computed from the same successful-refit ledger as
/// the percentile intervals. The canonical null is positive IEEE zero.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapHypothesisTestsV1 {
    pub method_version: String,
    pub null_hypothesis: String,
    pub statistic: String,
    pub tie_policy: String,
    pub probability_method: String,
    pub decision_rule: String,
    pub selected_test_tail: CbsemBootstrapTestTail,
    pub null_value: f64,
    pub significance_level: f64,
    pub usable_replicates: u32,
    pub inference: CbsemExactCaseBootstrapHypothesisTestInferenceV1,
    pub parameters: Vec<CbsemExactCaseBootstrapHypothesisTestParameterV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemExactCaseBootstrapHypothesisTestInferenceV1 {
    Available,
    Unavailable {
        reason_code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapHypothesisTestParameterV1 {
    pub parameter_id: String,
    pub outcome: CbsemExactCaseBootstrapHypothesisTestOutcomeV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemExactCaseBootstrapHypothesisTestOutcomeV1 {
    Available {
        point_estimate: f64,
        two_sided_exceedances: u32,
        greater_or_equal_exceedances: u32,
        less_or_equal_exceedances: u32,
        p_value_two_sided: f64,
        p_value_greater: f64,
        p_value_less: f64,
        selected_exceedances: u32,
        selected_p_value: f64,
        reject_null: bool,
    },
    Unavailable {
        reason: CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1 {
    InsufficientUsableReplicates,
    NonregularVarianceBoundary,
    ZeroNullOutsideOpenDomain,
    UnsupportedParameterFamily,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemExactCaseBootstrapInferenceV1 {
    Available,
    Unavailable {
        reason_code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapParameterIntervalV1 {
    pub parameter_id: String,
    pub original: f64,
    pub bootstrap_mean: f64,
    pub bias: f64,
    pub standard_error: f64,
    pub percentile_lower: f64,
    pub percentile_upper: f64,
    pub usable_replicates: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapWitnessV1 {
    pub replicate_index: u32,
    pub sampling_positions_sha256: String,
    pub sample_indices_sha256: String,
    pub parameter_estimates: Vec<f64>,
    pub iterations: u32,
    pub objective: f64,
    pub gradient_norm: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemExactCaseBootstrapFailureKindV1 {
    MomentMatrixNotPositiveDefinite,
    NonConvergence,
    InadmissibleSolution,
    NumericalFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapFailureV1 {
    pub replicate_index: u32,
    pub sampling_positions_sha256: String,
    pub sample_indices_sha256: String,
    pub kind: CbsemExactCaseBootstrapFailureKindV1,
    pub message: String,
}

/// Deterministic receipt available before fitting a replicate, so both usable
/// and failed fits retain the same schedule and mapped-source-row identities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapSamplingReceiptV1 {
    pub complete_case_sample_size: usize,
    pub sampling_positions_digest_method: String,
    pub sampling_positions_sha256: String,
    pub sample_indices_digest_method: String,
    pub sample_indices_sha256: String,
}

/// One exact ML refit over an ordered, with-replacement source-row draw.
/// Parameter rows are returned in the exact compiled parameter-table order;
/// equality-constrained free rows retain their distinct stable parameter ids.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapRefitV1 {
    pub method_version: String,
    pub estimator_method_version: String,
    pub source_dataset_id: String,
    pub source_dataset_fingerprint: String,
    pub compiler_analytical_identity_sha256: String,
    pub plan_sha256: String,
    pub model_scientific_sha256: String,
    pub source_row_count: usize,
    pub complete_case_sample_size: usize,
    pub complete_case_universe_digest_method: String,
    pub complete_case_universe_sha256: String,
    pub resampled_observations: usize,
    pub covariance_denominator: SemCovarianceDenominatorV4,
    pub sample_indices_digest_method: String,
    pub sampling_positions_digest_method: String,
    pub sampling_positions_sha256: String,
    pub sample_indices_sha256: String,
    pub free_parameters: Vec<CbsemExactCaseBootstrapParameterEstimateV1>,
    pub iterations: u32,
    pub objective: f64,
    pub gradient_norm: f64,
}

/// Explicit opt-in envelope for the analytically studentized scheduler.
/// Keeping this distinct leaves the established refit type and v1/v9/v10
/// serializations byte- and source-compatible.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapRefitWithAnalyticStandardErrorsV1 {
    pub refit: CbsemExactCaseBootstrapRefitV1,
    pub standard_errors: CbsemExactCaseBootstrapRefitStandardErrorsV1,
}

/// Opt-in scheduler result for analytically studentized inference. `base` is
/// the unchanged v10 aggregate produced by the established scheduler; all new
/// evidence and arithmetic live in the distinct sidecar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapWithStudentizedResultV1 {
    pub base: CbsemExactCaseBootstrapResultV1,
    pub studentized: CbsemExactCaseBootstrapStudentizedSidecarV1,
}

/// Opt-in exact bootstrap plus BCa sidecar. The base draw ledger and its
/// delete-one derivative are stored atomically so no consumer can mix a BCa
/// sidecar with a different percentile authority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapWithBcaResultV1 {
    pub base: CbsemExactCaseBootstrapResultV1,
    pub bca: CbsemExactCaseBootstrapBcaSidecarV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapStudentizedSidecarV1 {
    pub method_version: String,
    pub standard_error_method_version: String,
    pub expected_information_method: String,
    pub pivot_method: String,
    pub quantile_method: String,
    pub interval_method: String,
    pub archive_validation_scope: String,
    pub confidence_level: f64,
    pub minimum_usable_fraction: f64,
    pub minimum_usable_replicates: u32,
    pub studentized_usable_replicates: u32,
    pub parameter_ids: Vec<String>,
    pub point_standard_errors: CbsemExactCaseBootstrapRefitStandardErrorsV1,
    pub inference: CbsemExactCaseBootstrapStudentizedInferenceV1,
    pub intervals: Vec<CbsemExactCaseBootstrapStudentizedParameterIntervalV1>,
    /// Exactly one compact receipt for each row in `base.successful_refits`, in
    /// the same replicate order. Parameter order is `parameter_ids`; estimates
    /// and pivots are intentionally not duplicated here.
    pub refit_standard_errors: Vec<CbsemExactCaseBootstrapStudentizedRefitStandardErrorsV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapStudentizedRefitStandardErrorsV1 {
    pub replicate_index: u32,
    pub outcome: CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1 {
    Available {
        information_method: String,
        standard_errors: Vec<f64>,
    },
    Unavailable {
        reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemExactCaseBootstrapStudentizedInferenceV1 {
    Available,
    Unavailable {
        reason: CbsemExactCaseBootstrapStudentizedUnavailableReasonV1,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemExactCaseBootstrapStudentizedUnavailableReasonV1 {
    PointStandardErrorsUnavailable,
    InsufficientStudentizedUsableReplicates,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapStudentizedParameterIntervalV1 {
    pub parameter_id: String,
    pub outcome: CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1 {
    Available {
        point_estimate: f64,
        point_standard_error: f64,
        lower_pivot_quantile: f64,
        upper_pivot_quantile: f64,
        interval_lower: f64,
        interval_upper: f64,
        usable_replicates: u32,
    },
    Unavailable {
        reason: CbsemExactCaseBootstrapStudentizedUnavailableReasonV1,
    },
}

/// Distinct N-1 exact-ML refit. It is never serialized or interpreted as a
/// with-replacement bootstrap refit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapDeleteOneRefitV1 {
    pub method_version: String,
    pub estimator_method_version: String,
    pub source_dataset_id: String,
    pub source_dataset_fingerprint: String,
    pub compiler_analytical_identity_sha256: String,
    pub plan_sha256: String,
    pub model_scientific_sha256: String,
    pub source_row_count: usize,
    pub complete_case_sample_size: usize,
    pub complete_case_universe_sha256: String,
    pub omitted_complete_case_position: usize,
    pub omitted_source_row_index: usize,
    pub retained_observations: usize,
    pub covariance_denominator: SemCovarianceDenominatorV4,
    pub sampling_positions_digest_method: String,
    pub retained_sampling_positions_sha256: String,
    pub sample_indices_digest_method: String,
    pub retained_sample_indices_sha256: String,
    pub free_parameters: Vec<CbsemExactCaseBootstrapParameterEstimateV1>,
    pub iterations: u32,
    pub objective: f64,
    pub gradient_norm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapBcaSidecarV1 {
    pub method_version: String,
    pub base_bootstrap_method_version: String,
    pub outer_recipe_analytical_identity_sha256: String,
    pub base_point_result_sha256: String,
    pub compiler_analytical_identity_sha256: String,
    pub plan_sha256: String,
    pub model_scientific_sha256: String,
    pub delete_one_refit_method_version: String,
    pub bias_correction_method: String,
    pub acceleration_method: String,
    pub adjusted_probability_method: String,
    pub quantile_method: String,
    pub retry_policy: String,
    pub confidence_level: f64,
    pub bootstrap_usable_replicates: u32,
    pub minimum_bootstrap_usable_replicates: u32,
    pub delete_one_case_count: usize,
    pub parameter_ids: Vec<String>,
    pub inference: CbsemExactCaseBootstrapBcaInferenceV1,
    pub intervals: Vec<CbsemExactCaseBootstrapBcaParameterIntervalV1>,
    pub successful_delete_one_refits: Vec<CbsemExactCaseBootstrapDeleteOneWitnessV1>,
    pub failed_delete_one_refits: Vec<CbsemExactCaseBootstrapDeleteOneFailureV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemExactCaseBootstrapBcaInferenceV1 {
    Available,
    Unavailable {
        reason: CbsemExactCaseBootstrapBcaUnavailableReasonV1,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemExactCaseBootstrapBcaUnavailableReasonV1 {
    BaseInferenceUnavailable,
    IncompleteDeleteOneLedger,
    BiasCorrectionProbabilityAtBoundary,
    DegenerateJackknifeAcceleration,
    NonfiniteJackknifeArithmetic,
    SingularAccelerationAdjustment,
    InvalidAdjustedProbability,
    AdjustedProbabilityOrderInvalid,
    NonfiniteOrReversedInterval,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapBcaParameterIntervalV1 {
    pub parameter_id: String,
    pub outcome: CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1 {
    Available {
        point_estimate: f64,
        bias_correction: f64,
        acceleration: f64,
        adjusted_lower_probability: f64,
        adjusted_upper_probability: f64,
        interval_lower: f64,
        interval_upper: f64,
        usable_replicates: u32,
    },
    Unavailable {
        reason: CbsemExactCaseBootstrapBcaUnavailableReasonV1,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapDeleteOneWitnessV1 {
    pub omitted_complete_case_position: usize,
    pub omitted_source_row_index: usize,
    pub retained_sampling_positions_sha256: String,
    pub retained_sample_indices_sha256: String,
    pub parameter_estimates: Vec<f64>,
    pub iterations: u32,
    pub objective: f64,
    pub gradient_norm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapDeleteOneFailureV1 {
    pub omitted_complete_case_position: usize,
    pub omitted_source_row_index: usize,
    pub retained_sampling_positions_sha256: String,
    pub retained_sample_indices_sha256: String,
    pub kind: CbsemExactCaseBootstrapFailureKindV1,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapParameterEstimateV1 {
    pub parameter_id: String,
    pub estimate: f64,
}

/// Whole-vector analytical-SE receipt for one successful exact-ML refit.
/// Information failure never converts the successful point refit into a
/// failed bootstrap replicate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapRefitStandardErrorsV1 {
    pub method_version: String,
    pub outcome: CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1 {
    Available {
        information_method: String,
        parameters: Vec<CbsemExactCaseBootstrapParameterStandardErrorV1>,
    },
    Unavailable {
        reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactCaseBootstrapParameterStandardErrorV1 {
    pub parameter_id: String,
    pub standard_error: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1 {
    SingularInformation,
    InformationNotPositiveDefinite,
    InvalidInformationVarianceOrStandardError,
    DerivativeUnavailable,
    NumericalInformationFailure,
}
