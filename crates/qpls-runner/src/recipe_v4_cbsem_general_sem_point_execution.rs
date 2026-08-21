use qpls_core::{
    AnalysisRecipeV4, CanonicalCbsemEndpointV1, CanonicalCbsemFitResultV1,
    CanonicalCbsemParameterResultV1, CanonicalCbsemParameterRoleV1, CanonicalCbsemParameterStateV1,
    CanonicalCbsemParameterTargetV1, CanonicalGeneralSemIntervalV1,
    CanonicalGeneralSemResultTraceV1, CanonicalGeneralSemResultsV1,
    CanonicalIdentificationDiagnosticV1, CanonicalIdentificationScopeV1,
    CanonicalIdentificationStatusV1, CompiledAnalysisRecipeV4, CompiledCbsemExecutionDispositionV3,
    CompiledCbsemFactorScaleRestrictionV3, CompiledCbsemIdentificationStatusV3,
    CompiledCbsemParameterStatusV2, CompiledCbsemPlanV3, CompiledCbsemStructuralFormV3,
    CompiledRecipePlanV4, GeneralSemInferenceV1, RecipeV4CompilationError,
    RecipeV4CompilationReceipt, RecipeV4CompilerTarget, SemEndpointV4, SemModelV4,
    SemParameterTargetV4, SemVariableV4, cbsem_general_sem_ml_capability_cell_v1,
    compile_analysis_recipe_v4, validate_compiled_analysis_recipe_v4,
};
use qpls_data::Dataset;
use qpls_estimation::{
    CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1, CBSEM_FIT_METHOD_VERSION,
    CbsemCompiledMomentResultV2, CbsemParameter, CbsemStandardizedParameter,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{RecipeV4CbsemExecutionError, RunnerProgress, run_compiled_cbsem_recipe_v4};

pub(crate) const RECIPE_V4_CBSEM_GENERAL_SEM_POINT_EXECUTION_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub(crate) const RECIPE_V4_CBSEM_GENERAL_SEM_POINT_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_recipe_v4_cbsem_plan_v3_point_execution_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipeV4CbsemGeneralSemPointExecutionProvenanceV1 {
    adapter_version: String,
    v3_compilation_receipt: RecipeV4CompilationReceipt,
    v2_kernel_compilation_receipt: RecipeV4CompilationReceipt,
    dataset_id: String,
    dataset_fingerprint: String,
    kernel_adapter_version: String,
    estimator_method_version: String,
    moment_input_method_version: String,
    fit_method_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rmsea_interval_method_version: Option<String>,
    kernel_diagnostics: Vec<String>,
    kernel_warnings: Vec<String>,
}

impl RecipeV4CbsemGeneralSemPointExecutionProvenanceV1 {
    pub(crate) fn adapter_version(&self) -> &str {
        &self.adapter_version
    }

    pub(crate) fn v3_compilation_receipt(&self) -> &RecipeV4CompilationReceipt {
        &self.v3_compilation_receipt
    }

    pub(crate) fn v2_kernel_compilation_receipt(&self) -> &RecipeV4CompilationReceipt {
        &self.v2_kernel_compilation_receipt
    }

    pub(crate) fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    pub(crate) fn dataset_fingerprint(&self) -> &str {
        &self.dataset_fingerprint
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipeV4CbsemGeneralSemPointExecutionResultV1 {
    schema_version: u32,
    provenance: RecipeV4CbsemGeneralSemPointExecutionProvenanceV1,
    general_sem_results: CanonicalGeneralSemResultsV1,
}

impl RecipeV4CbsemGeneralSemPointExecutionResultV1 {
    pub(crate) fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub(crate) fn provenance(&self) -> &RecipeV4CbsemGeneralSemPointExecutionProvenanceV1 {
        &self.provenance
    }

    pub(crate) fn general_sem_results(&self) -> &CanonicalGeneralSemResultsV1 {
        &self.general_sem_results
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RecipeV4CbsemGeneralSemPointExecutionErrorV1 {
    #[error("analysis was cancelled")]
    Cancelled,
    #[error(transparent)]
    Compilation(#[from] RecipeV4CompilationError),
    #[error("CB-SEM General SEM point execution requires CbsemPlanV3 (found {0:?})")]
    CompilerTarget(RecipeV4CompilerTarget),
    #[error("CB-SEM General SEM V3 adapter is point-only")]
    PointOnly,
    #[error("qualified CB-SEM V2 kernel execution failed: {0}")]
    Kernel(RecipeV4CbsemExecutionError),
    #[error("CB-SEM General SEM V3 authority mismatch: {0}")]
    Authority(String),
    #[error("unsupported CB-SEM General SEM V3 parameter {parameter_id}: {reason}")]
    UnsupportedParameter {
        parameter_id: String,
        reason: String,
    },
}

/// Bounded adapter reached only through the Registry-authorized archive job
/// boundary. It cannot select or promote a cell; the existing qualified V2
/// runner remains the sole numerical execution path.
pub(crate) fn run_compiled_cbsem_general_sem_point_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    artifact: &CompiledAnalysisRecipeV4,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<
    RecipeV4CbsemGeneralSemPointExecutionResultV1,
    RecipeV4CbsemGeneralSemPointExecutionErrorV1,
> {
    if should_cancel() {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Cancelled);
    }
    validate_compiled_analysis_recipe_v4(artifact, recipe, Some(resolved_model))?;
    let CompiledRecipePlanV4::CbsemPlanV3 { plan } = artifact.plan() else {
        return Err(
            RecipeV4CbsemGeneralSemPointExecutionErrorV1::CompilerTarget(artifact.plan().target()),
        );
    };
    ensure_point_v3_authority(dataset, recipe, artifact, plan)?;

    let kernel_artifact = compile_analysis_recipe_v4(
        recipe,
        Some(resolved_model),
        RecipeV4CompilerTarget::CbsemPlanV2,
        RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
    )?;
    let CompiledRecipePlanV4::CbsemPlanV2 { plan: kernel_plan } = kernel_artifact.plan() else {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "deterministic V2 kernel compilation returned the wrong plan target".into(),
        ));
    };
    if kernel_plan != plan.base_plan()
        || kernel_plan.deterministic_sha256() != plan.base_plan_sha256()
        || kernel_artifact.receipt().model_scientific_sha256()
            != artifact.receipt().model_scientific_sha256()
        || kernel_artifact.receipt().dataset_fingerprint()
            != artifact.receipt().dataset_fingerprint()
        || kernel_artifact.receipt().recipe_analytical_sha256()
            != artifact.receipt().recipe_analytical_sha256()
    {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "derived V2 kernel artifact differs from the embedded V3 base-plan, model, recipe, or dataset authority"
                .into(),
        ));
    }

    let kernel_result = run_compiled_cbsem_recipe_v4(
        dataset,
        recipe,
        resolved_model,
        &kernel_artifact,
        &should_cancel,
        &progress,
    )
    .map_err(|error| match error {
        RecipeV4CbsemExecutionError::Cancelled => {
            RecipeV4CbsemGeneralSemPointExecutionErrorV1::Cancelled
        }
        other => RecipeV4CbsemGeneralSemPointExecutionErrorV1::Kernel(other),
    })?;
    if should_cancel() {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Cancelled);
    }
    ensure_kernel_result_authority(
        dataset,
        artifact,
        &kernel_artifact,
        plan,
        kernel_result.estimation(),
    )?;

    let trace = CanonicalGeneralSemResultTraceV1 {
        model_id: plan.model_id().into(),
        capability_cell: cbsem_general_sem_ml_capability_cell_v1(),
    };
    let general_sem_results = canonical_point_results_v1(plan, kernel_result.estimation(), &trace)?;
    if should_cancel() {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Cancelled);
    }

    Ok(RecipeV4CbsemGeneralSemPointExecutionResultV1 {
        schema_version: RECIPE_V4_CBSEM_GENERAL_SEM_POINT_EXECUTION_RESULT_SCHEMA_VERSION_V1,
        provenance: RecipeV4CbsemGeneralSemPointExecutionProvenanceV1 {
            adapter_version: RECIPE_V4_CBSEM_GENERAL_SEM_POINT_EXECUTION_ADAPTER_VERSION_V1.into(),
            v3_compilation_receipt: artifact.receipt().clone(),
            v2_kernel_compilation_receipt: kernel_artifact.receipt().clone(),
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            kernel_adapter_version: kernel_result.provenance().adapter_version().into(),
            estimator_method_version: kernel_result.provenance().estimator_method_version().into(),
            moment_input_method_version: kernel_result
                .provenance()
                .moment_input_method_version()
                .into(),
            fit_method_version: kernel_result
                .estimation()
                .analysis
                .fit
                .method_version
                .clone(),
            rmsea_interval_method_version: kernel_result
                .estimation()
                .analysis
                .fit
                .rmsea_interval_attribution
                .as_ref()
                .map(|attribution| attribution.method_version.clone()),
            kernel_diagnostics: kernel_result.estimation().analysis.diagnostics.clone(),
            kernel_warnings: kernel_result.estimation().analysis.warnings.clone(),
        },
        general_sem_results,
    })
}

fn ensure_point_v3_authority(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    artifact: &CompiledAnalysisRecipeV4,
    plan: &CompiledCbsemPlanV3,
) -> Result<(), RecipeV4CbsemGeneralSemPointExecutionErrorV1> {
    let point_cell = cbsem_general_sem_ml_capability_cell_v1();
    if !matches!(
        recipe
            .general_sem_config
            .as_ref()
            .map(|config| config.inference),
        Some(GeneralSemInferenceV1::None)
    ) || artifact.receipt().capability_cell() != &point_cell
        || plan.capability_cells() != [point_cell]
    {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::PointOnly);
    }
    if plan.identification_evidence().structural_form() != CompiledCbsemStructuralFormV3::Recursive
        || plan.identification_evidence().status()
            != CompiledCbsemIdentificationStatusV3::Provisional
        || plan.identification_evidence().execution_disposition()
            != CompiledCbsemExecutionDispositionV3::RequiresSeparateCapabilityValidation
        || !plan
            .identification_evidence()
            .explicit_constraints()
            .is_empty()
    {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "point adapter requires recursive provisional V3 evidence with no explicit SemConstraintV4 objects; fixed/free rows, equality_label groups, and finite row bounds remain executable"
                .into(),
        ));
    }
    if plan.base_plan().input().dataset_id() != dataset.id.to_string()
        || artifact.receipt().dataset_fingerprint() != dataset.fingerprint.0
        || artifact.receipt().model_id() != plan.model_id()
        || artifact.receipt().model_scientific_sha256() != plan.scientific_sha256()
    {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "resident dataset, model, and V3 compilation receipt do not agree".into(),
        ));
    }
    Ok(())
}

fn ensure_kernel_result_authority(
    dataset: &Dataset,
    v3_artifact: &CompiledAnalysisRecipeV4,
    kernel_artifact: &CompiledAnalysisRecipeV4,
    plan: &CompiledCbsemPlanV3,
    estimation: &CbsemCompiledMomentResultV2,
) -> Result<(), RecipeV4CbsemGeneralSemPointExecutionErrorV1> {
    let analysis = &estimation.analysis;
    let expected_model_type = if plan.base_plan().regressions().is_empty() {
        "cfa"
    } else {
        "sem"
    };
    let has_inference = analysis.bootstrap.is_some()
        || analysis.bootstrap_v2.is_some()
        || analysis.exact_case_bootstrap.is_some()
        || analysis.exact_case_bootstrap_studentized.is_some()
        || analysis.exact_case_bootstrap_bca.is_some()
        || analysis.multigroup.is_some();
    if estimation.compiler_analytical_identity_sha256
        != kernel_artifact.receipt().analytical_identity_sha256()
        || estimation.plan_sha256 != kernel_artifact.receipt().plan_sha256()
        || estimation.model_scientific_sha256 != v3_artifact.receipt().model_scientific_sha256()
        || estimation.input.dataset_id != dataset.id.to_string()
        || estimation.input.dataset_fingerprint != dataset.fingerprint.0
        || analysis.model_type != expected_model_type
        || analysis.estimator != "ml"
        || analysis.input != "raw"
        || analysis.mean_structure
        || !analysis.converged
        || has_inference
        || !analysis.modification_indices.is_empty()
        || analysis.fit.method_version != CBSEM_FIT_METHOD_VERSION
    {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "qualified V2 kernel result changed identity, scope, convergence, fit, or point-only semantics"
                .into(),
        ));
    }
    Ok(())
}

fn canonical_point_results_v1(
    plan: &CompiledCbsemPlanV3,
    estimation: &CbsemCompiledMomentResultV2,
    trace: &CanonicalGeneralSemResultTraceV1,
) -> Result<CanonicalGeneralSemResultsV1, RecipeV4CbsemGeneralSemPointExecutionErrorV1> {
    let parameters = canonical_parameter_rows_v1(plan, estimation, trace)?;
    let fit = canonical_fit_row_v1(estimation, trace)?;
    let identification = canonical_identification_rows_v1(plan, fit.degrees_of_freedom, trace)?;
    Ok(CanonicalGeneralSemResultsV1 {
        schema_version: qpls_core::CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION,
        inference_receipt: None,
        specific_indirect_effects: Vec::new(),
        aggregate_effects: Vec::new(),
        joint_stage_structural_coefficients: Vec::new(),
        interaction_effects: Vec::new(),
        three_way_interaction_effects: Vec::new(),
        three_way_conditional_interaction_effects: Vec::new(),
        three_way_simple_slopes: Vec::new(),
        three_way_moderation_bootstrap_receipt: None,
        conditional_effect_probes: Vec::new(),
        conditional_effects: Vec::new(),
        conditional_indirect_effects: Vec::new(),
        moderated_mediation_indices: Vec::new(),
        interaction_plots: Vec::new(),
        higher_order_stages: Vec::new(),
        higher_order_inference_receipt: None,
        cbsem_parameters: parameters,
        cbsem_fit: vec![fit],
        identification_diagnostics: identification,
        cbsem_bootstrap_receipt: None,
        cbsem_bootstrap_inference: Vec::new(),
    })
}

fn canonical_parameter_rows_v1(
    plan: &CompiledCbsemPlanV3,
    estimation: &CbsemCompiledMomentResultV2,
    trace: &CanonicalGeneralSemResultTraceV1,
) -> Result<Vec<CanonicalCbsemParameterResultV1>, RecipeV4CbsemGeneralSemPointExecutionErrorV1> {
    let point_by_id = point_parameters_by_stable_id(estimation)?;
    let standardized_by_name = standardized_parameters_by_name(estimation)?;
    let relation_ids = relation_ids_by_parameter_id(plan)?;
    if point_by_id.len() != plan.parameters().len()
        || standardized_by_name.len() != plan.parameters().len()
    {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "kernel parameter, standardized, and authoritative V3 inventories differ".into(),
        ));
    }

    let factor_ids = plan
        .base_plan()
        .factors()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let observed_sources = plan
        .base_plan()
        .variables()
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } => Some((id.as_str(), source_column.as_str())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    if observed_sources.len() != plan.base_plan().observed_variables().len() {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "authoritative observed-variable source mapping is incomplete".into(),
        ));
    }
    let mut rows = Vec::with_capacity(plan.parameters().len());
    for authority in plan.parameters() {
        let point = point_by_id.get(authority.id()).ok_or_else(|| {
            RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(format!(
                "kernel omitted authoritative parameter {}",
                authority.id()
            ))
        })?;
        let standardized = standardized_by_name
            .get(point.name.as_str())
            .ok_or_else(|| {
                RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(format!(
                    "kernel omitted standardized parameter {}",
                    authority.id()
                ))
            })?;
        if standardized.kind != point.kind
            || standardized.lhs != point.lhs
            || standardized.rhs != point.rhs
            || !standardized.std_lv.is_finite()
            || !standardized.std_all.is_finite()
            || point.warning.is_some()
            || !point.estimate.is_finite()
        {
            return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                format!(
                    "kernel point/standardized rows for {} are inconsistent or nonfinite",
                    authority.id()
                ),
            ));
        }
        let (role, target, expected_kernel_kind, requires_relation, expected_lhs, expected_rhs) =
            canonical_parameter_target_v1(
                authority.id(),
                authority.target(),
                &factor_ids,
                &observed_sources,
            )?;
        let separator = match expected_kernel_kind {
            "loading" => "=~",
            "structural_path" => "~",
            _ => "~~",
        };
        let expected_name = format!("{expected_lhs}{separator}{expected_rhs}");
        if point.kind != expected_kernel_kind
            || point.lhs != expected_lhs
            || point.rhs != expected_rhs
            || point.name != expected_name
        {
            return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                format!(
                    "kernel coordinates do not match authoritative semantics for {}",
                    authority.id()
                ),
            ));
        }
        let relation_id = relation_ids.get(authority.id()).cloned();
        if requires_relation != relation_id.is_some() {
            return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                format!(
                    "relation identity does not match authoritative semantics for {}",
                    authority.id()
                ),
            ));
        }
        let (state, standard_error, z_value, p_value) = match authority.specification() {
            CompiledCbsemParameterStatusV2::Fixed { value } => {
                if !point.fixed
                    || point.estimate.to_bits() != value.to_bits()
                    || point.standard_error.is_some()
                    || point.z_statistic.is_some()
                    || point.p_value_two_sided.is_some()
                {
                    return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                        format!("fixed parameter {} changed state or value", authority.id()),
                    ));
                }
                (
                    CanonicalCbsemParameterStateV1::Fixed { value: *value },
                    None,
                    None,
                    None,
                )
            }
            CompiledCbsemParameterStatusV2::Free {
                lower,
                upper,
                equality_label,
                ..
            } => {
                let effective_lower =
                    if authority.role() == qpls_core::CompiledCbsemParameterRoleV2::Variance {
                        Some(lower.unwrap_or(0.0).max(0.0))
                    } else {
                        *lower
                    };
                let (Some(standard_error), Some(z_value), Some(p_value)) = (
                    point.standard_error,
                    point.z_statistic,
                    point.p_value_two_sided,
                ) else {
                    return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                        format!(
                            "free parameter {} lacks complete analytical uncertainty",
                            authority.id()
                        ),
                    ));
                };
                if point.fixed
                    || !standard_error.is_finite()
                    || standard_error < 0.0
                    || !z_value.is_finite()
                    || !p_value.is_finite()
                    || !(0.0..=1.0).contains(&p_value)
                    || equality_label
                        .as_deref()
                        .is_some_and(|label| label.trim().is_empty())
                    || lower.is_some_and(|value| !value.is_finite())
                    || upper.is_some_and(|value| !value.is_finite())
                    || effective_lower
                        .zip(*upper)
                        .is_some_and(|(lower, upper)| lower >= upper)
                    || effective_lower.is_some_and(|lower| point.estimate <= lower)
                    || upper.is_some_and(|upper| point.estimate >= upper)
                {
                    return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                        format!("free parameter {} has invalid uncertainty", authority.id()),
                    ));
                }
                (
                    CanonicalCbsemParameterStateV1::Free {
                        equality_label: equality_label.clone(),
                        lower: *lower,
                        upper: *upper,
                    },
                    Some(standard_error),
                    Some(z_value),
                    Some(p_value),
                )
            }
            CompiledCbsemParameterStatusV2::Derived { .. } => {
                return Err(
                    RecipeV4CbsemGeneralSemPointExecutionErrorV1::UnsupportedParameter {
                        parameter_id: authority.id().into(),
                        reason: "derived parameter rows are outside the point v1 predicate".into(),
                    },
                );
            }
        };
        rows.push(CanonicalCbsemParameterResultV1 {
            parameter_id: authority.id().into(),
            trace: trace.clone(),
            role,
            target,
            relation_id,
            state,
            estimate: point.estimate,
            standard_error,
            z_value,
            p_value,
            standardized_estimate: Some(standardized.std_all),
        });
    }
    rows.sort_by(|left, right| left.parameter_id.cmp(&right.parameter_id));
    Ok(rows)
}

fn point_parameters_by_stable_id<'a>(
    estimation: &'a CbsemCompiledMomentResultV2,
) -> Result<BTreeMap<&'a str, &'a CbsemParameter>, RecipeV4CbsemGeneralSemPointExecutionErrorV1> {
    if estimation.parameter_ids.len() != estimation.analysis.parameters.len() {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "kernel parameter-name map and point-row inventory differ".into(),
        ));
    }
    let mut rows = BTreeMap::new();
    for point in &estimation.analysis.parameters {
        let parameter_id = estimation.parameter_ids.get(&point.name).ok_or_else(|| {
            RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(format!(
                "kernel point row {} has no stable parameter identity",
                point.name
            ))
        })?;
        if rows.insert(parameter_id.as_str(), point).is_some() {
            return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                format!("kernel duplicated stable parameter identity {parameter_id}"),
            ));
        }
    }
    Ok(rows)
}

fn standardized_parameters_by_name<'a>(
    estimation: &'a CbsemCompiledMomentResultV2,
) -> Result<
    BTreeMap<&'a str, &'a CbsemStandardizedParameter>,
    RecipeV4CbsemGeneralSemPointExecutionErrorV1,
> {
    let mut rows = BTreeMap::new();
    for row in &estimation.analysis.standardized {
        if rows.insert(row.name.as_str(), row).is_some() {
            return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                format!("kernel duplicated standardized parameter name {}", row.name),
            ));
        }
    }
    Ok(rows)
}

fn relation_ids_by_parameter_id(
    plan: &CompiledCbsemPlanV3,
) -> Result<BTreeMap<String, String>, RecipeV4CbsemGeneralSemPointExecutionErrorV1> {
    let mut relations = BTreeMap::new();
    for (parameter_id, relation_id) in plan
        .base_plan()
        .loadings()
        .iter()
        .map(|row| (row.parameter_id(), row.relation_id()))
        .chain(
            plan.base_plan()
                .regressions()
                .iter()
                .map(|row| (row.parameter_id(), row.relation_id())),
        )
        .chain(
            plan.base_plan()
                .covariances()
                .iter()
                .map(|row| (row.parameter_id(), row.relation_id())),
        )
    {
        if relations
            .insert(parameter_id.to_string(), relation_id.to_string())
            .is_some()
        {
            return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                format!("parameter {parameter_id} is bound to multiple relations"),
            ));
        }
    }
    Ok(relations)
}

fn canonical_parameter_target_v1(
    parameter_id: &str,
    target: &SemParameterTargetV4,
    factor_ids: &BTreeSet<&str>,
    observed_sources: &BTreeMap<&str, &str>,
) -> Result<
    (
        CanonicalCbsemParameterRoleV1,
        CanonicalCbsemParameterTargetV1,
        &'static str,
        bool,
        String,
        String,
    ),
    RecipeV4CbsemGeneralSemPointExecutionErrorV1,
> {
    let unsupported =
        |reason: &str| RecipeV4CbsemGeneralSemPointExecutionErrorV1::UnsupportedParameter {
            parameter_id: parameter_id.into(),
            reason: reason.into(),
        };
    match target {
        SemParameterTargetV4::Loading {
            construct,
            indicator,
        } if factor_ids.contains(construct.as_str())
            && observed_sources.contains_key(indicator.as_str()) =>
        {
            let indicator_source = observed_sources[indicator.as_str()];
            Ok((
                CanonicalCbsemParameterRoleV1::Loading,
                CanonicalCbsemParameterTargetV1::Loading {
                    factor_id: construct.clone(),
                    indicator_id: indicator.clone(),
                },
                "loading",
                true,
                construct.clone(),
                indicator_source.into(),
            ))
        }
        SemParameterTargetV4::Regression { source, target }
            if factor_ids.contains(source.as_str()) && factor_ids.contains(target.as_str()) =>
        {
            Ok((
                CanonicalCbsemParameterRoleV1::Regression,
                CanonicalCbsemParameterTargetV1::Regression {
                    source_id: source.clone(),
                    target_id: target.clone(),
                },
                "structural_path",
                true,
                target.clone(),
                source.clone(),
            ))
        }
        SemParameterTargetV4::Variance { endpoint } => {
            let (expected_kind, coordinate) = match endpoint {
                SemEndpointV4::Variable(id) | SemEndpointV4::DisturbanceOf(id)
                    if factor_ids.contains(id.as_str()) =>
                {
                    ("latent_variance", id.as_str())
                }
                SemEndpointV4::ResidualOf(id) if observed_sources.contains_key(id.as_str()) => {
                    ("residual_variance", observed_sources[id.as_str()])
                }
                _ => {
                    return Err(unsupported(
                        "variance endpoint is outside the bounded factor/residual predicate",
                    ));
                }
            };
            Ok((
                CanonicalCbsemParameterRoleV1::Variance,
                CanonicalCbsemParameterTargetV1::Variance {
                    endpoint: canonical_endpoint_v1(endpoint),
                },
                expected_kind,
                false,
                coordinate.into(),
                coordinate.into(),
            ))
        }
        SemParameterTargetV4::Covariance { left, right } => {
            let (expected_kind, left_coordinate, right_coordinate) = match (left, right) {
                (SemEndpointV4::Variable(left), SemEndpointV4::Variable(right))
                    if factor_ids.contains(left.as_str())
                        && factor_ids.contains(right.as_str()) =>
                {
                    ("latent_covariance", left.as_str(), right.as_str())
                }
                (SemEndpointV4::DisturbanceOf(left), SemEndpointV4::DisturbanceOf(right))
                    if factor_ids.contains(left.as_str())
                        && factor_ids.contains(right.as_str()) =>
                {
                    ("disturbance_covariance", left.as_str(), right.as_str())
                }
                (SemEndpointV4::ResidualOf(left), SemEndpointV4::ResidualOf(right))
                    if observed_sources.contains_key(left.as_str())
                        && observed_sources.contains_key(right.as_str()) =>
                {
                    (
                        "residual_covariance",
                        observed_sources[left.as_str()],
                        observed_sources[right.as_str()],
                    )
                }
                _ => {
                    return Err(unsupported(
                        "covariance endpoints are mixed or outside the bounded latent/residual predicate",
                    ));
                }
            };
            let (left_coordinate, right_coordinate) =
                canonical_coordinate_pair_v1(left_coordinate, right_coordinate);
            Ok((
                CanonicalCbsemParameterRoleV1::Covariance,
                CanonicalCbsemParameterTargetV1::Covariance {
                    left: canonical_endpoint_v1(left),
                    right: canonical_endpoint_v1(right),
                },
                expected_kind,
                true,
                left_coordinate,
                right_coordinate,
            ))
        }
        _ => Err(unsupported(
            "only loading, regression, variance, and covariance rows are executable",
        )),
    }
}

fn canonical_coordinate_pair_v1(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.into(), right.into())
    } else {
        (right.into(), left.into())
    }
}

fn canonical_endpoint_v1(endpoint: &SemEndpointV4) -> CanonicalCbsemEndpointV1 {
    match endpoint {
        SemEndpointV4::Variable(variable_id) => CanonicalCbsemEndpointV1::Variable {
            variable_id: variable_id.clone(),
        },
        SemEndpointV4::ResidualOf(variable_id) => CanonicalCbsemEndpointV1::Residual {
            variable_id: variable_id.clone(),
        },
        SemEndpointV4::DisturbanceOf(variable_id) => CanonicalCbsemEndpointV1::Disturbance {
            variable_id: variable_id.clone(),
        },
    }
}

fn canonical_fit_row_v1(
    estimation: &CbsemCompiledMomentResultV2,
    trace: &CanonicalGeneralSemResultTraceV1,
) -> Result<CanonicalCbsemFitResultV1, RecipeV4CbsemGeneralSemPointExecutionErrorV1> {
    let fit = &estimation.analysis.fit;
    let degrees_of_freedom = u32::try_from(fit.degrees_of_freedom).map_err(|_| {
        RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "fit degrees of freedom are negative or exceed u32".into(),
        )
    })?;
    if !fit.chi_square.is_finite()
        || fit.chi_square < 0.0
        || fit
            .p_value
            .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        || (degrees_of_freedom == 0 && fit.p_value.is_some())
        || fit
            .rmsea
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        || fit.cfi.is_some_and(|value| !value.is_finite())
        || fit.tli.is_some_and(|value| !value.is_finite())
        || !fit.srmr.is_finite()
        || fit.srmr < 0.0
        || !fit.aic.is_finite()
        || !fit.bic.is_finite()
    {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "qualified V2 kernel returned invalid canonical fit values".into(),
        ));
    }
    let rmsea_interval = match (
        &fit.rmsea_interval_attribution,
        fit.rmsea_ci_lower,
        fit.rmsea_ci_upper,
    ) {
        (Some(attribution), Some(lower), Some(upper))
            if attribution.method_version == CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1
                && attribution.confidence_level.is_finite()
                && attribution.confidence_level > 0.0
                && attribution.confidence_level < 1.0
                && fit.rmsea.is_some()
                && lower.is_finite()
                && upper.is_finite()
                && lower >= 0.0
                && lower <= upper =>
        {
            Some(CanonicalGeneralSemIntervalV1 {
                confidence_level: attribution.confidence_level,
                lower,
                upper,
            })
        }
        (None, None, None) => None,
        _ => {
            return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                "RMSEA interval bounds and attribution are incomplete or invalid".into(),
            ));
        }
    };
    Ok(CanonicalCbsemFitResultV1 {
        fit_id: "cbsem_fit:ml".into(),
        trace: trace.clone(),
        chi_square: fit.chi_square,
        degrees_of_freedom,
        chi_square_p_value: fit.p_value,
        rmsea: fit.rmsea,
        rmsea_interval,
        cfi: fit.cfi,
        tli: fit.tli,
        srmr: Some(fit.srmr),
        aic: Some(fit.aic),
        bic: Some(fit.bic),
    })
}

fn canonical_identification_rows_v1(
    plan: &CompiledCbsemPlanV3,
    degrees_of_freedom: u32,
    trace: &CanonicalGeneralSemResultTraceV1,
) -> Result<Vec<CanonicalIdentificationDiagnosticV1>, RecipeV4CbsemGeneralSemPointExecutionErrorV1>
{
    let factor_ids = plan
        .base_plan()
        .factors()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut restricted_factors = BTreeSet::new();
    let mut rows = vec![CanonicalIdentificationDiagnosticV1 {
        diagnostic_id: "cbsem_identification:model".into(),
        trace: trace.clone(),
        scope: CanonicalIdentificationScopeV1::Model,
        subject_id: plan.model_id().into(),
        status: CanonicalIdentificationStatusV1::Provisional,
        code: "local_identification_evidence_only".into(),
        message: format!(
            "Recursive topology with {} strongly connected component(s) and local factor-scale restrictions; no global Jacobian-rank proof is claimed.",
            plan.identification_evidence()
                .strongly_connected_components()
                .len()
        ),
        degrees_of_freedom: Some(i64::from(degrees_of_freedom)),
    }];
    for restriction in plan.identification_evidence().factor_scale_restrictions() {
        let (factor_id, code, message) = match restriction {
            CompiledCbsemFactorScaleRestrictionV3::MarkerLoading {
                factor_id,
                indicator_id,
                parameter_id,
            } => (
                factor_id,
                "marker_loading_scale",
                format!(
                    "Factor scale is anchored by marker indicator {indicator_id} through fixed loading {parameter_id}."
                ),
            ),
            CompiledCbsemFactorScaleRestrictionV3::FixedVariance {
                factor_id,
                parameter_id,
            } => (
                factor_id,
                "fixed_variance_scale",
                format!("Factor scale is anchored by fixed variance {parameter_id}."),
            ),
            CompiledCbsemFactorScaleRestrictionV3::EffectsCoding { factor_id, .. } => {
                return Err(
                    RecipeV4CbsemGeneralSemPointExecutionErrorV1::UnsupportedParameter {
                        parameter_id: factor_id.clone(),
                        reason: "effects-coding constraints are outside the point v1 predicate"
                            .into(),
                    },
                );
            }
        };
        if !factor_ids.contains(factor_id.as_str())
            || !restricted_factors.insert(factor_id.as_str())
        {
            return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
                format!("factor-scale evidence for {factor_id} is missing or duplicated"),
            ));
        }
        rows.push(CanonicalIdentificationDiagnosticV1 {
            diagnostic_id: format!("cbsem_identification:factor:{factor_id}"),
            trace: trace.clone(),
            scope: CanonicalIdentificationScopeV1::Variable,
            subject_id: factor_id.clone(),
            status: CanonicalIdentificationStatusV1::Identified,
            code: code.into(),
            message,
            degrees_of_freedom: None,
        });
    }
    if restricted_factors != factor_ids {
        return Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::Authority(
            "factor-scale evidence does not cover the exact common-factor inventory".into(),
        ));
    }
    rows.sort_by(|left, right| left.diagnostic_id.cmp(&right.diagnostic_id));
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_V4_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipeModelBindingV4,
        AnalysisSettings, CbsemBootstrapAlgorithm, CbsemBootstrapConfigV2, CbsemBootstrapInterval,
        CbsemBootstrapTestTail, CbsemEstimator, CbsemInput, CbsemModelType, Construct,
        GeneralSemBootstrapIntervalV1, GeneralSemConfigV1, GeneralSemInferenceTailV1,
        LegacyBasicModelInterpretationV4, LegacyEstimandConfirmationV4, MeasurementMode,
        MethodConfig, MissingDataPolicy, MissingDataPolicyV4, ModelSpec, Preprocessing,
        SemDataBindingV4, StructuralPath, cbsem_recursive_sem_bootstrap_capability_cell_v1,
        convert_legacy_basic_model_v4,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use uuid::Uuid;

    fn recursive_fixture() -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/lavaan_latent_regression_sem.csv"),
            "rank3-cbsem-v3-recursive-point.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let legacy = ModelSpec {
            id: Uuid::from_u128(0xcb53_3001),
            name: "Rank 3 recursive point adapter fixture".into(),
            constructs: [
                ("x", ["x1", "x2", "x3"]),
                ("m", ["m1", "m2", "m3"]),
                ("y", ["y1", "y2", "y3"]),
            ]
            .into_iter()
            .map(|(id, indicators)| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: indicators.into_iter().map(str::to_owned).collect(),
            })
            .collect(),
            paths: [("x", "m"), ("x", "y"), ("m", "y")]
                .into_iter()
                .map(|(source, target)| StructuralPath {
                    source: source.into(),
                    target: target.into(),
                })
                .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.data_binding = SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        model.ensure_valid().unwrap();

        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Cbsem;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.missing_data = MissingDataPolicy::ListwiseDeletion;
        settings.workers = 1;
        let recipe = AnalysisRecipeV4 {
            schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: Uuid::from_u128(0xcb53_3002),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                model: model.clone(),
                scientific_sha256: model.scientific_sha256().unwrap(),
            },
            estimand_confirmation: LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
            settings,
            method_config: Some(MethodConfig::Cbsem {
                model_type: CbsemModelType::Sem,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Raw,
                mean_structure: false,
                bootstrap_samples: 0,
                bootstrap_v2: None,
                group_column: None,
                invariance_steps: Vec::new(),
            }),
            general_sem_config: Some(GeneralSemConfigV1::default()),
            metadata: BTreeMap::new(),
            legacy_source: None,
        };
        let target = RecipeV4CompilerTarget::CbsemPlanV3;
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            target,
            target.capability_cell_for_recipe(&recipe),
        )
        .unwrap();
        (dataset, recipe, model, artifact)
    }

    #[test]
    fn v3_point_adapter_maps_the_complete_v2_kernel_authority() {
        let (dataset, recipe, model, artifact) = recursive_fixture();
        let result = run_compiled_cbsem_general_sem_point_v1(
            &dataset,
            &recipe,
            &model,
            &artifact,
            || false,
            |_| {},
        )
        .unwrap();
        let CompiledRecipePlanV4::CbsemPlanV3 { plan } = artifact.plan() else {
            unreachable!()
        };
        let general_sem = result.general_sem_results();

        assert_eq!(
            result.schema_version(),
            RECIPE_V4_CBSEM_GENERAL_SEM_POINT_EXECUTION_RESULT_SCHEMA_VERSION_V1
        );
        assert_eq!(
            result.provenance().adapter_version(),
            RECIPE_V4_CBSEM_GENERAL_SEM_POINT_EXECUTION_ADAPTER_VERSION_V1
        );
        assert_eq!(
            result.provenance().v3_compilation_receipt(),
            artifact.receipt()
        );
        assert_eq!(
            result
                .provenance()
                .v2_kernel_compilation_receipt()
                .compiler_target(),
            RecipeV4CompilerTarget::CbsemPlanV2
        );
        assert_eq!(result.provenance().dataset_id(), dataset.id.to_string());
        assert_eq!(
            result.provenance().dataset_fingerprint(),
            dataset.fingerprint.0
        );
        assert_eq!(general_sem.cbsem_parameters.len(), plan.parameters().len());
        assert_eq!(
            general_sem
                .cbsem_parameters
                .iter()
                .map(|row| row.parameter_id.as_str())
                .collect::<Vec<_>>(),
            plan.parameters()
                .iter()
                .map(|row| row.id())
                .collect::<Vec<_>>()
        );
        assert!(general_sem.cbsem_parameters.iter().all(|row| {
            row.trace.model_id == plan.model_id()
                && row.trace.capability_cell == cbsem_general_sem_ml_capability_cell_v1()
                && row.standardized_estimate.is_some()
        }));
        assert_eq!(
            general_sem
                .cbsem_parameters
                .iter()
                .filter(|row| row.role == CanonicalCbsemParameterRoleV1::Regression)
                .count(),
            plan.base_plan().regressions().len()
        );
        assert!(general_sem.cbsem_parameters.iter().all(|row| {
            matches!(
                row.role,
                CanonicalCbsemParameterRoleV1::Loading
                    | CanonicalCbsemParameterRoleV1::Regression
                    | CanonicalCbsemParameterRoleV1::Covariance
            ) == row.relation_id.is_some()
        }));
        assert_eq!(general_sem.cbsem_fit.len(), 1);
        assert_eq!(
            general_sem.cbsem_fit[0].trace.capability_cell,
            cbsem_general_sem_ml_capability_cell_v1()
        );
        assert!(general_sem.cbsem_fit[0].chi_square.is_finite());
        assert_eq!(general_sem.identification_diagnostics.len(), 4);
        assert!(general_sem.identification_diagnostics.iter().any(|row| {
            row.scope == CanonicalIdentificationScopeV1::Model
                && row.status == CanonicalIdentificationStatusV1::Provisional
        }));
        assert_eq!(
            general_sem
                .identification_diagnostics
                .iter()
                .filter(|row| row.status == CanonicalIdentificationStatusV1::Identified)
                .count(),
            plan.base_plan().factors().len()
        );
        assert!(general_sem.cbsem_bootstrap_receipt.is_none());
        assert!(general_sem.cbsem_bootstrap_inference.is_empty());
    }

    #[test]
    fn bootstrap_artifact_is_rejected_before_the_point_kernel() {
        let (dataset, mut recipe, model, _) = recursive_fixture();
        recipe.settings.bootstrap_samples = 500;
        recipe.settings.seed = 777;
        recipe.settings.confidence_level = 0.95;
        recipe.general_sem_config = Some(GeneralSemConfigV1 {
            inference: GeneralSemInferenceV1::CaseBootstrap {
                resamples: 500,
                seed: 777,
                confidence_level: 0.95,
                interval: GeneralSemBootstrapIntervalV1::Percentile,
                tail: GeneralSemInferenceTailV1::TwoSided,
            },
            ..GeneralSemConfigV1::default()
        });
        let Some(MethodConfig::Cbsem {
            bootstrap_samples,
            bootstrap_v2,
            ..
        }) = recipe.method_config.as_mut()
        else {
            unreachable!()
        };
        *bootstrap_samples = 500;
        *bootstrap_v2 = Some(CbsemBootstrapConfigV2 {
            algorithm: CbsemBootstrapAlgorithm::CaseResamplingFullMl,
            interval: CbsemBootstrapInterval::PercentileType7,
            test_tail: CbsemBootstrapTestTail::TwoSided,
        });
        let target = RecipeV4CompilerTarget::CbsemPlanV3;
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            target,
            cbsem_recursive_sem_bootstrap_capability_cell_v1(),
        )
        .unwrap();

        assert!(matches!(
            run_compiled_cbsem_general_sem_point_v1(
                &dataset,
                &recipe,
                &model,
                &artifact,
                || false,
                |_| {},
            ),
            Err(RecipeV4CbsemGeneralSemPointExecutionErrorV1::PointOnly)
        ));
    }

    #[test]
    fn unsupported_parameter_families_fail_closed() {
        let error = canonical_parameter_target_v1(
            "parameter:intercept:x1",
            &SemParameterTargetV4::Intercept {
                variable: "observed:x1".into(),
            },
            &BTreeSet::new(),
            &BTreeMap::new(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            RecipeV4CbsemGeneralSemPointExecutionErrorV1::UnsupportedParameter {
                parameter_id,
                ..
            } if parameter_id == "parameter:intercept:x1"
        ));
    }
}
