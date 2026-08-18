use qpls_core::{
    CompiledAnalysisRecipeV4, CompiledCbsemInputV2, CompiledCbsemParameterRoleV2,
    CompiledCbsemParameterStatusV2, CompiledCbsemPlanV2, CompiledRecipePlanV4, MissingDataPolicyV4,
    RECIPE_V4_CBSEM_COMPILER_VERSION, RECIPE_V4_COMPILATION_RECEIPT_SCHEMA_VERSION,
    RecipeV4CompilerTarget, SemCovarianceDenominatorV4, SemGroupV4, SemVariableV4,
};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ChiSquared, ContinuousCDF};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3, CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
    CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2, CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3,
    CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3, CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
    CBSEM_FIT_METHOD_VERSION, CbsemCompiledMomentResultV2, CbsemMomentInputKindV2,
    cbsem_exact_parameter_table::{
        cbsem_exact_covariance_ml_discrepancy_v1, cbsem_exact_mean_structure_ml_discrepancy_v1,
    },
    cbsem_matrix_input::{
        covariance_sha256, ensure_strict_positive_definite, observed_means_sha256,
        validate_symmetric_matrix,
    },
};

pub const CBSEM_EXACT_NESTED_LRT_METHOD_VERSION_V1: &str = "cbsem_exact_nested_model_lrt_v1";
pub const CBSEM_EXACT_NESTING_PROOF_METHOD_VERSION_V1: &str =
    "cbsem_exact_compiled_plan_nesting_proof_v1";
pub const CBSEM_EXACT_LRT_NEGATIVE_DIFFERENCE_RELATIVE_TOLERANCE_V1: f64 = 1.0e-10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactLrtDataAuthorityV1 {
    pub dataset_id: String,
    pub dataset_fingerprint: String,
    pub variable_ids: Vec<String>,
    pub source_columns: Vec<String>,
    pub used_sample_size: usize,
    pub omitted_observations: usize,
    pub input_kind: CbsemMomentInputKindV2,
    pub missing_data: MissingDataPolicyV4,
    pub covariance_denominator: SemCovarianceDenominatorV4,
    pub moment_schema_version: u32,
    pub moment_method_version: String,
    pub canonical_ml_covariance_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_observed_means_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactLrtFitReceiptV1 {
    pub compiler_analytical_identity_sha256: String,
    pub plan_sha256: String,
    pub model_scientific_sha256: String,
    pub data: CbsemExactLrtDataAuthorityV1,
    pub estimator_method_version: String,
    pub fit_method_version: String,
    pub model_type: String,
    pub estimator: String,
    pub input: String,
    pub mean_structure: bool,
    pub converged: bool,
    pub admissible: bool,
    pub iterations: u32,
    pub objective: f64,
    pub gradient_norm: f64,
    pub chi_square: f64,
    pub degrees_of_freedom: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactNestingProofV1 {
    pub method_version: String,
    pub unrestricted_plan_sha256: String,
    pub restricted_plan_sha256: String,
    pub unrestricted_free_dimensions: usize,
    pub restricted_free_dimensions: usize,
    pub dimension_reduction: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactNestedLrtResultV1 {
    pub method_version: String,
    pub data: CbsemExactLrtDataAuthorityV1,
    pub nesting: CbsemExactNestingProofV1,
    pub unrestricted: CbsemExactLrtFitReceiptV1,
    pub restricted: CbsemExactLrtFitReceiptV1,
    pub raw_chi_square_difference: f64,
    pub negative_difference_tolerance: f64,
    pub likelihood_ratio_statistic: f64,
    pub delta_degrees_of_freedom: i64,
    pub p_value: f64,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum CbsemExactNestedLrtErrorV1 {
    #[error("exact nested-model plan contract is invalid: {0}")]
    InvalidPlan(String),
    #[error("compiled recipe artifact is invalid for exact nested-model LRT: {0}")]
    InvalidArtifact(String),
    #[error("restricted plan is not a strict nested reduction: {0}")]
    NotNested(String),
    #[error("exact ML fit receipt is invalid: {0}")]
    InvalidFit(String),
    #[error("exact nested-model fits do not share the same data authority")]
    DifferentDataAuthority,
    #[error("chi-square difference {difference} is below negative tolerance {tolerance}")]
    NegativeChiSquareDifference { difference: f64, tolerance: f64 },
    #[error("chi-square upper-tail evaluation failed")]
    ChiSquareTail,
}

pub fn prove_cbsem_exact_nesting_v1(
    unrestricted: &CompiledCbsemPlanV2,
    restricted: &CompiledCbsemPlanV2,
) -> Result<CbsemExactNestingProofV1, CbsemExactNestedLrtErrorV1> {
    validate_shared_plan_surface(unrestricted, restricted)?;
    let unrestricted_rows = parameter_rows(unrestricted)?;
    let restricted_rows = parameter_rows(restricted)?;
    if unrestricted_rows.len() != restricted_rows.len() {
        return Err(CbsemExactNestedLrtErrorV1::NotNested(
            "parameter row counts differ".into(),
        ));
    }

    let unrestricted_groups = free_groups(&unrestricted_rows)?;
    let restricted_groups = free_groups(&restricted_rows)?;
    let mut unrestricted_to_restricted = BTreeMap::<String, String>::new();
    let mut unrestricted_fixed_values = BTreeMap::<String, u64>::new();

    for (unrestricted_row, restricted_row) in unrestricted_rows.iter().zip(&restricted_rows) {
        if unrestricted_row.id != restricted_row.id
            || unrestricted_row.label != restricted_row.label
            || unrestricted_row.target != restricted_row.target
            || unrestricted_row.role != restricted_row.role
        {
            return Err(CbsemExactNestedLrtErrorV1::NotNested(format!(
                "parameter identity, label, target, or family differs at {}",
                unrestricted_row.id
            )));
        }
        match (&unrestricted_row.status, &restricted_row.status) {
            (Status::Fixed(left), Status::Fixed(right)) if left.to_bits() == right.to_bits() => {}
            (Status::Fixed(_), _) => {
                return Err(CbsemExactNestedLrtErrorV1::NotNested(format!(
                    "fixed unrestricted parameter {} was released or changed",
                    unrestricted_row.id
                )));
            }
            (
                Status::Free {
                    start: left_start,
                    lower: left_lower,
                    upper: left_upper,
                    ..
                },
                Status::Free {
                    start: right_start,
                    lower: right_lower,
                    upper: right_upper,
                    ..
                },
            ) => {
                if !same_optional_bits(*left_start, *right_start)
                    || !same_optional_bits(*left_lower, *right_lower)
                    || !same_optional_bits(*left_upper, *right_upper)
                {
                    return Err(CbsemExactNestedLrtErrorV1::NotNested(format!(
                        "free-parameter start or bounds changed at {}",
                        unrestricted_row.id
                    )));
                }
                let source = unrestricted_groups[&unrestricted_row.id].clone();
                let target = restricted_groups[&restricted_row.id].clone();
                if unrestricted_fixed_values.contains_key(&source)
                    || unrestricted_to_restricted
                        .insert(source.clone(), target.clone())
                        .is_some_and(|previous| previous != target)
                {
                    return Err(CbsemExactNestedLrtErrorV1::NotNested(format!(
                        "unrestricted equality dimension {source} was split"
                    )));
                }
            }
            (Status::Free { lower, upper, .. }, Status::Fixed(value)) => {
                let effective_lower =
                    if unrestricted_row.role == CompiledCbsemParameterRoleV2::Variance {
                        Some(lower.unwrap_or(0.0).max(0.0))
                    } else {
                        *lower
                    };
                if !value.is_finite()
                    || effective_lower.is_some_and(|bound| *value <= bound)
                    || upper.is_some_and(|bound| *value >= bound)
                {
                    return Err(CbsemExactNestedLrtErrorV1::NotNested(format!(
                        "fixed value for {} is not interior to its unrestricted domain",
                        unrestricted_row.id
                    )));
                }
                let source = unrestricted_groups[&unrestricted_row.id].clone();
                if unrestricted_fixed_values
                    .insert(source.clone(), value.to_bits())
                    .is_some_and(|previous| previous != value.to_bits())
                    || unrestricted_to_restricted.contains_key(&source)
                {
                    return Err(CbsemExactNestedLrtErrorV1::NotNested(format!(
                        "unrestricted equality dimension {source} was partially or inconsistently fixed"
                    )));
                }
            }
        }
    }

    for source in unrestricted_groups.values() {
        if !unrestricted_to_restricted.contains_key(source)
            && !unrestricted_fixed_values.contains_key(source)
        {
            return Err(CbsemExactNestedLrtErrorV1::NotNested(format!(
                "unrestricted equality dimension {source} has no complete restricted image"
            )));
        }
    }
    let unrestricted_dimensions = unrestricted_groups
        .values()
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let restricted_dimensions = restricted_groups
        .values()
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let dimension_reduction = unrestricted_dimensions
        .checked_sub(restricted_dimensions)
        .ok_or_else(|| {
            CbsemExactNestedLrtErrorV1::NotNested("restricted plan has more free dimensions".into())
        })?;
    if dimension_reduction == 0 {
        return Err(CbsemExactNestedLrtErrorV1::NotNested(
            "restriction does not reduce an independent free dimension".into(),
        ));
    }
    Ok(CbsemExactNestingProofV1 {
        method_version: CBSEM_EXACT_NESTING_PROOF_METHOD_VERSION_V1.into(),
        unrestricted_plan_sha256: unrestricted.deterministic_sha256(),
        restricted_plan_sha256: restricted.deterministic_sha256(),
        unrestricted_free_dimensions: unrestricted_dimensions,
        restricted_free_dimensions: restricted_dimensions,
        dimension_reduction,
    })
}

pub fn compare_cbsem_exact_nested_ml_v1(
    unrestricted_artifact: &CompiledAnalysisRecipeV4,
    unrestricted_result: &CbsemCompiledMomentResultV2,
    restricted_artifact: &CompiledAnalysisRecipeV4,
    restricted_result: &CbsemCompiledMomentResultV2,
) -> Result<CbsemExactNestedLrtResultV1, CbsemExactNestedLrtErrorV1> {
    let unrestricted_plan = validated_cbsem_artifact_plan(unrestricted_artifact)?;
    let restricted_plan = validated_cbsem_artifact_plan(restricted_artifact)?;
    let nesting = prove_cbsem_exact_nesting_v1(unrestricted_plan, restricted_plan)?;
    let unrestricted = validated_fit_receipt(
        unrestricted_artifact,
        unrestricted_plan,
        unrestricted_result,
    )?;
    let restricted =
        validated_fit_receipt(restricted_artifact, restricted_plan, restricted_result)?;
    if unrestricted.data != restricted.data
        || unrestricted.model_type != restricted.model_type
        || unrestricted.input != restricted.input
        || unrestricted.mean_structure != restricted.mean_structure
    {
        return Err(CbsemExactNestedLrtErrorV1::DifferentDataAuthority);
    }
    let expected_delta_df = i64::try_from(nesting.dimension_reduction).map_err(|_| {
        CbsemExactNestedLrtErrorV1::InvalidPlan("dimension reduction exceeds i64".into())
    })?;
    let delta_degrees_of_freedom = restricted.degrees_of_freedom - unrestricted.degrees_of_freedom;
    if delta_degrees_of_freedom <= 0 || delta_degrees_of_freedom != expected_delta_df {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "fit delta-df does not equal the independently proved dimension reduction".into(),
        ));
    }
    let raw = restricted.chi_square - unrestricted.chi_square;
    let tolerance = CBSEM_EXACT_LRT_NEGATIVE_DIFFERENCE_RELATIVE_TOLERANCE_V1
        * restricted
            .chi_square
            .abs()
            .max(unrestricted.chi_square.abs())
            .max(1.0);
    if raw < -tolerance {
        return Err(CbsemExactNestedLrtErrorV1::NegativeChiSquareDifference {
            difference: raw,
            tolerance,
        });
    }
    let statistic = raw.max(0.0);
    let distribution = ChiSquared::new(delta_degrees_of_freedom as f64)
        .map_err(|_| CbsemExactNestedLrtErrorV1::ChiSquareTail)?;
    let p_value = distribution.sf(statistic);
    if !p_value.is_finite() {
        return Err(CbsemExactNestedLrtErrorV1::ChiSquareTail);
    }
    Ok(CbsemExactNestedLrtResultV1 {
        method_version: CBSEM_EXACT_NESTED_LRT_METHOD_VERSION_V1.into(),
        data: unrestricted.data.clone(),
        nesting,
        unrestricted,
        restricted,
        raw_chi_square_difference: raw,
        negative_difference_tolerance: tolerance,
        likelihood_ratio_statistic: statistic,
        delta_degrees_of_freedom,
        p_value,
    })
}

#[derive(Clone)]
struct ParameterRow {
    id: String,
    label: String,
    target: qpls_core::SemParameterTargetV4,
    role: CompiledCbsemParameterRoleV2,
    status: Status,
}

#[derive(Clone)]
enum Status {
    Free {
        start: Option<f64>,
        lower: Option<f64>,
        upper: Option<f64>,
        equality_label: Option<String>,
    },
    Fixed(f64),
}

fn parameter_rows(
    plan: &CompiledCbsemPlanV2,
) -> Result<Vec<ParameterRow>, CbsemExactNestedLrtErrorV1> {
    plan.parameters()
        .iter()
        .map(|row| {
            if !row.group_overrides().is_empty() {
                return Err(CbsemExactNestedLrtErrorV1::InvalidPlan(
                    "group overrides are outside the exact single-group LRT slice".into(),
                ));
            }
            let status = match row.specification() {
                CompiledCbsemParameterStatusV2::Free {
                    start,
                    lower,
                    upper,
                    equality_label,
                } => Status::Free {
                    start: *start,
                    lower: *lower,
                    upper: *upper,
                    equality_label: equality_label.clone(),
                },
                CompiledCbsemParameterStatusV2::Fixed { value } if value.is_finite() => {
                    Status::Fixed(*value)
                }
                CompiledCbsemParameterStatusV2::Fixed { .. } => {
                    return Err(CbsemExactNestedLrtErrorV1::InvalidPlan(
                        "fixed parameter value is nonfinite".into(),
                    ));
                }
                CompiledCbsemParameterStatusV2::Derived { .. } => {
                    return Err(CbsemExactNestedLrtErrorV1::InvalidPlan(
                        "derived parameters are outside the exact LRT slice".into(),
                    ));
                }
            };
            Ok(ParameterRow {
                id: row.id().into(),
                label: row.label().into(),
                target: row.target().clone(),
                role: row.role(),
                status,
            })
        })
        .collect()
}

fn free_groups(
    rows: &[ParameterRow],
) -> Result<BTreeMap<String, String>, CbsemExactNestedLrtErrorV1> {
    let mut groups = BTreeMap::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut roles = BTreeMap::<String, CompiledCbsemParameterRoleV2>::new();
    for row in rows {
        if let Status::Free { equality_label, .. } = &row.status {
            let key = match equality_label {
                Some(label) if label.trim().is_empty() => {
                    return Err(CbsemExactNestedLrtErrorV1::InvalidPlan(format!(
                        "empty equality label at {}",
                        row.id
                    )));
                }
                Some(label) => format!("equality:{}", label.trim()),
                None => format!("parameter:{}", row.id),
            };
            if let Some(existing_role) = roles.insert(key.clone(), row.role) {
                if existing_role != row.role {
                    return Err(CbsemExactNestedLrtErrorV1::InvalidPlan(format!(
                        "equality dimension {key} spans different parameter families"
                    )));
                }
            }
            *counts.entry(key.clone()).or_default() += 1;
            groups.insert(row.id.clone(), key);
        }
    }
    if let Some((key, _)) = counts
        .iter()
        .find(|(key, count)| key.starts_with("equality:") && **count < 2)
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidPlan(format!(
            "singleton equality dimension {key}"
        )));
    }
    Ok(groups)
}

fn validate_shared_plan_surface(
    left: &CompiledCbsemPlanV2,
    right: &CompiledCbsemPlanV2,
) -> Result<(), CbsemExactNestedLrtErrorV1> {
    let raw_listwise = |plan: &CompiledCbsemPlanV2| {
        matches!(
            plan.input(),
            CompiledCbsemInputV2::Raw {
                missing_data: MissingDataPolicyV4::ListwiseDeletion,
                weight: None,
                cluster_variable: None,
                strata_variable: None,
                ..
            }
        )
    };
    if !raw_listwise(left)
        || !raw_listwise(right)
        || !matches!(left.group(), SemGroupV4::SingleGroup)
        || !matches!(right.group(), SemGroupV4::SingleGroup)
        || !left.constraints().is_empty()
        || !right.constraints().is_empty()
        || !left.derived_terms().is_empty()
        || !right.derived_terms().is_empty()
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidPlan("only raw-listwise, unweighted, single-group exact plans without explicit constraints or derived terms are supported".into()));
    }
    if left.schema_version() != right.schema_version()
        || left.input() != right.input()
        || left.variables() != right.variables()
        || left.loadings() != right.loadings()
        || left.causal_measurements() != right.causal_measurements()
        || left.regressions() != right.regressions()
        || left.covariances() != right.covariances()
        || left.has_feedback() != right.has_feedback()
    {
        return Err(CbsemExactNestedLrtErrorV1::NotNested(
            "input, variables, relations, or structural settings differ".into(),
        ));
    }
    Ok(())
}

fn validated_cbsem_artifact_plan(
    artifact: &CompiledAnalysisRecipeV4,
) -> Result<&CompiledCbsemPlanV2, CbsemExactNestedLrtErrorV1> {
    let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
        return Err(CbsemExactNestedLrtErrorV1::InvalidArtifact(
            "artifact plan is not CB-SEM v2".into(),
        ));
    };
    let receipt = artifact.receipt();
    let reconstructed_scientific_sha256 = plan
        .to_scientific_sem_model_v4()
        .scientific_sha256()
        .map_err(|error| {
            CbsemExactNestedLrtErrorV1::InvalidArtifact(format!(
                "compiled plan does not reconstruct a valid scientific model: {error}"
            ))
        })?;
    let hashes_are_canonical = [
        receipt.recipe_document_sha256(),
        receipt.recipe_analytical_sha256(),
        receipt.model_document_sha256(),
        receipt.model_scientific_sha256(),
        receipt.plan_sha256(),
        receipt.analytical_identity_sha256(),
    ]
    .into_iter()
    .all(is_lower_sha256);
    if receipt.schema_version() != RECIPE_V4_COMPILATION_RECEIPT_SCHEMA_VERSION
        || receipt.compiler_target() != RecipeV4CompilerTarget::CbsemPlanV2
        || receipt.compiler_version() != RECIPE_V4_CBSEM_COMPILER_VERSION
        || receipt.capability_cell() != &RecipeV4CompilerTarget::CbsemPlanV2.capability_cell()
        || receipt.model_id() != plan.model_id()
        || receipt.model_scientific_sha256() != plan.scientific_hash()
        || reconstructed_scientific_sha256 != plan.scientific_hash()
        || receipt.dataset_fingerprint().trim().is_empty()
        || receipt.plan_sha256() != qpls_core::sha256_serialized(artifact.plan())
        || !hashes_are_canonical
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidArtifact(
            "CB-SEM plan and immutable compilation receipt are not self-consistent".into(),
        ));
    }
    Ok(plan)
}

fn validated_fit_receipt(
    artifact: &CompiledAnalysisRecipeV4,
    plan: &CompiledCbsemPlanV2,
    result: &CbsemCompiledMomentResultV2,
) -> Result<CbsemExactLrtFitReceiptV1, CbsemExactNestedLrtErrorV1> {
    let receipt = artifact.receipt();
    if result.compiler_analytical_identity_sha256 != receipt.analytical_identity_sha256()
        || result.plan_sha256 != receipt.plan_sha256()
        || result.model_scientific_sha256 != receipt.model_scientific_sha256()
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "compiled moment result identity does not match its artifact receipt".into(),
        ));
    }
    let rows = parameter_rows(plan)?;
    let dimensions = free_groups(&rows)?.values().collect::<BTreeSet<_>>().len();
    let mean_structure = rows.iter().any(|row| {
        matches!(
            row.role,
            CompiledCbsemParameterRoleV2::Intercept | CompiledCbsemParameterRoleV2::Mean
        )
    });
    let data = validated_data_authority(artifact, plan, result, mean_structure)?;
    let analysis = &result.analysis;
    let observed = data.variable_ids.len();
    let expected_model_type = if plan.regressions().is_empty() {
        "cfa"
    } else {
        "sem"
    };
    let expected_estimator_method = if mean_structure {
        CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4
    } else {
        CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
    };
    let observed_moments = observed
        .checked_mul(observed + 1)
        .and_then(|value| value.checked_div(2))
        .and_then(|value| value.checked_add(if mean_structure { observed } else { 0 }))
        .ok_or_else(|| {
            CbsemExactNestedLrtErrorV1::InvalidPlan("observed-moment count overflow".into())
        })?;
    let expected_df = i64::try_from(observed_moments)
        .ok()
        .and_then(|moments| {
            i64::try_from(dimensions)
                .ok()
                .map(|dimensions| moments - dimensions)
        })
        .ok_or_else(|| {
            CbsemExactNestedLrtErrorV1::InvalidPlan("dimension count exceeds i64".into())
        })?;
    let expected_chi_square = (data.used_sample_size as f64 * analysis.objective).max(0.0);
    if analysis.method_version != expected_estimator_method
        || analysis.fit.method_version != CBSEM_FIT_METHOD_VERSION
        || analysis.model_type != expected_model_type
        || analysis.estimator != "ml"
        || analysis.input != "raw"
        || !analysis.converged
        || analysis.iterations == 0
        || analysis.mean_structure != mean_structure
        || analysis.sample_size != data.used_sample_size
        || !analysis.objective.is_finite()
        || analysis.objective < 0.0
        || !analysis.gradient_norm.is_finite()
        || analysis.gradient_norm < 0.0
        || !analysis.fit.chi_square.is_finite()
        || analysis.fit.chi_square.to_bits() != expected_chi_square.to_bits()
        || analysis.fit.degrees_of_freedom != expected_df
        || expected_df < 0
        || analysis.bootstrap.is_some()
        || analysis.bootstrap_v2.is_some()
        || analysis.exact_case_bootstrap.is_some()
        || analysis.multigroup.is_some()
        || !analysis.modification_indices.is_empty()
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit("plan identity, ordinary-ML method, convergence/admissibility, finite fit, or independently derived df differs".into()));
    }
    validate_analysis_numeric_contract(plan, result, &data)?;
    Ok(CbsemExactLrtFitReceiptV1 {
        compiler_analytical_identity_sha256: result.compiler_analytical_identity_sha256.clone(),
        plan_sha256: result.plan_sha256.clone(),
        model_scientific_sha256: result.model_scientific_sha256.clone(),
        data,
        estimator_method_version: analysis.method_version.clone(),
        fit_method_version: analysis.fit.method_version.clone(),
        model_type: analysis.model_type.clone(),
        estimator: analysis.estimator.clone(),
        input: analysis.input.clone(),
        mean_structure: analysis.mean_structure,
        converged: analysis.converged,
        admissible: true,
        iterations: analysis.iterations,
        objective: analysis.objective,
        gradient_norm: analysis.gradient_norm,
        chi_square: analysis.fit.chi_square,
        degrees_of_freedom: analysis.fit.degrees_of_freedom,
    })
}

fn validated_data_authority(
    artifact: &CompiledAnalysisRecipeV4,
    plan: &CompiledCbsemPlanV2,
    result: &CbsemCompiledMomentResultV2,
    mean_structure: bool,
) -> Result<CbsemExactLrtDataAuthorityV1, CbsemExactNestedLrtErrorV1> {
    let CompiledCbsemInputV2::Raw {
        dataset_id,
        missing_data: MissingDataPolicyV4::ListwiseDeletion,
        weight: None,
        cluster_variable: None,
        strata_variable: None,
    } = plan.input()
    else {
        return Err(CbsemExactNestedLrtErrorV1::InvalidPlan(
            "exact LRT requires raw listwise input without weights, clusters, or strata".into(),
        ));
    };
    let (variable_ids, source_columns) = ordered_observed_bindings(plan)?;
    let (expected_schema, expected_method) = if mean_structure {
        (
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3,
            CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
        )
    } else {
        (
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
            CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
        )
    };
    let input = &result.input;
    if result.schema_version != expected_schema
        || result.method_version != expected_method
        || input.kind != CbsemMomentInputKindV2::Raw
        || input.dataset_id != *dataset_id
        || input.dataset_fingerprint != artifact.receipt().dataset_fingerprint()
        || input.declared_sample_size.is_some()
        || input.used_sample_size < 10
        || input
            .used_sample_size
            .checked_add(input.omitted_observations)
            .is_none()
        || input.covariance_denominator != SemCovarianceDenominatorV4::MaximumLikelihoodN
        || input.variable_ids != variable_ids
        || input.source_columns != source_columns
        || input.standard_deviations.is_some()
        || input.missing_data_treatment.is_some()
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "raw-listwise moment schema, method, dataset, variable binding, or N authority is invalid"
                .into(),
        ));
    }
    if result.covariance_ml.len() != variable_ids.len()
        || result.covariance_ml.iter().any(|row| {
            row.len() != variable_ids.len() || row.iter().any(|value| !value.is_finite())
        })
        || validate_symmetric_matrix(&result.covariance_ml).is_err()
        || ensure_strict_positive_definite(&result.covariance_ml).is_err()
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "canonical ML covariance is not finite, square, symmetric, and positive definite"
                .into(),
        ));
    }
    let covariance_hash = covariance_sha256(
        CbsemMomentInputKindV2::Raw,
        input.used_sample_size,
        &variable_ids,
        SemCovarianceDenominatorV4::MaximumLikelihoodN,
        &result.covariance_ml,
    );
    if input.canonical_ml_covariance_sha256 != covariance_hash {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "canonical ML covariance hash does not reproduce the persisted matrix".into(),
        ));
    }
    let observed_values = if mean_structure {
        let observed = ordered_mean_values(&result.observed_means, &source_columns, "observed")?;
        let implied = ordered_mean_values(&result.implied_means, &source_columns, "implied")?;
        let residual = ordered_mean_values(&result.residual_means, &source_columns, "residual")?;
        if observed
            .iter()
            .zip(&implied)
            .zip(&residual)
            .any(|((observed, implied), residual)| {
                (*observed - *implied).to_bits() != residual.to_bits()
            })
        {
            return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
                "residual means do not bit-exactly reproduce observed minus implied means".into(),
            ));
        }
        Some(observed)
    } else {
        if !result.observed_means.is_empty()
            || !result.implied_means.is_empty()
            || !result.residual_means.is_empty()
        {
            return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
                "covariance-only exact result contains injected mean rows".into(),
            ));
        }
        None
    };
    let means_hash = observed_values
        .as_ref()
        .map(|values| observed_means_sha256(input.used_sample_size, &variable_ids, values));
    if input.canonical_observed_means_sha256 != means_hash {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "canonical observed-means hash does not reproduce the persisted means".into(),
        ));
    }
    Ok(CbsemExactLrtDataAuthorityV1 {
        dataset_id: input.dataset_id.clone(),
        dataset_fingerprint: input.dataset_fingerprint.clone(),
        variable_ids,
        source_columns,
        used_sample_size: input.used_sample_size,
        omitted_observations: input.omitted_observations,
        input_kind: input.kind,
        missing_data: MissingDataPolicyV4::ListwiseDeletion,
        covariance_denominator: input.covariance_denominator,
        moment_schema_version: result.schema_version,
        moment_method_version: result.method_version.clone(),
        canonical_ml_covariance_sha256: covariance_hash,
        canonical_observed_means_sha256: means_hash,
    })
}

fn ordered_observed_bindings(
    plan: &CompiledCbsemPlanV2,
) -> Result<(Vec<String>, Vec<String>), CbsemExactNestedLrtErrorV1> {
    let mut bindings = BTreeMap::<String, String>::new();
    let mut sources = BTreeSet::new();
    for variable in plan.variables() {
        if let SemVariableV4::Observed {
            id, source_column, ..
        } = variable
        {
            if bindings.insert(id.clone(), source_column.clone()).is_some()
                || !sources.insert(source_column.clone())
            {
                return Err(CbsemExactNestedLrtErrorV1::InvalidPlan(
                    "observed ids and source columns must be unique".into(),
                ));
            }
        }
    }
    if bindings.is_empty() {
        return Err(CbsemExactNestedLrtErrorV1::InvalidPlan(
            "exact LRT plan has no observed variables".into(),
        ));
    }
    Ok(bindings.into_iter().unzip())
}

fn ordered_mean_values(
    cells: &[crate::CbsemMeanCellV4],
    expected_names: &[String],
    family: &str,
) -> Result<Vec<f64>, CbsemExactNestedLrtErrorV1> {
    if cells.len() != expected_names.len()
        || cells
            .iter()
            .zip(expected_names)
            .any(|(cell, name)| cell.variable != *name || !cell.value.is_finite())
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(format!(
            "{family} means are nonfinite, missing, duplicated, or out of canonical order"
        )));
    }
    Ok(cells.iter().map(|cell| cell.value).collect())
}

fn validate_analysis_numeric_contract(
    plan: &CompiledCbsemPlanV2,
    result: &CbsemCompiledMomentResultV2,
    data: &CbsemExactLrtDataAuthorityV1,
) -> Result<(), CbsemExactNestedLrtErrorV1> {
    let analysis = &result.analysis;
    let finite_optional = |value: Option<f64>| value.is_none_or(f64::is_finite);
    if [
        Some(analysis.fit.srmr),
        Some(analysis.fit.aic),
        Some(analysis.fit.bic),
        Some(analysis.fit.baseline_chi_square),
        analysis.fit.p_value,
        analysis.fit.cfi,
        analysis.fit.tli,
        analysis.fit.rmsea,
        analysis.fit.rmsea_ci_lower,
        analysis.fit.rmsea_ci_upper,
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.is_finite())
        || analysis
            .standardized
            .iter()
            .any(|row| !row.std_lv.is_finite() || !row.std_all.is_finite())
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "analysis contains a nonfinite standardized or fit value".into(),
        ));
    }
    let plan_rows = plan
        .parameters()
        .iter()
        .map(|row| (row.id(), row))
        .collect::<BTreeMap<_, _>>();
    let stable_ids = result
        .parameter_ids
        .values()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if result.parameter_ids.len() != analysis.parameters.len()
        || stable_ids.len() != result.parameter_ids.len()
        || stable_ids != plan_rows.keys().copied().collect::<BTreeSet<_>>()
        || analysis.parameters.iter().any(|parameter| {
            let Some(stable_id) = result.parameter_ids.get(&parameter.name) else {
                return true;
            };
            let Some(row) = plan_rows.get(stable_id.as_str()) else {
                return true;
            };
            let expected_fixed = matches!(
                row.specification(),
                CompiledCbsemParameterStatusV2::Fixed { .. }
            );
            parameter.fixed != expected_fixed
                || !parameter.estimate.is_finite()
                || !finite_optional(parameter.standard_error)
                || !finite_optional(parameter.z_statistic)
                || !finite_optional(parameter.p_value_two_sided)
        })
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "stable parameter identities, fixed status, or numeric rows differ from the plan"
                .into(),
        ));
    }
    validate_parameter_estimate_contract(plan, result)?;
    let implied = matrix_from_cells(
        &analysis.implied_covariance,
        &data.source_columns,
        "implied covariance",
    )?;
    let residual = matrix_from_cells(
        &analysis.residual_covariance,
        &data.source_columns,
        "residual covariance",
    )?;
    matrix_from_cells(
        &analysis.residual_correlation,
        &data.source_columns,
        "residual correlation",
    )?;
    if validate_symmetric_matrix(&implied).is_err()
        || ensure_strict_positive_definite(&implied).is_err()
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "engine implied covariance is not symmetric positive definite".into(),
        ));
    }
    if result
        .covariance_ml
        .iter()
        .zip(&implied)
        .zip(&residual)
        .any(|((sample_row, implied_row), residual_row)| {
            sample_row.iter().zip(implied_row).zip(residual_row).any(
                |((sample, implied), residual)| {
                    (*sample - *implied).to_bits() != residual.to_bits()
                },
            )
        })
    {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "residual covariance does not bit-exactly reproduce sample minus implied covariance"
                .into(),
        ));
    }
    let covariance_objective =
        cbsem_exact_covariance_ml_discrepancy_v1(&result.covariance_ml, &implied).map_err(
            |error| {
                CbsemExactNestedLrtErrorV1::InvalidFit(format!(
                    "exact covariance ML discrepancy could not be reproduced: {error}"
                ))
            },
        )?;
    let mean_objective = if analysis.mean_structure {
        let observed =
            ordered_mean_values(&result.observed_means, &data.source_columns, "observed")?;
        let implied_means =
            ordered_mean_values(&result.implied_means, &data.source_columns, "implied")?;
        cbsem_exact_mean_structure_ml_discrepancy_v1(&observed, &implied_means, &implied).map_err(
            |error| {
                CbsemExactNestedLrtErrorV1::InvalidFit(format!(
                    "exact mean-structure ML discrepancy could not be reproduced: {error}"
                ))
            },
        )?
    } else {
        0.0
    };
    let expected_objective = covariance_objective + mean_objective;
    if analysis.objective.to_bits() != expected_objective.to_bits() {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(
            "analysis objective does not bit-exactly reproduce the persisted ML moments".into(),
        ));
    }
    Ok(())
}

fn validate_parameter_estimate_contract(
    plan: &CompiledCbsemPlanV2,
    result: &CbsemCompiledMomentResultV2,
) -> Result<(), CbsemExactNestedLrtErrorV1> {
    let estimates = result
        .analysis
        .parameters
        .iter()
        .map(|parameter| {
            let stable_id = result.parameter_ids[&parameter.name].as_str();
            (stable_id, parameter.estimate)
        })
        .collect::<BTreeMap<_, _>>();
    let mut equality_estimates = BTreeMap::<&str, u64>::new();
    for row in plan.parameters() {
        let estimate = estimates[row.id()];
        match row.specification() {
            CompiledCbsemParameterStatusV2::Fixed { value } => {
                if estimate.to_bits() != value.to_bits() {
                    return Err(CbsemExactNestedLrtErrorV1::InvalidFit(format!(
                        "fixed parameter {} estimate differs from its declared fixed value",
                        row.id()
                    )));
                }
            }
            CompiledCbsemParameterStatusV2::Free {
                lower,
                upper,
                equality_label,
                ..
            } => {
                let effective_lower = if row.role() == CompiledCbsemParameterRoleV2::Variance {
                    Some(lower.unwrap_or(0.0).max(0.0))
                } else {
                    *lower
                };
                if effective_lower.is_some_and(|bound| estimate <= bound)
                    || upper.is_some_and(|bound| estimate >= bound)
                {
                    return Err(CbsemExactNestedLrtErrorV1::InvalidFit(format!(
                        "free parameter {} estimate is outside its open domain",
                        row.id()
                    )));
                }
                if let Some(label) = equality_label.as_deref() {
                    let label = label.trim();
                    if equality_estimates
                        .insert(label, estimate.to_bits())
                        .is_some_and(|previous| previous != estimate.to_bits())
                    {
                        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(format!(
                            "equality group {label} contains bit-distinct estimates"
                        )));
                    }
                }
            }
            CompiledCbsemParameterStatusV2::Derived { .. } => {
                return Err(CbsemExactNestedLrtErrorV1::InvalidPlan(
                    "derived parameters are outside the exact LRT slice".into(),
                ));
            }
        }
    }
    Ok(())
}

fn matrix_from_cells(
    cells: &[crate::CbsemMatrixCell],
    names: &[String],
    family: &str,
) -> Result<Vec<Vec<f64>>, CbsemExactNestedLrtErrorV1> {
    let expected_len = names.len().checked_mul(names.len()).ok_or_else(|| {
        CbsemExactNestedLrtErrorV1::InvalidPlan("matrix cell count overflow".into())
    })?;
    if cells.len() != expected_len {
        return Err(CbsemExactNestedLrtErrorV1::InvalidFit(format!(
            "{family} does not contain one canonical row-major cell per variable pair"
        )));
    }
    let mut matrix = vec![vec![0.0; names.len()]; names.len()];
    for (ordinal, cell) in cells.iter().enumerate() {
        let row = ordinal / names.len();
        let column = ordinal % names.len();
        if cell.row != names[row] || cell.column != names[column] || !cell.value.is_finite() {
            return Err(CbsemExactNestedLrtErrorV1::InvalidFit(format!(
                "{family} is nonfinite, duplicated, or out of canonical order"
            )));
        }
        matrix[row][column] = cell.value;
    }
    Ok(matrix)
}

fn same_optional_bits(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        (None, None) => true,
        _ => false,
    }
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn free(label: Option<&str>) -> Value {
        json!({
            "status": "free",
            "start": 0.25,
            "lower": -2.0,
            "upper": 2.0,
            "equality_label": label,
        })
    }

    fn fixed(value: f64) -> Value {
        json!({ "status": "fixed", "value": value })
    }

    fn plan(statuses: [Value; 3]) -> CompiledCbsemPlanV2 {
        let parameters = statuses
            .into_iter()
            .enumerate()
            .map(|(index, specification)| {
                json!({
                    "id": format!("p{}", index + 1),
                    "label": format!("Parameter {}", index + 1),
                    "target": {
                        "kind": "regression",
                        "source": "x",
                        "target": "y",
                    },
                    "specification": specification,
                    "group_overrides": [],
                })
            })
            .collect::<Vec<_>>();
        let provisional: CompiledCbsemPlanV2 = serde_json::from_value(json!({
            "schema_version": 2,
            "sem_model_schema_version": 4,
            "model_id": "model",
            "model_name": "Model",
            "scientific_hash": "a".repeat(64),
            "input": {
                "kind": "raw",
                "dataset_id": "dataset",
                "missing_data": "listwise_deletion",
                "weight": null,
                "cluster_variable": null,
                "strata_variable": null,
            },
            "variables": [
                { "kind": "observed", "id": "x", "label": "X", "source_column": "x", "scale": "continuous", "role": "structural", "categories": [], "value_labels": {}, "missing_markers": [], "transformation_lineage": [] },
                { "kind": "observed", "id": "y", "label": "Y", "source_column": "y", "scale": "continuous", "role": "structural", "categories": [], "value_labels": {}, "missing_markers": [], "transformation_lineage": [] }
            ],
            "loadings": [],
            "causal_measurements": [],
            "regressions": [],
            "covariances": [],
            "parameter_table": parameters,
            "constraints": [],
            "derived_terms": [],
            "group": { "kind": "single_group" },
            "has_feedback": false,
        }))
        .unwrap();
        let scientific_hash = provisional
            .to_scientific_sem_model_v4()
            .scientific_sha256()
            .unwrap();
        let mut encoded = serde_json::to_value(provisional).unwrap();
        encoded["scientific_hash"] = json!(scientific_hash);
        serde_json::from_value(encoded).unwrap()
    }

    fn artifact(plan: CompiledCbsemPlanV2, dataset_fingerprint: &str) -> CompiledAnalysisRecipeV4 {
        let model_id = plan.model_id().to_owned();
        let model_scientific_sha256 = plan.scientific_hash().to_owned();
        let wrapper = CompiledRecipePlanV4::CbsemPlanV2 { plan };
        let plan_sha256 = qpls_core::sha256_serialized(&wrapper);
        serde_json::from_value(json!({
            "plan": wrapper,
            "receipt": {
                "schema_version": RECIPE_V4_COMPILATION_RECEIPT_SCHEMA_VERSION,
                "recipe_id": "00000000-0000-0000-0000-000000000001",
                "recipe_document_sha256": "1".repeat(64),
                "recipe_analytical_sha256": "2".repeat(64),
                "model_id": model_id,
                "model_document_sha256": "3".repeat(64),
                "model_scientific_sha256": model_scientific_sha256,
                "dataset_fingerprint": dataset_fingerprint,
                "compiler_target": "cbsem_plan_v2",
                "compiler_version": RECIPE_V4_CBSEM_COMPILER_VERSION,
                "capability_cell": {
                    "registry_schema_version": 2,
                    "capability_id": "smartpls.cbsem",
                    "cell_id": "qpls3.cbsem.ml",
                    "capability_version": "cbsem_ml_v1",
                },
                "plan_sha256": plan_sha256,
                "analytical_identity_sha256": "4".repeat(64),
            },
        }))
        .unwrap()
    }

    fn matrix_cells(names: &[String], matrix: &[Vec<f64>]) -> Vec<crate::CbsemMatrixCell> {
        names
            .iter()
            .enumerate()
            .flat_map(|(row, row_name)| {
                names
                    .iter()
                    .enumerate()
                    .map(move |(column, column_name)| crate::CbsemMatrixCell {
                        row: row_name.clone(),
                        column: column_name.clone(),
                        value: matrix[row][column],
                    })
            })
            .collect()
    }

    fn result(
        artifact: &CompiledAnalysisRecipeV4,
        implied_scale: f64,
        degrees_of_freedom: i64,
    ) -> CbsemCompiledMomentResultV2 {
        result_for_covariance(
            artifact,
            vec![vec![1.0, 0.2], vec![0.2, 1.0]],
            implied_scale,
            degrees_of_freedom,
        )
    }

    fn result_for_covariance(
        artifact: &CompiledAnalysisRecipeV4,
        covariance_ml: Vec<Vec<f64>>,
        implied_scale: f64,
        degrees_of_freedom: i64,
    ) -> CbsemCompiledMomentResultV2 {
        let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
            unreachable!()
        };
        let (variable_ids, source_columns) = ordered_observed_bindings(plan).unwrap();
        let implied_covariance = covariance_ml
            .iter()
            .map(|row| row.iter().map(|value| value * implied_scale).collect())
            .collect::<Vec<Vec<f64>>>();
        let residual_covariance = covariance_ml
            .iter()
            .zip(&implied_covariance)
            .map(|(sample, implied)| {
                sample
                    .iter()
                    .zip(implied)
                    .map(|(sample, implied)| sample - implied)
                    .collect()
            })
            .collect::<Vec<Vec<f64>>>();
        let residual_correlation = residual_covariance.clone();
        let used_sample_size = 100;
        let canonical_ml_covariance_sha256 = covariance_sha256(
            CbsemMomentInputKindV2::Raw,
            used_sample_size,
            &variable_ids,
            SemCovarianceDenominatorV4::MaximumLikelihoodN,
            &covariance_ml,
        );
        let mut parameter_ids = BTreeMap::new();
        let parameters = plan
            .parameters()
            .iter()
            .enumerate()
            .map(|(ordinal, row)| {
                let name = format!("parameter_{}", ordinal + 1);
                parameter_ids.insert(name.clone(), row.id().to_owned());
                let (estimate, fixed) = match row.specification() {
                    CompiledCbsemParameterStatusV2::Fixed { value } => (*value, true),
                    _ => (0.25, false),
                };
                crate::CbsemParameter {
                    name,
                    kind: "regression".into(),
                    lhs: "y".into(),
                    rhs: "x".into(),
                    estimate,
                    standard_error: (!fixed).then_some(0.1),
                    z_statistic: (!fixed).then_some(2.5),
                    p_value_two_sided: (!fixed).then_some(0.012),
                    fixed,
                    warning: None,
                }
            })
            .collect();
        let objective =
            cbsem_exact_covariance_ml_discrepancy_v1(&covariance_ml, &implied_covariance).unwrap();
        let chi_square = (used_sample_size as f64 * objective).max(0.0);
        CbsemCompiledMomentResultV2 {
            schema_version: CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
            method_version: CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3.into(),
            compiler_analytical_identity_sha256: artifact
                .receipt()
                .analytical_identity_sha256()
                .into(),
            plan_sha256: artifact.receipt().plan_sha256().into(),
            model_scientific_sha256: artifact.receipt().model_scientific_sha256().into(),
            input: crate::CbsemMomentInputProvenanceV2 {
                kind: CbsemMomentInputKindV2::Raw,
                dataset_id: plan.input().dataset_id().into(),
                dataset_fingerprint: artifact.receipt().dataset_fingerprint().into(),
                declared_sample_size: None,
                used_sample_size,
                omitted_observations: 2,
                covariance_denominator: SemCovarianceDenominatorV4::MaximumLikelihoodN,
                variable_ids,
                source_columns: source_columns.clone(),
                standard_deviations: None,
                canonical_ml_covariance_sha256,
                canonical_observed_means_sha256: None,
                missing_data_treatment: None,
            },
            covariance_ml,
            parameter_ids,
            observed_means: Vec::new(),
            implied_means: Vec::new(),
            residual_means: Vec::new(),
            analysis: crate::CbsemAnalysis {
                method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3.into(),
                model_type: "cfa".into(),
                estimator: "ml".into(),
                input: "raw".into(),
                mean_structure: false,
                converged: true,
                iterations: 5,
                objective,
                gradient_norm: 1.0e-8,
                sample_size: used_sample_size,
                parameters,
                standardized: Vec::new(),
                implied_covariance: matrix_cells(&source_columns, &implied_covariance),
                residual_covariance: matrix_cells(&source_columns, &residual_covariance),
                residual_correlation: matrix_cells(&source_columns, &residual_correlation),
                fit: crate::CbsemFitIndices {
                    method_version: CBSEM_FIT_METHOD_VERSION.into(),
                    chi_square,
                    degrees_of_freedom,
                    p_value: (degrees_of_freedom > 0).then_some(0.5),
                    cfi: Some(0.95),
                    tli: (degrees_of_freedom > 0).then_some(0.9),
                    rmsea: Some(0.05),
                    rmsea_interval_attribution: Some(crate::CbsemRmseaIntervalAttributionV1 {
                        method_version: crate::CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1.into(),
                        confidence_level: 0.9,
                    }),
                    rmsea_ci_lower: Some(0.01),
                    rmsea_ci_upper: Some(0.09),
                    srmr: 0.04,
                    aic: chi_square + 6.0,
                    bic: chi_square + 12.0,
                    baseline_chi_square: 20.0,
                    baseline_degrees_of_freedom: 1,
                },
                modification_indices: Vec::new(),
                score_lm: None,
                bootstrap: None,
                bootstrap_v2: None,
                exact_case_bootstrap: None,
                exact_case_bootstrap_studentized: None,
                exact_case_bootstrap_bca: None,
                multigroup: None,
                diagnostics: Vec::new(),
                warnings: Vec::new(),
            },
        }
    }

    #[test]
    fn coarser_equality_partition_proves_dimension_reduction_and_lrt_tail() {
        let unrestricted = plan([free(None), free(None), free(None)]);
        let restricted = plan([free(Some("equal")), free(Some("equal")), free(None)]);
        let proof = prove_cbsem_exact_nesting_v1(&unrestricted, &restricted).unwrap();
        assert_eq!(proof.unrestricted_free_dimensions, 3);
        assert_eq!(proof.restricted_free_dimensions, 2);
        assert_eq!(proof.dimension_reduction, 1);

        let unrestricted_artifact = artifact(unrestricted, "dataset-fingerprint");
        let restricted_artifact = artifact(restricted, "dataset-fingerprint");
        let unrestricted_result = result(&unrestricted_artifact, 1.0, 0);
        let restricted_result = result(&restricted_artifact, 1.224_599_070_303_46, 1);
        let result = compare_cbsem_exact_nested_ml_v1(
            &unrestricted_artifact,
            &unrestricted_result,
            &restricted_artifact,
            &restricted_result,
        )
        .unwrap();
        assert_eq!(result.delta_degrees_of_freedom, 1);
        assert!((result.p_value - 0.05).abs() < 1.0e-12);
    }

    #[test]
    fn equality_partition_rejects_mixed_parameter_families() {
        let unrestricted = plan([free(None), free(None), free(None)]);
        let mut encoded =
            serde_json::to_value(plan([free(Some("mixed")), free(Some("mixed")), free(None)]))
                .unwrap();
        encoded["parameter_table"][1]["target"] = json!({
            "kind": "variance",
            "endpoint": { "kind": "variable", "id": "x" },
        });
        let restricted = serde_json::from_value(encoded).unwrap();

        assert!(matches!(
            prove_cbsem_exact_nesting_v1(&unrestricted, &restricted),
            Err(CbsemExactNestedLrtErrorV1::InvalidPlan(message))
                if message.contains("different parameter families")
        ));
    }

    #[test]
    fn survival_tail_remains_finite_and_positive_for_large_statistic() {
        let unrestricted = artifact(
            plan([free(None), free(None), free(None)]),
            "dataset-fingerprint",
        );
        let restricted = artifact(
            plan([fixed(0.5), free(None), free(None)]),
            "dataset-fingerprint",
        );
        let unrestricted_result = result(&unrestricted, 1.0, 0);
        let restricted_result = result(&restricted, 3.314_445_823_668_68, 1);
        let result = compare_cbsem_exact_nested_ml_v1(
            &unrestricted,
            &unrestricted_result,
            &restricted,
            &restricted_result,
        )
        .unwrap();

        assert!(result.p_value.is_finite());
        assert!(result.p_value > 0.0);
        assert!(result.p_value < 1.0e-20);
    }

    #[test]
    fn interior_fix_is_nested_but_release_split_boundary_and_surface_changes_fail() {
        let unrestricted = plan([free(None), free(None), free(None)]);
        let interior = plan([fixed(0.5), free(None), free(None)]);
        assert_eq!(
            prove_cbsem_exact_nesting_v1(&unrestricted, &interior)
                .unwrap()
                .dimension_reduction,
            1
        );

        let boundary = plan([fixed(2.0), free(None), free(None)]);
        assert!(matches!(
            prove_cbsem_exact_nesting_v1(&unrestricted, &boundary),
            Err(CbsemExactNestedLrtErrorV1::NotNested(_))
        ));

        let equal_unrestricted = plan([free(Some("equal")), free(Some("equal")), free(None)]);
        let split = plan([free(None), free(None), free(None)]);
        assert!(matches!(
            prove_cbsem_exact_nesting_v1(&equal_unrestricted, &split),
            Err(CbsemExactNestedLrtErrorV1::NotNested(_))
        ));

        let mut different_input = interior.clone();
        let mut encoded = serde_json::to_value(&different_input).unwrap();
        encoded["input"]["dataset_id"] = json!("other");
        different_input = serde_json::from_value(encoded).unwrap();
        assert!(matches!(
            prove_cbsem_exact_nesting_v1(&unrestricted, &different_input),
            Err(CbsemExactNestedLrtErrorV1::NotNested(_))
        ));
    }

    #[test]
    fn artifact_result_identity_method_estimator_and_fit_splices_fail_closed() {
        let unrestricted = artifact(
            plan([free(None), free(None), free(None)]),
            "dataset-fingerprint",
        );
        let restricted = artifact(
            plan([fixed(0.5), free(None), free(None)]),
            "dataset-fingerprint",
        );
        let unrestricted_result = result(&unrestricted, 1.0, 0);
        let restricted_result = result(&restricted, 1.1, 1);

        assert!(matches!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &restricted_result,
                &restricted,
                &unrestricted_result,
            ),
            Err(CbsemExactNestedLrtErrorV1::InvalidFit(_))
        ));

        let mut artifact_json = serde_json::to_value(&restricted).unwrap();
        artifact_json["plan"]["plan"]["model_name"] = json!("spliced plan");
        let spliced_artifact = serde_json::from_value(artifact_json).unwrap();
        assert!(matches!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &spliced_artifact,
                &restricted_result,
            ),
            Err(CbsemExactNestedLrtErrorV1::InvalidArtifact(_))
        ));

        let mut scientific_json = serde_json::to_value(&restricted).unwrap();
        scientific_json["plan"]["plan"]["scientific_hash"] = json!("f".repeat(64));
        scientific_json["receipt"]["model_scientific_sha256"] = json!("f".repeat(64));
        let wrapper: CompiledRecipePlanV4 =
            serde_json::from_value(scientific_json["plan"].clone()).unwrap();
        scientific_json["receipt"]["plan_sha256"] = json!(qpls_core::sha256_serialized(&wrapper));
        let scientific_tamper = serde_json::from_value(scientific_json).unwrap();
        assert!(matches!(
            validated_cbsem_artifact_plan(&scientific_tamper),
            Err(CbsemExactNestedLrtErrorV1::InvalidArtifact(_))
        ));

        for mutation in [
            "method",
            "estimator",
            "chi_objective",
            "objective_moment_splice",
            "residual_moment_splice",
            "df",
            "convergence",
            "fixed_estimate",
            "free_domain",
        ] {
            let mut tampered = restricted_result.clone();
            match mutation {
                "method" => tampered.method_version = "wrong_moment_method".into(),
                "estimator" => tampered.analysis.estimator = "gls".into(),
                "chi_objective" => {
                    tampered.analysis.fit.chi_square =
                        f64::from_bits(tampered.analysis.fit.chi_square.to_bits() + 1)
                }
                "objective_moment_splice" => {
                    tampered.analysis.objective =
                        f64::from_bits(tampered.analysis.objective.to_bits() + 1);
                    tampered.analysis.fit.chi_square = (tampered.analysis.sample_size as f64
                        * tampered.analysis.objective)
                        .max(0.0);
                }
                "residual_moment_splice" => {
                    tampered.analysis.residual_covariance[0].value =
                        f64::from_bits(tampered.analysis.residual_covariance[0].value.to_bits() + 1)
                }
                "df" => tampered.analysis.fit.degrees_of_freedom = 2,
                "convergence" => tampered.analysis.converged = false,
                "fixed_estimate" => {
                    tampered.analysis.parameters[0].estimate =
                        f64::from_bits(tampered.analysis.parameters[0].estimate.to_bits() + 1)
                }
                "free_domain" => tampered.analysis.parameters[1].estimate = 2.0,
                _ => unreachable!(),
            }
            assert!(matches!(
                compare_cbsem_exact_nested_ml_v1(
                    &unrestricted,
                    &unrestricted_result,
                    &restricted,
                    &tampered,
                ),
                Err(CbsemExactNestedLrtErrorV1::InvalidFit(_))
            ));
        }

        let mut inadmissible = restricted_result.clone();
        inadmissible.analysis.implied_covariance[0].value = -1.0;
        assert!(matches!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &restricted,
                &inadmissible,
            ),
            Err(CbsemExactNestedLrtErrorV1::InvalidFit(_))
        ));

        let alternate_run =
            result_for_covariance(&restricted, vec![vec![1.4, 0.05], vec![0.05, 0.8]], 1.1, 1);
        let mut coherent_analysis_splice = restricted_result.clone();
        coherent_analysis_splice.analysis = alternate_run.analysis;
        assert!(matches!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &restricted,
                &coherent_analysis_splice,
            ),
            Err(CbsemExactNestedLrtErrorV1::InvalidFit(_))
        ));
    }

    #[test]
    fn equality_group_estimates_are_bit_bound() {
        let unrestricted = artifact(
            plan([free(None), free(None), free(None)]),
            "dataset-fingerprint",
        );
        let restricted = artifact(
            plan([free(Some("equal")), free(Some("equal")), free(None)]),
            "dataset-fingerprint",
        );
        let unrestricted_result = result(&unrestricted, 1.0, 0);
        let mut restricted_result = result(&restricted, 1.1, 1);
        restricted_result.analysis.parameters[1].estimate =
            f64::from_bits(restricted_result.analysis.parameters[1].estimate.to_bits() + 1);
        assert!(matches!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &restricted,
                &restricted_result,
            ),
            Err(CbsemExactNestedLrtErrorV1::InvalidFit(_))
        ));
    }

    #[test]
    fn source_fingerprint_n_and_moment_authority_splices_fail_closed() {
        let unrestricted = artifact(
            plan([free(None), free(None), free(None)]),
            "dataset-fingerprint",
        );
        let restricted = artifact(
            plan([fixed(0.5), free(None), free(None)]),
            "dataset-fingerprint",
        );
        let unrestricted_result = result(&unrestricted, 1.0, 0);
        let restricted_result = result(&restricted, 1.1, 1);

        let mut source_order = restricted_result.clone();
        source_order.input.source_columns.swap(0, 1);
        assert!(matches!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &restricted,
                &source_order,
            ),
            Err(CbsemExactNestedLrtErrorV1::InvalidFit(_))
        ));

        let other_fingerprint_artifact = artifact(
            plan([fixed(0.5), free(None), free(None)]),
            "other-dataset-fingerprint",
        );
        let other_fingerprint_result = result(&other_fingerprint_artifact, 1.1, 1);
        assert_eq!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &other_fingerprint_artifact,
                &other_fingerprint_result,
            ),
            Err(CbsemExactNestedLrtErrorV1::DifferentDataAuthority)
        );

        let mut other_n = restricted_result.clone();
        other_n.input.used_sample_size = 101;
        other_n.analysis.sample_size = 101;
        other_n.analysis.fit.chi_square = (101.0 * other_n.analysis.objective).max(0.0);
        other_n.input.canonical_ml_covariance_sha256 = covariance_sha256(
            CbsemMomentInputKindV2::Raw,
            101,
            &other_n.input.variable_ids,
            SemCovarianceDenominatorV4::MaximumLikelihoodN,
            &other_n.covariance_ml,
        );
        assert_eq!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &restricted,
                &other_n,
            ),
            Err(CbsemExactNestedLrtErrorV1::DifferentDataAuthority)
        );

        let mut other_omitted_n = restricted_result.clone();
        other_omitted_n.input.omitted_observations += 1;
        assert_eq!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &restricted,
                &other_omitted_n,
            ),
            Err(CbsemExactNestedLrtErrorV1::DifferentDataAuthority)
        );

        let other_moments =
            result_for_covariance(&restricted, vec![vec![1.0, 0.1], vec![0.1, 1.0]], 1.1, 1);
        assert_eq!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &restricted,
                &other_moments,
            ),
            Err(CbsemExactNestedLrtErrorV1::DifferentDataAuthority)
        );

        let mut forged_hash = restricted_result.clone();
        forged_hash.input.canonical_ml_covariance_sha256 = "f".repeat(64);
        assert!(matches!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &restricted,
                &forged_hash,
            ),
            Err(CbsemExactNestedLrtErrorV1::InvalidFit(_))
        ));
    }

    #[test]
    fn negative_difference_remains_fail_closed() {
        let unrestricted = artifact(
            plan([free(None), free(None), free(None)]),
            "dataset-fingerprint",
        );
        let restricted = artifact(
            plan([fixed(0.5), free(None), free(None)]),
            "dataset-fingerprint",
        );
        let unrestricted_result = result(&unrestricted, 1.3, 0);
        let restricted_result = result(&restricted, 1.2, 1);
        assert!(matches!(
            compare_cbsem_exact_nested_ml_v1(
                &unrestricted,
                &unrestricted_result,
                &restricted,
                &restricted_result,
            ),
            Err(CbsemExactNestedLrtErrorV1::NegativeChiSquareDifference { .. })
        ));
    }
}
