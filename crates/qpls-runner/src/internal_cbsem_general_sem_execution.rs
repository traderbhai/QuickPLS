//! Bounded product-integration bridge for the CB-SEM V3 Experimental Labs cells.
//!
//! Registry, surface, archive, and resident-recipe authorization remain owned
//! by the native job boundary. This runner accepts no caller-selected method or
//! capability override and therefore cannot promote a neighboring CB-SEM cell.

use qpls_core::{
    AnalysisRecipeV4, CanonicalGeneralSemResultsV1, CapabilityCellReferenceV2,
    GeneralSemInferenceV1, RecipeV4CompilationReceipt, RecipeV4CompilerTarget, SemModelV4,
    cbsem_general_sem_ml_capability_cell_v1, cbsem_recursive_sem_bootstrap_capability_cell_v1,
    compile_analysis_recipe_v4,
};
use qpls_data::Dataset;

use crate::{
    RunnerProgress,
    recipe_v4_cbsem_general_sem_bootstrap_execution::{
        RECIPE_V4_CBSEM_GENERAL_SEM_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
        RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1,
        run_compiled_cbsem_general_sem_recursive_bootstrap_v1,
    },
    recipe_v4_cbsem_general_sem_point_execution::{
        RECIPE_V4_CBSEM_GENERAL_SEM_POINT_EXECUTION_ADAPTER_VERSION_V1,
        RecipeV4CbsemGeneralSemPointExecutionErrorV1, run_compiled_cbsem_general_sem_point_v1,
    },
};

pub const INTERNAL_CBSEM_GENERAL_SEM_EXECUTION_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub const INTERNAL_CBSEM_GENERAL_SEM_POINT_ADAPTER_VERSION_V1: &str =
    RECIPE_V4_CBSEM_GENERAL_SEM_POINT_EXECUTION_ADAPTER_VERSION_V1;
pub const INTERNAL_CBSEM_GENERAL_SEM_BOOTSTRAP_ADAPTER_VERSION_V1: &str =
    RECIPE_V4_CBSEM_GENERAL_SEM_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1;

#[derive(Debug, Clone, PartialEq)]
pub struct InternalCbsemGeneralSemExecutionResultV1 {
    schema_version: u32,
    adapter_version: String,
    compilation_receipt: RecipeV4CompilationReceipt,
    capability_cells: Vec<CapabilityCellReferenceV2>,
    general_sem_results: CanonicalGeneralSemResultsV1,
}

impl InternalCbsemGeneralSemExecutionResultV1 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn adapter_version(&self) -> &str {
        &self.adapter_version
    }

    pub fn compilation_receipt(&self) -> &RecipeV4CompilationReceipt {
        &self.compilation_receipt
    }

    pub fn capability_cells(&self) -> &[CapabilityCellReferenceV2] {
        &self.capability_cells
    }

    pub fn general_sem_results(&self) -> &CanonicalGeneralSemResultsV1 {
        &self.general_sem_results
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InternalCbsemGeneralSemExecutionErrorV1 {
    #[error("analysis was cancelled")]
    Cancelled,
    #[error("CB-SEM General SEM V3 compilation failed: {0}")]
    Compilation(String),
    #[error("CB-SEM General SEM point execution failed: {0}")]
    Point(String),
    #[error("CB-SEM General SEM recursive bootstrap failed: {0}")]
    Bootstrap(String),
}

/// Runs exactly one V3 point or recursive-bootstrap adapter selected
/// by the strict resident recipe. No caller-provided method/cell override is
/// accepted, which prevents an internal native caller from relabelling output.
pub fn run_internal_cbsem_general_sem_v3(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<InternalCbsemGeneralSemExecutionResultV1, InternalCbsemGeneralSemExecutionErrorV1> {
    if should_cancel() {
        return Err(InternalCbsemGeneralSemExecutionErrorV1::Cancelled);
    }
    let inference = recipe
        .general_sem_config
        .as_ref()
        .ok_or_else(|| {
            InternalCbsemGeneralSemExecutionErrorV1::Compilation(
                "GeneralSemConfigV1 is required".into(),
            )
        })?
        .inference;
    let capability_cell = match inference {
        GeneralSemInferenceV1::None => cbsem_general_sem_ml_capability_cell_v1(),
        GeneralSemInferenceV1::CaseBootstrap { .. } => {
            cbsem_recursive_sem_bootstrap_capability_cell_v1()
        }
    };
    let artifact = compile_analysis_recipe_v4(
        recipe,
        Some(resolved_model),
        RecipeV4CompilerTarget::CbsemPlanV3,
        capability_cell,
    )
    .map_err(|error| InternalCbsemGeneralSemExecutionErrorV1::Compilation(error.to_string()))?;

    let (adapter_version, general_sem_results) = match inference {
        GeneralSemInferenceV1::None => {
            let result = run_compiled_cbsem_general_sem_point_v1(
                dataset,
                recipe,
                resolved_model,
                &artifact,
                &should_cancel,
                &progress,
            )
            .map_err(map_point_error)?;
            (
                result.provenance().adapter_version().to_owned(),
                result.general_sem_results().clone(),
            )
        }
        GeneralSemInferenceV1::CaseBootstrap { .. } => {
            let result = run_compiled_cbsem_general_sem_recursive_bootstrap_v1(
                dataset,
                recipe,
                resolved_model,
                &artifact,
                &should_cancel,
                &progress,
            )
            .map_err(map_bootstrap_error)?;
            (
                result.adapter_version().to_owned(),
                result.general_sem_results().clone(),
            )
        }
    };
    if should_cancel() {
        return Err(InternalCbsemGeneralSemExecutionErrorV1::Cancelled);
    }

    let capability_cells = match artifact.plan() {
        qpls_core::CompiledRecipePlanV4::CbsemPlanV3 { plan } => plan.capability_cells().to_vec(),
        _ => {
            return Err(InternalCbsemGeneralSemExecutionErrorV1::Compilation(
                "compiler returned a non-V3 plan".into(),
            ));
        }
    };
    Ok(InternalCbsemGeneralSemExecutionResultV1 {
        schema_version: INTERNAL_CBSEM_GENERAL_SEM_EXECUTION_RESULT_SCHEMA_VERSION_V1,
        adapter_version,
        compilation_receipt: artifact.receipt().clone(),
        capability_cells,
        general_sem_results,
    })
}

fn map_point_error(
    error: RecipeV4CbsemGeneralSemPointExecutionErrorV1,
) -> InternalCbsemGeneralSemExecutionErrorV1 {
    match error {
        RecipeV4CbsemGeneralSemPointExecutionErrorV1::Cancelled => {
            InternalCbsemGeneralSemExecutionErrorV1::Cancelled
        }
        other => InternalCbsemGeneralSemExecutionErrorV1::Point(other.to_string()),
    }
}

fn map_bootstrap_error(
    error: RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1,
) -> InternalCbsemGeneralSemExecutionErrorV1 {
    match error {
        RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Cancelled => {
            InternalCbsemGeneralSemExecutionErrorV1::Cancelled
        }
        other => InternalCbsemGeneralSemExecutionErrorV1::Bootstrap(other.to_string()),
    }
}
