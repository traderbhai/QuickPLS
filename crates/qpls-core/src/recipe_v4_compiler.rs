use crate::{
    AnalysisMethod, AnalysisRecipeModelBindingV4, AnalysisRecipeV4, AnalysisRecipeV4Error,
    CapabilityCellReferenceV2, CbsemBootstrapAlgorithm, CbsemBootstrapConfigV2,
    CbsemBootstrapInterval, CbsemEstimator, CbsemEstimatorCapabilityErrorV2,
    CbsemExactCaseBootstrapZeroNullEligibilityStatusV1,
    CbsemExactCaseBootstrapZeroNullEligibilityV1,
    CbsemExactCaseBootstrapZeroNullUnavailableReasonV1, CbsemInput,
    CbsemMlExactCapabilityProfileV2, CbsemModelType, CompiledCbsemParameterRoleV2,
    CompiledCbsemParameterStatusV2, CompiledCbsemPlanV2, CompiledCbsemPlanV2Error,
    CompiledCbsemProductIndicatorErrorV1, CompiledCbsemProductIndicatorPlanV1, CompiledPlsPlanV2,
    CompiledPlsPlanV2Error, EXECUTABLE_LEGACY_METADATA_KEYS, LegacyEstimandConfirmationV4,
    MethodConfig, MissingDataPolicy, MissingDataPolicyV4, SemConstraintV4, SemDataBindingV4,
    SemModelV4, SemModelV4ValidationError, SemVariableV4, WeightCapabilityIssueContractErrorV1,
    WeightCapabilityIssueV1, WeightCapabilityTargetV1, WeightDeclarationResolutionErrorV1,
    compile_cbsem_plan_v2, compile_cbsem_product_indicator_plan_v1, compile_pls_plan_v2,
    ensure_cbsem_ml_exact_parameter_table_v4_capability_v2, resolve_weight_declaration_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use uuid::Uuid;

pub const RECIPE_V4_COMPILATION_RECEIPT_SCHEMA_VERSION: u32 = 1;
pub const RECIPE_V4_PLS_COMPILER_VERSION: &str = "recipe_v4_to_compiled_pls_plan_v2_v2";
pub const RECIPE_V4_CBSEM_COMPILER_VERSION: &str = "recipe_v4_to_compiled_cbsem_plan_v2_v2";
pub const RECIPE_V4_CBSEM_PRODUCT_INDICATOR_COMPILER_VERSION: &str =
    "recipe_v4_to_compiled_cbsem_product_indicator_plan_v1_v1";

pub const PLS_ALGORITHM_CAPABILITY_ID: &str = "smartpls.pls_algorithm";
pub const PLS_ALGORITHM_CELL_ID: &str = "qpls3.pls.algorithm";
pub const PLS_ALGORITHM_CAPABILITY_VERSION: &str = "pls_pm_v1";
pub const PLS_NONLINEAR_EFFECTS_CAPABILITY_ID: &str = "smartpls.nonlinear_relationships";
pub const PLS_NONLINEAR_EFFECTS_CELL_ID: &str = "qpls3.pls.nonlinear_quadratic";
pub const PLS_NONLINEAR_EFFECTS_CAPABILITY_VERSION: &str = "pls_quadratic_nonlinear_effects_v1";
pub const CBSEM_ML_CAPABILITY_ID: &str = "smartpls.cbsem";
pub const CBSEM_ML_CELL_ID: &str = "qpls3.cbsem.ml";
pub const CBSEM_ML_CAPABILITY_VERSION: &str = "cbsem_ml_v1";
pub const CBSEM_EXACT_BOOTSTRAP_CAPABILITY_ID: &str = "smartpls.cbsem_bootstrapping";
pub const CBSEM_EXACT_BOOTSTRAP_CELL_ID: &str = "qpls3.cbsem.bootstrap";
pub const CBSEM_EXACT_BOOTSTRAP_CAPABILITY_VERSION: &str =
    "cbsem_exact_case_bootstrap_v1";
pub const CBSEM_PRODUCT_INDICATOR_CAPABILITY_ID: &str = "smartpls.cbsem_moderator";
pub const CBSEM_PRODUCT_INDICATOR_CELL_ID: &str = "qpls3.cbsem.moderator.product_indicator";
pub const CBSEM_PRODUCT_INDICATOR_CAPABILITY_VERSION: &str = "cbsem_product_indicator_v1";
pub const CBSEM_PRODUCT_INDICATOR_MAX_ITERATIONS_V1: u32 = 1_000;
pub const CBSEM_PRODUCT_INDICATOR_TOLERANCE_V1: f64 = 1e-7;

/// Derive zero-null testing eligibility from the exact compiled free-row
/// family and domain. Rows are stable-ID sorted to match exact bootstrap point
/// and refit projections; no numeric estimate is consulted.
pub fn compile_cbsem_exact_case_bootstrap_zero_null_eligibility_v1(
    plan: &CompiledCbsemPlanV2,
) -> Vec<CbsemExactCaseBootstrapZeroNullEligibilityV1> {
    let zero_excluding_bound_parameters = plan
        .constraints()
        .iter()
        .filter_map(|constraint| match constraint {
            SemConstraintV4::Bound {
                parameter,
                lower,
                upper,
                ..
            } if lower.as_ref().is_some_and(|value| *value >= 0.0)
                || upper.as_ref().is_some_and(|value| *value <= 0.0) =>
            {
                Some(parameter.as_str())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut rows = plan
        .parameters()
        .iter()
        .filter_map(|parameter| {
            let CompiledCbsemParameterStatusV2::Free { lower, upper, .. } =
                parameter.specification()
            else {
                return None;
            };
            let status = match parameter.role() {
                CompiledCbsemParameterRoleV2::Variance => {
                    CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                        reason: CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::NonregularVarianceBoundary,
                    }
                }
                CompiledCbsemParameterRoleV2::Loading
                | CompiledCbsemParameterRoleV2::Covariance
                    if lower.as_ref().is_some_and(|value| *value >= 0.0)
                        || upper.as_ref().is_some_and(|value| *value <= 0.0)
                        || zero_excluding_bound_parameters.contains(parameter.id()) =>
                {
                    CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                        reason: CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::ZeroNullOutsideOpenDomain,
                    }
                }
                CompiledCbsemParameterRoleV2::Loading
                | CompiledCbsemParameterRoleV2::Covariance => {
                    CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Available
                }
                _ => CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                    reason: CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::UnsupportedParameterFamily,
                },
            };
            Some(CbsemExactCaseBootstrapZeroNullEligibilityV1 {
                parameter_id: parameter.id().into(),
                status,
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.parameter_id.cmp(&right.parameter_id));
    rows
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecipeV4CompilerTarget {
    PlsPlanV2,
    CbsemPlanV2,
    CbsemProductIndicatorPlanV1,
}

impl RecipeV4CompilerTarget {
    pub const fn compiler_version(self) -> &'static str {
        match self {
            Self::PlsPlanV2 => RECIPE_V4_PLS_COMPILER_VERSION,
            Self::CbsemPlanV2 => RECIPE_V4_CBSEM_COMPILER_VERSION,
            Self::CbsemProductIndicatorPlanV1 => RECIPE_V4_CBSEM_PRODUCT_INDICATOR_COMPILER_VERSION,
        }
    }

    pub fn capability_cell(self) -> CapabilityCellReferenceV2 {
        match self {
            Self::PlsPlanV2 => CapabilityCellReferenceV2 {
                registry_schema_version: 2,
                capability_id: PLS_ALGORITHM_CAPABILITY_ID.into(),
                cell_id: PLS_ALGORITHM_CELL_ID.into(),
                capability_version: PLS_ALGORITHM_CAPABILITY_VERSION.into(),
            },
            Self::CbsemPlanV2 => CapabilityCellReferenceV2 {
                registry_schema_version: 2,
                capability_id: CBSEM_ML_CAPABILITY_ID.into(),
                cell_id: CBSEM_ML_CELL_ID.into(),
                capability_version: CBSEM_ML_CAPABILITY_VERSION.into(),
            },
            Self::CbsemProductIndicatorPlanV1 => CapabilityCellReferenceV2 {
                registry_schema_version: 2,
                capability_id: CBSEM_PRODUCT_INDICATOR_CAPABILITY_ID.into(),
                cell_id: CBSEM_PRODUCT_INDICATOR_CELL_ID.into(),
                capability_version: CBSEM_PRODUCT_INDICATOR_CAPABILITY_VERSION.into(),
            },
        }
    }

    /// Primary capability identity for an exact Recipe-v4 compilation. The
    /// PLS plan remains the numerical base for nonlinear diagnostics, while
    /// the immutable receipt identifies the nonlinear analytical cell that
    /// the recipe actually requested.
    pub fn capability_cell_for_method(self, method: AnalysisMethod) -> CapabilityCellReferenceV2 {
        if self == Self::PlsPlanV2 && method == AnalysisMethod::NonlinearEffects {
            CapabilityCellReferenceV2 {
                registry_schema_version: 2,
                capability_id: PLS_NONLINEAR_EFFECTS_CAPABILITY_ID.into(),
                cell_id: PLS_NONLINEAR_EFFECTS_CELL_ID.into(),
                capability_version: PLS_NONLINEAR_EFFECTS_CAPABILITY_VERSION.into(),
            }
        } else {
            self.capability_cell()
        }
    }
}

pub fn cbsem_exact_bootstrap_capability_cell_v1() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: CBSEM_EXACT_BOOTSTRAP_CAPABILITY_ID.into(),
        cell_id: CBSEM_EXACT_BOOTSTRAP_CELL_ID.into(),
        capability_version: CBSEM_EXACT_BOOTSTRAP_CAPABILITY_VERSION.into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledRecipePlanV4 {
    PlsPlanV2 {
        plan: CompiledPlsPlanV2,
    },
    CbsemPlanV2 {
        plan: CompiledCbsemPlanV2,
    },
    CbsemProductIndicatorPlanV1 {
        plan: CompiledCbsemProductIndicatorPlanV1,
    },
}

impl CompiledRecipePlanV4 {
    pub const fn target(&self) -> RecipeV4CompilerTarget {
        match self {
            Self::PlsPlanV2 { .. } => RecipeV4CompilerTarget::PlsPlanV2,
            Self::CbsemPlanV2 { .. } => RecipeV4CompilerTarget::CbsemPlanV2,
            Self::CbsemProductIndicatorPlanV1 { .. } => {
                RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1
            }
        }
    }

    pub fn scientific_hash(&self) -> &str {
        match self {
            Self::PlsPlanV2 { plan } => plan.scientific_hash(),
            Self::CbsemPlanV2 { plan } => plan.scientific_hash(),
            Self::CbsemProductIndicatorPlanV1 { plan } => plan.source_model_scientific_sha256(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecipeV4CompilationReceipt {
    schema_version: u32,
    recipe_id: Uuid,
    recipe_document_sha256: String,
    recipe_analytical_sha256: String,
    model_id: String,
    model_document_sha256: String,
    model_scientific_sha256: String,
    dataset_fingerprint: String,
    compiler_target: RecipeV4CompilerTarget,
    compiler_version: String,
    capability_cell: CapabilityCellReferenceV2,
    plan_sha256: String,
    analytical_identity_sha256: String,
}

impl RecipeV4CompilationReceipt {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn recipe_id(&self) -> Uuid {
        self.recipe_id
    }

    pub fn recipe_document_sha256(&self) -> &str {
        &self.recipe_document_sha256
    }

    pub fn recipe_analytical_sha256(&self) -> &str {
        &self.recipe_analytical_sha256
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn model_document_sha256(&self) -> &str {
        &self.model_document_sha256
    }

    pub fn model_scientific_sha256(&self) -> &str {
        &self.model_scientific_sha256
    }

    pub fn dataset_fingerprint(&self) -> &str {
        &self.dataset_fingerprint
    }

    pub fn compiler_target(&self) -> RecipeV4CompilerTarget {
        self.compiler_target
    }

    pub fn compiler_version(&self) -> &str {
        &self.compiler_version
    }

    pub fn capability_cell(&self) -> &CapabilityCellReferenceV2 {
        &self.capability_cell
    }

    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub fn analytical_identity_sha256(&self) -> &str {
        &self.analytical_identity_sha256
    }

    fn ensure_valid(&self) -> Result<(), RecipeV4CompilationError> {
        if self.schema_version != RECIPE_V4_COMPILATION_RECEIPT_SCHEMA_VERSION {
            return Err(RecipeV4CompilationError::ReceiptSchema(self.schema_version));
        }
        for (field, value) in [
            (
                "recipe_document_sha256",
                self.recipe_document_sha256.as_str(),
            ),
            (
                "recipe_analytical_sha256",
                self.recipe_analytical_sha256.as_str(),
            ),
            ("model_document_sha256", self.model_document_sha256.as_str()),
            (
                "model_scientific_sha256",
                self.model_scientific_sha256.as_str(),
            ),
            ("plan_sha256", self.plan_sha256.as_str()),
            (
                "analytical_identity_sha256",
                self.analytical_identity_sha256.as_str(),
            ),
        ] {
            if !is_lowercase_sha256(value) {
                return Err(RecipeV4CompilationError::InvalidReceiptSha256 { field });
            }
        }
        if self.model_id.trim().is_empty() || self.dataset_fingerprint.trim().is_empty() {
            return Err(RecipeV4CompilationError::InvalidReceiptIdentity);
        }
        if self.compiler_version != self.compiler_target.compiler_version() {
            return Err(RecipeV4CompilationError::CompilerVersionMismatch);
        }
        ensure_receipt_capability_cell(self.compiler_target, &self.capability_cell)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledAnalysisRecipeV4 {
    plan: CompiledRecipePlanV4,
    receipt: RecipeV4CompilationReceipt,
}

impl CompiledAnalysisRecipeV4 {
    pub fn plan(&self) -> &CompiledRecipePlanV4 {
        &self.plan
    }

    pub fn receipt(&self) -> &RecipeV4CompilationReceipt {
        &self.receipt
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum RecipeV4CompilationError {
    #[error(transparent)]
    InvalidRecipe(#[from] AnalysisRecipeV4Error),
    #[error(transparent)]
    InvalidSemModel(#[from] SemModelV4ValidationError),
    #[error("recipe schema v4 model reference is unresolved")]
    UnresolvedModelReference,
    #[error("legacy_estimand_unspecified recipes cannot be compiled")]
    LegacyEstimandUnspecified,
    #[error("the explicitly resolved model differs from the embedded SemModelV4 payload")]
    EmbeddedModelMismatch,
    #[error("resolved model identifier {actual} does not match recipe model {expected}")]
    ModelIdMismatch { expected: String, actual: String },
    #[error("resolved model scientific digest does not match the recipe binding")]
    ModelScientificDigestMismatch,
    #[error("recipe contains executable legacy metadata key {0}")]
    ExecutableMetadataConflict(String),
    #[error("recipe settings are invalid for immutable recipe-v4 compilation: {0}")]
    InvalidSettings(String),
    #[error(transparent)]
    WeightDeclaration(#[from] WeightDeclarationResolutionErrorV1),
    #[error(transparent)]
    WeightCapability(#[from] WeightCapabilityIssueV1),
    #[error(transparent)]
    WeightCapabilityContract(#[from] WeightCapabilityIssueContractErrorV1),
    #[error("recipe method configuration is required for compilation")]
    MissingMethodConfig,
    #[error("recipe-v4 PLS score-execution contract is invalid: {0}")]
    PlsScoreExecutionContract(String),
    #[error("recipe-v4 PLS nonlinear-effects contract is invalid: {0}")]
    PlsNonlinearEffectsContract(String),
    #[error(
        "recipe method/config {method}/{config_kind} does not match compiler target {target:?}"
    )]
    MethodCompilerMismatch {
        target: RecipeV4CompilerTarget,
        method: AnalysisMethod,
        config_kind: String,
    },
    #[error("CB-SEM recipe settings do not match the SemModelV4 data/model shape")]
    CbsemConfigurationMismatch,
    #[error("Recipe-v4 missing-data settings do not match the SemModelV4 data binding")]
    MissingDataPolicyMismatch,
    #[error("continuous raw mean replacement is outside the bounded v1 scope: {0}")]
    MeanReplacementScope(String),
    #[error("recipe estimand {estimand:?} is incompatible with compiler target {target:?}")]
    EstimandCompilerMismatch {
        target: RecipeV4CompilerTarget,
        estimand: LegacyEstimandConfirmationV4,
    },
    #[error("SemModelV4 variable estimands are incompatible with compiler target {0:?}")]
    ModelEstimandMismatch(RecipeV4CompilerTarget),
    #[error("capability-cell reference does not identify the exact compiler capability")]
    CapabilityCellMismatch,
    #[error(transparent)]
    PlsCompiler(#[from] CompiledPlsPlanV2Error),
    #[error(transparent)]
    CbsemCompiler(#[from] CompiledCbsemPlanV2Error),
    #[error(transparent)]
    CbsemEstimatorCapability(#[from] CbsemEstimatorCapabilityErrorV2),
    #[error(transparent)]
    CbsemProductIndicatorCompiler(#[from] CompiledCbsemProductIndicatorErrorV1),
    #[error("compilation receipt schema must equal 1 (found {0})")]
    ReceiptSchema(u32),
    #[error("compilation receipt {field} is not a lowercase SHA-256 digest")]
    InvalidReceiptSha256 { field: &'static str },
    #[error("compilation receipt has an empty model or dataset identity")]
    InvalidReceiptIdentity,
    #[error("compilation receipt compiler version does not match its target")]
    CompilerVersionMismatch,
    #[error("compiled recipe plan target and receipt target differ")]
    PlanReceiptTargetMismatch,
    #[error("compiled recipe artifact differs from deterministic recompilation")]
    ArtifactMismatch,
    #[error("recipe-v4 compilation value could not be serialized: {0}")]
    Serialization(String),
}

pub fn compile_analysis_recipe_v4(
    recipe: &AnalysisRecipeV4,
    resolved_model: Option<&SemModelV4>,
    target: RecipeV4CompilerTarget,
    capability_cell: CapabilityCellReferenceV2,
) -> Result<CompiledAnalysisRecipeV4, RecipeV4CompilationError> {
    recipe.ensure_valid()?;
    ensure_capability_cell_for_recipe(target, recipe, &capability_cell)?;
    reject_executable_metadata(recipe)?;
    validate_common_settings(recipe)?;
    let model = resolve_recipe_model(recipe, resolved_model)?;
    ensure_weight_capability_v1(recipe, model, target)?;
    ensure_missing_data_policy_compatibility(recipe, model)?;
    ensure_estimand_compatibility(recipe, model, target)?;
    ensure_method_compatibility(recipe, model, target)?;

    let plan = match target {
        RecipeV4CompilerTarget::PlsPlanV2 => {
            let plan = compile_pls_plan_v2(model)?;
            ensure_pls_nonlinear_effects_compatibility(recipe, &plan)?;
            CompiledRecipePlanV4::PlsPlanV2 { plan }
        }
        RecipeV4CompilerTarget::CbsemPlanV2 => {
            let plan = compile_cbsem_plan_v2(model)?;
            let profile = if matches!(
                plan.input(),
                crate::CompiledCbsemInputV2::Raw {
                    missing_data: MissingDataPolicyV4::MeanReplacement,
                    ..
                }
            ) {
                CbsemMlExactCapabilityProfileV2::RawContinuousMeanReplacement
            } else if matches!(
                recipe.method_config.as_ref(),
                Some(MethodConfig::Cbsem {
                    mean_structure: true,
                    ..
                })
            ) {
                CbsemMlExactCapabilityProfileV2::RawCfaMeanStructure
            } else {
                CbsemMlExactCapabilityProfileV2::CovarianceStructure
            };
            ensure_cbsem_ml_exact_parameter_table_v4_capability_v2(&plan, profile)?;
            CompiledRecipePlanV4::CbsemPlanV2 { plan }
        }
        RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1 => {
            let source_plan = compile_cbsem_plan_v2(model)?;
            let plan = compile_cbsem_product_indicator_plan_v1(&source_plan)?;
            ensure_cbsem_ml_exact_parameter_table_v4_capability_v2(
                plan.expanded_plan(),
                CbsemMlExactCapabilityProfileV2::CovarianceStructure,
            )?;
            CompiledRecipePlanV4::CbsemProductIndicatorPlanV1 { plan }
        }
    };
    if let CompiledRecipePlanV4::PlsPlanV2 { plan } = &plan {
        ensure_pls_score_execution_compatibility(recipe, plan)?;
    }
    let recipe_document_sha256 = hash_serializable(recipe)?;
    let recipe_analytical_sha256 = recipe_analytical_sha256(recipe, model)?;
    let model_document_sha256 = hash_serializable(model)?;
    let model_scientific_sha256 = model.scientific_sha256()?;
    let plan_sha256 = hash_serializable(&plan)?;
    let analytical_identity_sha256 = hash_serializable(&CompilationAnalyticalIdentityV4 {
        receipt_schema_version: RECIPE_V4_COMPILATION_RECEIPT_SCHEMA_VERSION,
        recipe_id: recipe.id,
        recipe_analytical_sha256: &recipe_analytical_sha256,
        model_id: &model.id,
        model_scientific_sha256: &model_scientific_sha256,
        compiler_target: target,
        compiler_version: target.compiler_version(),
        capability_cell: &capability_cell,
        plan_sha256: &plan_sha256,
    })?;
    let receipt = RecipeV4CompilationReceipt {
        schema_version: RECIPE_V4_COMPILATION_RECEIPT_SCHEMA_VERSION,
        recipe_id: recipe.id,
        recipe_document_sha256,
        recipe_analytical_sha256,
        model_id: model.id.clone(),
        model_document_sha256,
        model_scientific_sha256,
        dataset_fingerprint: recipe.dataset_fingerprint.clone(),
        compiler_target: target,
        compiler_version: target.compiler_version().into(),
        capability_cell,
        plan_sha256,
        analytical_identity_sha256,
    };
    receipt.ensure_valid()?;
    Ok(CompiledAnalysisRecipeV4 { plan, receipt })
}

/// Archive-validation boundary: every stored plan and receipt is regenerated
/// from the exact recipe and explicitly resolved model before it is trusted.
pub fn validate_compiled_analysis_recipe_v4(
    artifact: &CompiledAnalysisRecipeV4,
    recipe: &AnalysisRecipeV4,
    resolved_model: Option<&SemModelV4>,
) -> Result<(), RecipeV4CompilationError> {
    artifact.receipt.ensure_valid()?;
    if artifact.plan.target() != artifact.receipt.compiler_target {
        return Err(RecipeV4CompilationError::PlanReceiptTargetMismatch);
    }
    let expected = compile_analysis_recipe_v4(
        recipe,
        resolved_model,
        artifact.receipt.compiler_target,
        artifact.receipt.capability_cell.clone(),
    )?;
    if expected != *artifact {
        return Err(RecipeV4CompilationError::ArtifactMismatch);
    }
    Ok(())
}

fn resolve_recipe_model<'a>(
    recipe: &'a AnalysisRecipeV4,
    resolved_model: Option<&'a SemModelV4>,
) -> Result<&'a SemModelV4, RecipeV4CompilationError> {
    if matches!(
        &recipe.model_binding,
        AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified { .. }
    ) {
        return Err(RecipeV4CompilationError::LegacyEstimandUnspecified);
    }
    let Some(resolved_model) = resolved_model else {
        return Err(RecipeV4CompilationError::UnresolvedModelReference);
    };
    resolved_model.ensure_valid()?;
    match &recipe.model_binding {
        AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified { .. } => unreachable!(),
        AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model,
            scientific_sha256,
        } => {
            if model != resolved_model {
                return Err(RecipeV4CompilationError::EmbeddedModelMismatch);
            }
            if resolved_model.scientific_sha256()? != *scientific_sha256 {
                return Err(RecipeV4CompilationError::ModelScientificDigestMismatch);
            }
            Ok(resolved_model)
        }
        AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id,
            scientific_sha256,
        } => {
            if resolved_model.id != *model_id {
                return Err(RecipeV4CompilationError::ModelIdMismatch {
                    expected: model_id.clone(),
                    actual: resolved_model.id.clone(),
                });
            }
            if resolved_model.scientific_sha256()? != *scientific_sha256 {
                return Err(RecipeV4CompilationError::ModelScientificDigestMismatch);
            }
            Ok(resolved_model)
        }
    }
}

fn reject_executable_metadata(recipe: &AnalysisRecipeV4) -> Result<(), RecipeV4CompilationError> {
    if let Some(key) = recipe
        .metadata
        .keys()
        .find(|key| EXECUTABLE_LEGACY_METADATA_KEYS.contains(&key.as_str()))
    {
        return Err(RecipeV4CompilationError::ExecutableMetadataConflict(
            key.clone(),
        ));
    }
    Ok(())
}

fn validate_common_settings(recipe: &AnalysisRecipeV4) -> Result<(), RecipeV4CompilationError> {
    let settings = &recipe.settings;
    if !settings.tolerance.is_finite() || settings.tolerance <= 0.0 {
        return Err(RecipeV4CompilationError::InvalidSettings(
            "tolerance must be finite and greater than zero".into(),
        ));
    }
    if settings.max_iterations == 0 {
        return Err(RecipeV4CompilationError::InvalidSettings(
            "max_iterations must be greater than zero".into(),
        ));
    }
    if !settings.confidence_level.is_finite() || !(0.0..1.0).contains(&settings.confidence_level) {
        return Err(RecipeV4CompilationError::InvalidSettings(
            "confidence_level must be finite and strictly between zero and one".into(),
        ));
    }
    if settings.workers == 0 {
        return Err(RecipeV4CompilationError::InvalidSettings(
            "workers must be greater than zero".into(),
        ));
    }
    if settings
        .case_weight_column
        .as_deref()
        .is_some_and(str::is_empty)
    {
        return Err(RecipeV4CompilationError::InvalidSettings(
            "case_weight_column cannot be empty".into(),
        ));
    }
    Ok(())
}

fn ensure_weight_capability_v1(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    target: RecipeV4CompilerTarget,
) -> Result<(), RecipeV4CompilationError> {
    let declaration = resolve_weight_declaration_v1(model)?;
    let capability_target = match target {
        RecipeV4CompilerTarget::PlsPlanV2 => WeightCapabilityTargetV1::PlsPlanV2,
        RecipeV4CompilerTarget::CbsemPlanV2 => {
            if matches!(
                &model.data_binding,
                SemDataBindingV4::Raw {
                    missing_data: MissingDataPolicyV4::MeanReplacement,
                    ..
                }
            ) {
                WeightCapabilityTargetV1::CbsemMlMeanReplacementV1
            } else {
                WeightCapabilityTargetV1::CbsemMlV1
            }
        }
        RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1 => {
            WeightCapabilityTargetV1::CbsemProductIndicatorPlanV1
        }
    };

    if let Some(legacy_column) = recipe.settings.case_weight_column.as_deref() {
        return Err(
            WeightCapabilityIssueV1::legacy_case_weight_binding_ambiguous(
                capability_target,
                legacy_column,
                declaration,
            )?
            .into(),
        );
    }

    if let Some(declaration) = declaration {
        return Err(WeightCapabilityIssueV1::unsupported(capability_target, declaration).into());
    }
    Ok(())
}

fn ensure_missing_data_policy_compatibility(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
) -> Result<(), RecipeV4CompilationError> {
    let coherent = match (&model.data_binding, &recipe.settings.missing_data) {
        (
            SemDataBindingV4::Raw {
                missing_data: MissingDataPolicyV4::ListwiseDeletion,
                ..
            },
            MissingDataPolicy::ListwiseDeletion,
        )
        | (
            SemDataBindingV4::Raw {
                missing_data: MissingDataPolicyV4::MeanReplacement,
                ..
            },
            MissingDataPolicy::MeanReplacement,
        ) => true,
        (
            SemDataBindingV4::Covariance { .. } | SemDataBindingV4::Correlation { .. },
            MissingDataPolicy::ListwiseDeletion,
        ) => true,
        _ => false,
    };
    if coherent {
        Ok(())
    } else {
        Err(RecipeV4CompilationError::MissingDataPolicyMismatch)
    }
}

fn ensure_estimand_compatibility(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    target: RecipeV4CompilerTarget,
) -> Result<(), RecipeV4CompilationError> {
    let declared_mismatch = matches!(
        (target, recipe.estimand_confirmation),
        (
            RecipeV4CompilerTarget::PlsPlanV2,
            LegacyEstimandConfirmationV4::ConfirmedCommonFactor
        ) | (
            RecipeV4CompilerTarget::CbsemPlanV2
                | RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1,
            LegacyEstimandConfirmationV4::ConfirmedComposite
        ) | (_, LegacyEstimandConfirmationV4::LegacyEstimandUnspecified)
    );
    if declared_mismatch {
        return Err(RecipeV4CompilationError::EstimandCompilerMismatch {
            target,
            estimand: recipe.estimand_confirmation,
        });
    }
    let composites = model
        .variables
        .iter()
        .filter(|variable| matches!(variable, SemVariableV4::Composite { .. }))
        .count();
    let factors = model
        .variables
        .iter()
        .filter(|variable| matches!(variable, SemVariableV4::CommonFactor { .. }))
        .count();
    let compatible = match target {
        RecipeV4CompilerTarget::PlsPlanV2 => composites > 0 && factors == 0,
        RecipeV4CompilerTarget::CbsemPlanV2
        | RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1 => factors > 0 && composites == 0,
    };
    if !compatible {
        return Err(RecipeV4CompilationError::ModelEstimandMismatch(target));
    }
    Ok(())
}

fn ensure_method_compatibility(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    target: RecipeV4CompilerTarget,
) -> Result<(), RecipeV4CompilationError> {
    let Some(config) = recipe.method_config.as_ref() else {
        return Err(RecipeV4CompilationError::MissingMethodConfig);
    };
    let compatible = match target {
        RecipeV4CompilerTarget::PlsPlanV2 => {
            matches!(
                (recipe.settings.method, config),
                (
                    AnalysisMethod::PlsPm,
                    MethodConfig::PlsAlgorithm | MethodConfig::PlsAlgorithmConfiguredV2(_)
                ) | (
                    AnalysisMethod::NonlinearEffects,
                    MethodConfig::NonlinearEffects
                )
            ) && (recipe.settings.method != AnalysisMethod::NonlinearEffects
                || matches!(
                    recipe.settings.weighting_scheme,
                    crate::WeightingScheme::Path | crate::WeightingScheme::Factor
                ))
                && recipe.settings.bootstrap_samples == 0
                && recipe.settings.studentized_inner_samples == 0
                && recipe.settings.permutation_samples == 0
        }
        RecipeV4CompilerTarget::CbsemPlanV2 => {
            let point_only = recipe.settings.bootstrap_samples == 0
                && recipe.settings.studentized_inner_samples == 0
                && recipe.settings.permutation_samples == 0
                && matches!(
                    config,
                    MethodConfig::Cbsem {
                        estimator: CbsemEstimator::Ml,
                        bootstrap_samples: 0,
                        bootstrap_v2: None,
                        group_column: None,
                        invariance_steps,
                        ..
                    } if invariance_steps.is_empty()
                );
            recipe.settings.method == AnalysisMethod::Cbsem
                && (point_only || is_exact_cbsem_cfa_case_bootstrap_v2(recipe, model, config))
        }
        RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1 => {
            recipe.settings.method == AnalysisMethod::Cbsem
                && recipe.settings.bootstrap_samples == 0
                && recipe.settings.studentized_inner_samples == 0
                && recipe.settings.permutation_samples == 0
                && recipe.settings.max_iterations == CBSEM_PRODUCT_INDICATOR_MAX_ITERATIONS_V1
                && recipe.settings.tolerance.to_bits()
                    == CBSEM_PRODUCT_INDICATOR_TOLERANCE_V1.to_bits()
                && matches!(
                    config,
                    MethodConfig::Cbsem {
                        model_type: CbsemModelType::Sem,
                        estimator: CbsemEstimator::Ml,
                        input: CbsemInput::Raw,
                        mean_structure: false,
                        bootstrap_samples: 0,
                        bootstrap_v2: None,
                        group_column: None,
                        invariance_steps,
                    } if invariance_steps.is_empty()
                )
        }
    };
    let exact_cbsem_bootstrap_v2_requested = target == RecipeV4CompilerTarget::CbsemPlanV2
        && matches!(
            config,
            MethodConfig::Cbsem {
                bootstrap_v2: Some(_),
                ..
            }
        );
    if recipe.settings.missing_data == MissingDataPolicy::MeanReplacement
        && !exact_cbsem_bootstrap_v2_requested
    {
        let exact_scope = target == RecipeV4CompilerTarget::CbsemPlanV2
            && recipe.settings.preprocessing == crate::Preprocessing::Unstandardized
            && recipe.settings.bootstrap_samples == 0
            && recipe.settings.studentized_inner_samples == 0
            && recipe.settings.permutation_samples == 0
            && matches!(
                config,
                MethodConfig::Cbsem {
                    estimator: CbsemEstimator::Ml,
                    input: CbsemInput::Raw,
                    mean_structure: false,
                    bootstrap_samples: 0,
                    bootstrap_v2: None,
                    group_column: None,
                    invariance_steps,
                    ..
                } if invariance_steps.is_empty()
            );
        if !exact_scope {
            return Err(RecipeV4CompilationError::MeanReplacementScope(
                "requires CB-SEM raw ML covariance-structure point estimation with unstandardized preprocessing and no resampling or grouping options"
                    .into(),
            ));
        }
    }
    if !compatible {
        return Err(RecipeV4CompilationError::MethodCompilerMismatch {
            target,
            method: recipe.settings.method,
            config_kind: config.kind().into(),
        });
    }
    if matches!(
        target,
        RecipeV4CompilerTarget::CbsemPlanV2 | RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1
    ) {
        let MethodConfig::Cbsem {
            model_type, input, ..
        } = config
        else {
            unreachable!("CB-SEM compatibility matched above")
        };
        let expected_model_type = if model
            .relations
            .iter()
            .any(|relation| matches!(relation, crate::SemRelationV4::Structural { .. }))
        {
            CbsemModelType::Sem
        } else {
            CbsemModelType::Cfa
        };
        let expected_input = match model.data_binding {
            SemDataBindingV4::Raw { .. } => CbsemInput::Raw,
            SemDataBindingV4::Covariance { .. } => CbsemInput::Covariance,
            SemDataBindingV4::Correlation { .. } => CbsemInput::Correlation,
        };
        if *model_type != expected_model_type || *input != expected_input {
            return Err(RecipeV4CompilationError::CbsemConfigurationMismatch);
        }
    }
    Ok(())
}

fn is_exact_cbsem_cfa_case_bootstrap_v2(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    config: &MethodConfig,
) -> bool {
    let selector_matches = matches!(
        config,
        MethodConfig::Cbsem {
            model_type: CbsemModelType::Cfa,
            estimator: CbsemEstimator::Ml,
            input: CbsemInput::Raw,
            mean_structure: false,
            bootstrap_samples,
            bootstrap_v2: Some(CbsemBootstrapConfigV2 {
                algorithm: CbsemBootstrapAlgorithm::CaseResamplingFullMl,
                interval:
                    CbsemBootstrapInterval::PercentileType7
                    | CbsemBootstrapInterval::AnalyticStudentizedType7
                    | CbsemBootstrapInterval::BcaType7,
                test_tail: _,
            }),
            group_column: None,
            invariance_steps,
        } if invariance_steps.is_empty()
            && (500..=10_000).contains(bootstrap_samples)
            && *bootstrap_samples == recipe.settings.bootstrap_samples
    );
    let raw_listwise_unclustered = matches!(
        &model.data_binding,
        SemDataBindingV4::Raw {
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
            ..
        }
    );
    let covariance_cfa = !model
        .relations
        .iter()
        .any(|relation| matches!(relation, crate::SemRelationV4::Structural { .. }))
        && !model.parameters.iter().any(|parameter| {
            matches!(
                parameter.target(),
                crate::SemParameterTargetV4::Intercept { .. }
                    | crate::SemParameterTargetV4::Mean { .. }
            )
        });
    let single_group_without_overrides = matches!(model.group, crate::SemGroupV4::SingleGroup)
        && model
            .parameters
            .iter()
            .all(|parameter| parameter.group_overrides().is_empty());

    selector_matches
        && recipe.settings.missing_data == MissingDataPolicy::ListwiseDeletion
        && recipe.settings.studentized_inner_samples == 0
        && recipe.settings.permutation_samples == 0
        && recipe.settings.case_weight_column.is_none()
        && recipe.settings.confidence_level.to_bits() == 0.95_f64.to_bits()
        && raw_listwise_unclustered
        && covariance_cfa
        && single_group_without_overrides
        && model.derived_terms.is_empty()
}

fn ensure_pls_score_execution_compatibility(
    recipe: &AnalysisRecipeV4,
    plan: &CompiledPlsPlanV2,
) -> Result<(), RecipeV4CompilationError> {
    let configured = match recipe.method_config.as_ref() {
        Some(MethodConfig::PlsAlgorithmConfiguredV2(config)) => Some(config),
        Some(MethodConfig::PlsAlgorithm | MethodConfig::NonlinearEffects) => None,
        _ => return Ok(()),
    };
    let has_fixed_scoring = plan
        .blocks()
        .iter()
        .any(|block| block.fixed_scoring().is_some());
    if configured.is_none() && !has_fixed_scoring {
        return Ok(());
    }

    if recipe.settings.max_iterations != crate::PLS_ALGORITHM_FIXED_MAX_ITERATIONS
        || recipe.settings.tolerance.to_bits()
            != crate::PLS_ALGORITHM_FIXED_STOP_CRITERION.to_bits()
        || !matches!(
            recipe.settings.weighting_scheme,
            crate::WeightingScheme::Path | crate::WeightingScheme::Factor
        )
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.bootstrap_samples != 0
        || recipe.settings.studentized_inner_samples != 0
        || recipe.settings.permutation_samples != 0
    {
        return Err(RecipeV4CompilationError::PlsScoreExecutionContract(
            "configured or fixed scoring requires Path/Factor point estimation with exact 3000 iterations and a 1e-7 stop criterion"
                .into(),
        ));
    }

    let Some(configured) = configured else {
        return Ok(());
    };
    if !configured.has_exact_contract_version() {
        return Err(RecipeV4CompilationError::PlsScoreExecutionContract(
            "configured initialization contract version differs from pls_initial_outer_weights_v2"
                .into(),
        ));
    }
    let estimated_blocks = plan
        .blocks()
        .iter()
        .filter(|block| block.fixed_scoring().is_none())
        .collect::<Vec<_>>();
    if estimated_blocks.is_empty() {
        return Err(RecipeV4CompilationError::PlsScoreExecutionContract(
            "configured initialization requires at least one estimated score block".into(),
        ));
    }
    let crate::PlsInitialOuterWeightsV2::Individual { weights } = &configured.initial_outer_weights
    else {
        return Ok(());
    };

    let expected = estimated_blocks
        .iter()
        .flat_map(|block| {
            block.indicators().iter().map(|indicator| {
                (
                    block.construct_id().to_owned(),
                    indicator.variable_id().to_owned(),
                )
            })
        })
        .collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();
    let mut nonzero_blocks = BTreeSet::new();
    let mut previous: Option<(&str, &str)> = None;
    for weight in weights {
        let key = (weight.construct_id.as_str(), weight.indicator_id.as_str());
        if previous.is_some_and(|previous| previous >= key) {
            return Err(RecipeV4CompilationError::PlsScoreExecutionContract(
                "individual initial weights must be strictly sorted by stable construct ID then stable indicator ID"
                    .into(),
            ));
        }
        previous = Some(key);
        if !weight.value.is_finite() {
            return Err(RecipeV4CompilationError::PlsScoreExecutionContract(
                "individual initial weights must be finite".into(),
            ));
        }
        let owned = (weight.construct_id.clone(), weight.indicator_id.clone());
        if !actual.insert(owned) {
            return Err(RecipeV4CompilationError::PlsScoreExecutionContract(
                "individual initial weights contain a duplicate stable indicator identity".into(),
            ));
        }
        if weight.value != 0.0 {
            nonzero_blocks.insert(weight.construct_id.as_str());
        }
    }
    if actual != expected {
        return Err(RecipeV4CompilationError::PlsScoreExecutionContract(
            "individual initial weights must cover every estimated stable indicator identity exactly once and must not reference fixed blocks"
                .into(),
        ));
    }
    if estimated_blocks
        .iter()
        .any(|block| !nonzero_blocks.contains(block.construct_id()))
    {
        return Err(RecipeV4CompilationError::PlsScoreExecutionContract(
            "individual initialization cannot be all zero within an estimated block".into(),
        ));
    }
    Ok(())
}

fn ensure_pls_nonlinear_effects_compatibility(
    recipe: &AnalysisRecipeV4,
    plan: &CompiledPlsPlanV2,
) -> Result<(), RecipeV4CompilationError> {
    if recipe.settings.method != AnalysisMethod::NonlinearEffects {
        return Ok(());
    }
    if plan.paths().is_empty() {
        return Err(RecipeV4CompilationError::PlsNonlinearEffectsContract(
            "at least one compiled structural predecessor is required".into(),
        ));
    }
    if plan
        .blocks()
        .iter()
        .any(|block| block.fixed_scoring().is_some())
    {
        return Err(RecipeV4CompilationError::PlsNonlinearEffectsContract(
            "unit/custom fixed-scoring bases are outside the method-v1 scope".into(),
        ));
    }
    Ok(())
}

fn ensure_capability_cell_for_recipe(
    target: RecipeV4CompilerTarget,
    recipe: &AnalysisRecipeV4,
    actual: &CapabilityCellReferenceV2,
) -> Result<(), RecipeV4CompilationError> {
    let exact_cbsem_bootstrap = target == RecipeV4CompilerTarget::CbsemPlanV2
        && matches!(
            recipe.method_config.as_ref(),
            Some(MethodConfig::Cbsem {
                bootstrap_v2: Some(_),
                ..
            })
        );
    let current = exact_cbsem_bootstrap && *actual == cbsem_exact_bootstrap_capability_cell_v1();
    // Historical exact-bootstrap receipts used the point-ML capability cell.
    // They remain valid under their stored identity, while current desktop
    // requests are required to use the method-scoped bootstrap cell.
    let historical = exact_cbsem_bootstrap && *actual == target.capability_cell();
    if !current && !historical && *actual != target.capability_cell_for_method(recipe.settings.method) {
        return Err(RecipeV4CompilationError::CapabilityCellMismatch);
    }
    Ok(())
}

fn ensure_receipt_capability_cell(
    target: RecipeV4CompilerTarget,
    actual: &CapabilityCellReferenceV2,
) -> Result<(), RecipeV4CompilationError> {
    let base = target.capability_cell();
    let nonlinear = target.capability_cell_for_method(AnalysisMethod::NonlinearEffects);
    let exact_cbsem_bootstrap = cbsem_exact_bootstrap_capability_cell_v1();
    if *actual != base && *actual != nonlinear && *actual != exact_cbsem_bootstrap {
        return Err(RecipeV4CompilationError::CapabilityCellMismatch);
    }
    Ok(())
}

fn recipe_analytical_sha256(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
) -> Result<String, RecipeV4CompilationError> {
    hash_serializable(&RecipeAnalyticalIdentityV4 {
        schema_version: recipe.schema_version,
        recipe_id: recipe.id,
        dataset_fingerprint: &recipe.dataset_fingerprint,
        model_id: &model.id,
        model_scientific_sha256: model.scientific_sha256()?,
        estimand_confirmation: recipe.estimand_confirmation,
        settings: &recipe.settings,
        method_config: recipe.method_config.as_ref(),
        metadata: &recipe.metadata,
    })
}

#[derive(Serialize)]
struct RecipeAnalyticalIdentityV4<'a> {
    schema_version: u32,
    recipe_id: Uuid,
    dataset_fingerprint: &'a str,
    model_id: &'a str,
    model_scientific_sha256: String,
    estimand_confirmation: LegacyEstimandConfirmationV4,
    settings: &'a crate::AnalysisSettings,
    method_config: Option<&'a MethodConfig>,
    metadata: &'a std::collections::BTreeMap<String, String>,
}

#[derive(Serialize)]
struct CompilationAnalyticalIdentityV4<'a> {
    receipt_schema_version: u32,
    recipe_id: Uuid,
    recipe_analytical_sha256: &'a str,
    model_id: &'a str,
    model_scientific_sha256: &'a str,
    compiler_target: RecipeV4CompilerTarget,
    compiler_version: &'a str,
    capability_cell: &'a CapabilityCellReferenceV2,
    plan_sha256: &'a str,
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hash_serializable<T: Serialize>(value: &T) -> Result<String, RecipeV4CompilationError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| RecipeV4CompilationError::Serialization(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisRecipe, AnalysisSettings, Construct,
        ConvertDisplayCovarianceV4, HigherOrderConstructionApproachV4,
        HigherOrderMeasurementTypeV4, InteractionMethodV4, LegacyBasicModelInterpretationV4,
        LegacyDisplayCovarianceV4, MeasurementMode, ModelSpec, ProductIndicatorCenteringV4,
        ProductIndicatorPairingV4, ProductIndicatorSpecificationV4,
        ProductIndicatorStandardizationV4, SamplingWeightNormalizationV4, SemAnnotationV4,
        SemCanvasLineV4, SemDerivedTermV4, SemParameterTargetV4, SemParameterV4, SemPresentationV4,
        SemRelationV4, SemVariableV4, SemWeightBindingV4, StructuralPath, WeightCapabilityCodeV1,
        confirm_legacy_recipe_estimand_v4, convert_display_covariance_to_model_v4,
        convert_legacy_basic_model_v4, migrate_analysis_recipe_to_v4_pending,
    };
    use chrono::{TimeZone, Utc};
    use std::collections::BTreeMap;

    fn legacy_model() -> ModelSpec {
        ModelSpec {
            id: Uuid::from_u128(101),
            name: "Fixture".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["y1".into(), "y2".into()],
                },
            ],
            paths: vec![StructuralPath {
                source: "x".into(),
                target: "y".into(),
            }],
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        }
    }

    fn legacy_recipe(method: AnalysisMethod, config: MethodConfig) -> AnalysisRecipe {
        AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(202),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: "dataset-fingerprint".into(),
            model: legacy_model(),
            settings: AnalysisSettings {
                method,
                ..AnalysisSettings::default()
            },
            method_config: Some(config),
            metadata: BTreeMap::new(),
        }
    }

    fn pls_recipe_and_model() -> (AnalysisRecipeV4, SemModelV4) {
        let source = legacy_recipe(AnalysisMethod::PlsPm, MethodConfig::PlsAlgorithm);
        let pending = migrate_analysis_recipe_to_v4_pending(&source).unwrap();
        let (mut recipe, model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source.model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        (recipe, model)
    }

    fn cbsem_recipe_and_model(with_display_covariance: bool) -> (AnalysisRecipeV4, SemModelV4) {
        let source = legacy_recipe(
            AnalysisMethod::Cbsem,
            MethodConfig::Cbsem {
                model_type: CbsemModelType::Sem,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Raw,
                mean_structure: false,
                bootstrap_samples: 0,
                bootstrap_v2: None,
                group_column: None,
                invariance_steps: Vec::new(),
            },
        );
        let pending = migrate_analysis_recipe_to_v4_pending(&source).unwrap();
        let display = with_display_covariance.then(|| LegacyDisplayCovarianceV4 {
            id: "display-covariance".into(),
            left_construct: "x".into(),
            right_construct: "y".into(),
            label: None,
        });
        let drawings = display.into_iter().collect::<Vec<_>>();
        let (mut recipe, model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source.model,
            &drawings,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
        )
        .unwrap();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        (recipe, model)
    }

    fn cbsem_cfa_recipe_and_model() -> (AnalysisRecipeV4, SemModelV4) {
        let mut source = legacy_recipe(
            AnalysisMethod::Cbsem,
            MethodConfig::Cbsem {
                model_type: CbsemModelType::Cfa,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Raw,
                mean_structure: false,
                bootstrap_samples: 0,
                bootstrap_v2: None,
                group_column: None,
                invariance_steps: Vec::new(),
            },
        );
        source.model.paths.clear();
        source.model.constructs[0].indicators.push("x3".into());
        source.model.constructs[1].indicators.push("y3".into());
        let pending = migrate_analysis_recipe_to_v4_pending(&source).unwrap();
        let (mut recipe, model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source.model,
            &[],
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
        )
        .unwrap();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        (recipe, model)
    }

    fn enable_exact_cbsem_cfa_case_bootstrap_v2(recipe: &mut AnalysisRecipeV4, samples: u32) {
        enable_exact_cbsem_cfa_case_bootstrap_v2_with_interval(
            recipe,
            samples,
            CbsemBootstrapInterval::PercentileType7,
        );
    }

    fn enable_exact_cbsem_cfa_case_bootstrap_v2_with_interval(
        recipe: &mut AnalysisRecipeV4,
        samples: u32,
        interval: CbsemBootstrapInterval,
    ) {
        recipe.settings.bootstrap_samples = samples;
        recipe.settings.confidence_level = 0.95;
        let Some(MethodConfig::Cbsem {
            bootstrap_samples,
            bootstrap_v2,
            ..
        }) = recipe.method_config.as_mut()
        else {
            unreachable!()
        };
        *bootstrap_samples = samples;
        *bootstrap_v2 = Some(CbsemBootstrapConfigV2 {
            algorithm: CbsemBootstrapAlgorithm::CaseResamplingFullMl,
            interval,
            test_tail: crate::CbsemBootstrapTestTail::TwoSided,
        });
    }

    fn embed(recipe: &mut AnalysisRecipeV4, model: &SemModelV4) {
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
    }

    fn bind_weight(model: &mut SemModelV4, weight: SemWeightBindingV4) {
        if !model
            .variables
            .iter()
            .any(|variable| variable.id() == "observed:case_weight")
        {
            model.variables.push(SemVariableV4::Observed {
                id: "observed:case_weight".into(),
                label: "Case weight".into(),
                source_column: "case_weight".into(),
                scale: crate::ObservedScaleV4::Continuous,
                role: crate::ObservedRoleV4::Control,
                categories: Vec::new(),
                value_labels: BTreeMap::new(),
                missing_markers: Vec::new(),
                transformation_lineage: Vec::new(),
            });
        }
        let SemDataBindingV4::Raw {
            weight: configured, ..
        } = &mut model.data_binding
        else {
            unreachable!()
        };
        *configured = Some(weight);
        model.ensure_valid().unwrap();
    }

    fn assert_cbsem_capability_code(
        recipe: &AnalysisRecipeV4,
        model: &SemModelV4,
        expected_code: &str,
    ) {
        let target = RecipeV4CompilerTarget::CbsemPlanV2;
        let error =
            compile_analysis_recipe_v4(recipe, Some(model), target, target.capability_cell())
                .unwrap_err();
        let RecipeV4CompilationError::CbsemEstimatorCapability(error) = error else {
            panic!("expected CB-SEM capability error, found {error:?}")
        };
        assert!(
            error.issues.iter().any(|issue| issue.code == expected_code),
            "missing capability code {expected_code}: {:?}",
            error.issues
        );
    }

    fn assert_cbsem_method_mismatch(recipe: &AnalysisRecipeV4, model: &SemModelV4) {
        let target = RecipeV4CompilerTarget::CbsemPlanV2;
        assert!(matches!(
            compile_analysis_recipe_v4(recipe, Some(model), target, target.capability_cell()),
            Err(RecipeV4CompilationError::MethodCompilerMismatch { .. })
        ));
    }

    fn cbsem_product_indicator_recipe_and_model() -> (AnalysisRecipeV4, SemModelV4) {
        let mut legacy = legacy_model();
        legacy.constructs.push(Construct {
            id: "m".into(),
            name: "M".into(),
            short_name: "M".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["m1".into(), "m2".into()],
        });
        legacy.paths.push(StructuralPath {
            source: "m".into(),
            target: "y".into(),
        });
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        let focal_relation = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == "construct:x" && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        model.variables.push(SemVariableV4::Derived {
            id: "derived:x_by_m".into(),
            label: "X × M".into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: "relation:interaction_effect".into(),
            source: "derived:x_by_m".into(),
            target: "construct:y".into(),
            parameter: "parameter:interaction_effect".into(),
            role: crate::StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:interaction_effect".into(),
            label: "X × M -> Y".into(),
            target: SemParameterTargetV4::Regression {
                source: "derived:x_by_m".into(),
                target: "construct:y".into(),
            },
            start: Some(0.1),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.derived_terms.push(SemDerivedTermV4::Interaction {
            id: "term:x_by_m".into(),
            output: "derived:x_by_m".into(),
            predictor: "construct:x".into(),
            moderator: "construct:m".into(),
            focal_relation,
            method: InteractionMethodV4::ProductIndicator,
            product_indicator: Some(ProductIndicatorSpecificationV4 {
                centering: ProductIndicatorCenteringV4::DoubleMeanCenter,
                standardization: ProductIndicatorStandardizationV4::None,
                pairing: ProductIndicatorPairingV4::AllPairs,
            }),
        });
        model.ensure_valid().unwrap();
        let (mut recipe, _) = cbsem_recipe_and_model(false);
        recipe.settings.max_iterations = CBSEM_PRODUCT_INDICATOR_MAX_ITERATIONS_V1;
        recipe.settings.tolerance = CBSEM_PRODUCT_INDICATOR_TOLERANCE_V1;
        embed(&mut recipe, &model);
        (recipe, model)
    }

    #[test]
    fn presentation_only_changes_preserve_plan_and_receipt_analytical_identity() {
        let (base_recipe, base_model) = pls_recipe_and_model();
        let base = compile_analysis_recipe_v4(
            &base_recipe,
            Some(&base_model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();

        let mut decorated_model = base_model.clone();
        decorated_model.annotations.push(SemAnnotationV4::Caption {
            id: "caption".into(),
            text: "Presentation only".into(),
        });
        decorated_model.presentation = SemPresentationV4::Canvas {
            nodes: Vec::new(),
            edges: Vec::new(),
            shapes: Vec::new(),
            images: Vec::new(),
            lines: vec![SemCanvasLineV4 {
                id: "line".into(),
                x1: 0.0,
                y1: 0.0,
                x2: 1.0,
                y2: 1.0,
                label: None,
                start_marker: None,
                end_marker: None,
                style: BTreeMap::new(),
            }],
            zoom: Some(1.2),
            pan_x: Some(4.0),
            pan_y: Some(5.0),
        };
        decorated_model.ensure_valid().unwrap();
        let mut decorated_recipe = base_recipe.clone();
        embed(&mut decorated_recipe, &decorated_model);
        let decorated = compile_analysis_recipe_v4(
            &decorated_recipe,
            Some(&decorated_model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();

        assert_eq!(base.plan(), decorated.plan());
        assert_eq!(
            base.receipt().analytical_identity_sha256(),
            decorated.receipt().analytical_identity_sha256()
        );
        assert_ne!(
            base.receipt().recipe_document_sha256(),
            decorated.receipt().recipe_document_sha256()
        );
        assert_ne!(
            base.receipt().model_document_sha256(),
            decorated.receipt().model_document_sha256()
        );
    }

    #[test]
    fn nonlinear_effects_compile_to_the_pls_base_plan_with_a_distinct_primary_cell() {
        let (base_recipe, model) = pls_recipe_and_model();
        let base = compile_analysis_recipe_v4(
            &base_recipe,
            Some(&model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();
        let base_bytes = serde_json::to_vec(&base).unwrap();

        let mut nonlinear_recipe = base_recipe.clone();
        nonlinear_recipe.settings.method = AnalysisMethod::NonlinearEffects;
        nonlinear_recipe.method_config = Some(MethodConfig::NonlinearEffects);
        assert_eq!(
            (
                nonlinear_recipe.settings.bootstrap_samples,
                nonlinear_recipe.settings.studentized_inner_samples,
                nonlinear_recipe.settings.permutation_samples,
            ),
            (0, 0, 0)
        );
        let nonlinear_cell = RecipeV4CompilerTarget::PlsPlanV2
            .capability_cell_for_method(AnalysisMethod::NonlinearEffects);
        let nonlinear = compile_analysis_recipe_v4(
            &nonlinear_recipe,
            Some(&model),
            RecipeV4CompilerTarget::PlsPlanV2,
            nonlinear_cell.clone(),
        )
        .unwrap();

        assert_eq!(base.plan(), nonlinear.plan());
        assert_eq!(nonlinear.receipt().capability_cell(), &nonlinear_cell);
        assert_eq!(nonlinear_cell.registry_schema_version, 2);
        assert_eq!(
            nonlinear_cell.capability_id,
            PLS_NONLINEAR_EFFECTS_CAPABILITY_ID
        );
        assert_eq!(nonlinear_cell.cell_id, PLS_NONLINEAR_EFFECTS_CELL_ID);
        assert_eq!(
            nonlinear_cell.capability_version,
            PLS_NONLINEAR_EFFECTS_CAPABILITY_VERSION
        );
        assert_ne!(
            base.receipt().analytical_identity_sha256(),
            nonlinear.receipt().analytical_identity_sha256()
        );
        validate_compiled_analysis_recipe_v4(&nonlinear, &nonlinear_recipe, Some(&model)).unwrap();

        for resampled in [
            {
                let mut recipe = nonlinear_recipe.clone();
                recipe.settings.bootstrap_samples = 1;
                recipe
            },
            {
                let mut recipe = nonlinear_recipe.clone();
                recipe.settings.studentized_inner_samples = 1;
                recipe
            },
            {
                let mut recipe = nonlinear_recipe.clone();
                recipe.settings.permutation_samples = 1;
                recipe
            },
        ] {
            assert!(matches!(
                compile_analysis_recipe_v4(
                    &resampled,
                    Some(&model),
                    RecipeV4CompilerTarget::PlsPlanV2,
                    RecipeV4CompilerTarget::PlsPlanV2
                        .capability_cell_for_method(AnalysisMethod::NonlinearEffects),
                ),
                Err(RecipeV4CompilationError::MethodCompilerMismatch { .. })
            ));
        }

        assert!(matches!(
            compile_analysis_recipe_v4(
                &nonlinear_recipe,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::CapabilityCellMismatch)
        ));
        let mut downgraded = nonlinear_recipe.clone();
        downgraded.method_config = Some(MethodConfig::PlsAlgorithm);
        assert!(matches!(
            compile_analysis_recipe_v4(
                &downgraded,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                nonlinear_cell,
            ),
            Err(RecipeV4CompilationError::MethodCompilerMismatch { .. })
        ));
        let mut missing = nonlinear_recipe.clone();
        missing.method_config = None;
        assert!(matches!(
            compile_analysis_recipe_v4(
                &missing,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2
                    .capability_cell_for_method(AnalysisMethod::NonlinearEffects),
            ),
            Err(RecipeV4CompilationError::MissingMethodConfig)
        ));
        let mut pca = nonlinear_recipe;
        pca.settings.weighting_scheme = crate::WeightingScheme::Pca;
        assert!(matches!(
            compile_analysis_recipe_v4(
                &pca,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2
                    .capability_cell_for_method(AnalysisMethod::NonlinearEffects),
            ),
            Err(RecipeV4CompilationError::MethodCompilerMismatch { .. })
        ));

        assert_eq!(serde_json::to_vec(&base).unwrap(), base_bytes);
    }

    #[test]
    fn nonlinear_effects_reject_empty_derived_and_fixed_score_bases_before_execution() {
        let (base_recipe, base_model) = pls_recipe_and_model();
        let target = RecipeV4CompilerTarget::PlsPlanV2;
        let cell = target.capability_cell_for_method(AnalysisMethod::NonlinearEffects);
        let nonlinear_recipe = |model: &SemModelV4| {
            let mut recipe = base_recipe.clone();
            recipe.settings.method = AnalysisMethod::NonlinearEffects;
            recipe.method_config = Some(MethodConfig::NonlinearEffects);
            embed(&mut recipe, model);
            recipe
        };

        let mut empty = base_model.clone();
        let structural_parameters = empty
            .relations
            .iter()
            .filter_map(|relation| match relation {
                SemRelationV4::Structural { parameter, .. } => Some(parameter.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        empty
            .relations
            .retain(|relation| !matches!(relation, SemRelationV4::Structural { .. }));
        empty
            .parameters
            .retain(|parameter| !structural_parameters.contains(parameter.id()));
        empty.ensure_valid().unwrap();
        let empty_recipe = nonlinear_recipe(&empty);
        assert!(matches!(
            compile_analysis_recipe_v4(&empty_recipe, Some(&empty), target, cell.clone()),
            Err(RecipeV4CompilationError::PlsNonlinearEffectsContract(message))
                if message.contains("structural predecessor")
        ));

        let mut interaction_source = legacy_model();
        interaction_source.constructs.push(Construct {
            id: "m".into(),
            name: "Moderator".into(),
            short_name: "M".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["m1".into(), "m2".into()],
        });
        interaction_source.paths.push(StructuralPath {
            source: "m".into(),
            target: "y".into(),
        });
        let mut interaction = convert_legacy_basic_model_v4(
            &interaction_source,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let focal_relation = interaction
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == "construct:x" && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        interaction.variables.push(SemVariableV4::Derived {
            id: "derived:x-by-m".into(),
            label: "X x M".into(),
        });
        interaction.relations.push(SemRelationV4::Structural {
            id: "relation:interaction-effect".into(),
            source: "derived:x-by-m".into(),
            target: "construct:y".into(),
            parameter: "parameter:interaction-effect".into(),
            role: crate::StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        interaction.parameters.push(SemParameterV4::Free {
            id: "parameter:interaction-effect".into(),
            label: "X x M -> Y".into(),
            target: SemParameterTargetV4::Regression {
                source: "derived:x-by-m".into(),
                target: "construct:y".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        interaction
            .derived_terms
            .push(SemDerivedTermV4::Interaction {
                id: "term:x-by-m".into(),
                output: "derived:x-by-m".into(),
                predictor: "construct:x".into(),
                moderator: "construct:m".into(),
                focal_relation,
                method: InteractionMethodV4::TwoStage,
                product_indicator: None,
            });
        interaction.ensure_valid().unwrap();
        let interaction_recipe = nonlinear_recipe(&interaction);
        assert!(matches!(
            compile_analysis_recipe_v4(
                &interaction_recipe,
                Some(&interaction),
                target,
                cell.clone(),
            ),
            Err(RecipeV4CompilationError::PlsCompiler(
                CompiledPlsPlanV2Error::Unsupported { code, .. }
            )) if code == "derived_terms"
        ));

        let mut higher_order = base_model.clone();
        higher_order.variables.push(SemVariableV4::Derived {
            id: "derived:hoc".into(),
            label: "Higher-order construct".into(),
        });
        higher_order
            .derived_terms
            .push(SemDerivedTermV4::HigherOrder {
                id: "term:hoc".into(),
                output: "derived:hoc".into(),
                components: vec!["construct:x".into(), "construct:y".into()],
                approach: HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators,
                measurement_type: HigherOrderMeasurementTypeV4::ReflectiveFormative,
            });
        higher_order.ensure_valid().unwrap();
        let higher_order_recipe = nonlinear_recipe(&higher_order);
        assert!(matches!(
            compile_analysis_recipe_v4(
                &higher_order_recipe,
                Some(&higher_order),
                target,
                cell.clone(),
            ),
            Err(RecipeV4CompilationError::PlsCompiler(
                CompiledPlsPlanV2Error::Unsupported { code, .. }
            )) if code == "derived_terms"
        ));

        let mut formative = legacy_model();
        formative.constructs[0].mode = MeasurementMode::Formative;
        let fixed_base = convert_legacy_basic_model_v4(
            &formative,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        for weighting in [
            crate::CompositeWeightingV4::Unit {
                normalization: crate::CompositeWeightNormalizationV4::UnitVariance,
            },
            crate::CompositeWeightingV4::Custom {
                weights: BTreeMap::from([
                    ("observed:x1".into(), 0.25),
                    ("observed:x2".into(), 0.75),
                ]),
                normalization: crate::CompositeWeightNormalizationV4::SumToOne,
            },
        ] {
            let mut fixed = fixed_base.clone();
            let SemVariableV4::Composite {
                weighting: configured,
                ..
            } = fixed
                .variables
                .iter_mut()
                .find(|variable| variable.id() == "construct:x")
                .unwrap()
            else {
                unreachable!()
            };
            *configured = weighting;
            fixed.ensure_valid().unwrap();
            let fixed_recipe = nonlinear_recipe(&fixed);
            assert!(matches!(
                compile_analysis_recipe_v4(&fixed_recipe, Some(&fixed), target, cell.clone()),
                Err(RecipeV4CompilationError::PlsNonlinearEffectsContract(message))
                    if message.contains("fixed-scoring")
            ));
        }
    }

    #[test]
    fn configured_and_fixed_score_execution_are_versioned_and_fail_closed() {
        let (mut configured_recipe, model) = pls_recipe_and_model();
        configured_recipe.method_config = Some(MethodConfig::PlsAlgorithmConfiguredV2(
            crate::PlsAlgorithmConfigV2::standard(),
        ));
        let configured = compile_analysis_recipe_v4(
            &configured_recipe,
            Some(&model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();
        assert_eq!(
            configured.receipt().compiler_version(),
            RECIPE_V4_PLS_COMPILER_VERSION
        );
        assert_eq!(
            configured.receipt().compiler_version(),
            "recipe_v4_to_compiled_pls_plan_v2_v2"
        );
        for preprocessing in [
            crate::Preprocessing::MeanCentered,
            crate::Preprocessing::Unstandardized,
        ] {
            for weighting_scheme in [crate::WeightingScheme::Path, crate::WeightingScheme::Factor] {
                let mut widened = configured_recipe.clone();
                widened.settings.preprocessing = preprocessing.clone();
                widened.settings.weighting_scheme = weighting_scheme;
                compile_analysis_recipe_v4(
                    &widened,
                    Some(&model),
                    RecipeV4CompilerTarget::PlsPlanV2,
                    RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
                )
                .unwrap();
            }
        }

        let mut drifted = configured_recipe.clone();
        drifted.settings.max_iterations = 2_999;
        assert!(matches!(
            compile_analysis_recipe_v4(
                &drifted,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::PlsScoreExecutionContract(_))
        ));

        let mut formative = legacy_model();
        formative.constructs[0].mode = MeasurementMode::Formative;
        let mut fixed_model = convert_legacy_basic_model_v4(
            &formative,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        for normalization in [
            crate::CompositeWeightNormalizationV4::None,
            crate::CompositeWeightNormalizationV4::SumToOne,
            crate::CompositeWeightNormalizationV4::UnitVariance,
        ] {
            let SemVariableV4::Composite { weighting, .. } = fixed_model
                .variables
                .iter_mut()
                .find(|variable| variable.id() == "construct:x")
                .unwrap()
            else {
                unreachable!()
            };
            *weighting = crate::CompositeWeightingV4::Unit { normalization };
            let mut fixed_recipe = configured_recipe.clone();
            fixed_recipe.method_config = Some(MethodConfig::PlsAlgorithm);
            embed(&mut fixed_recipe, &fixed_model);
            let compiled = compile_analysis_recipe_v4(
                &fixed_recipe,
                Some(&fixed_model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            )
            .unwrap();
            let CompiledRecipePlanV4::PlsPlanV2 { plan } = compiled.plan() else {
                unreachable!("PLS compiler target must emit a PLS plan")
            };
            assert_eq!(
                plan.blocks()[0].fixed_scoring(),
                Some(&crate::CompiledPlsFixedScoringV2::Unit { normalization })
            );
        }
        for preprocessing in [
            crate::Preprocessing::MeanCentered,
            crate::Preprocessing::Unstandardized,
        ] {
            for weighting_scheme in [crate::WeightingScheme::Path, crate::WeightingScheme::Factor] {
                let mut fixed_recipe = configured_recipe.clone();
                fixed_recipe.method_config = Some(MethodConfig::PlsAlgorithm);
                fixed_recipe.settings.preprocessing = preprocessing.clone();
                fixed_recipe.settings.weighting_scheme = weighting_scheme;
                embed(&mut fixed_recipe, &fixed_model);
                compile_analysis_recipe_v4(
                    &fixed_recipe,
                    Some(&fixed_model),
                    RecipeV4CompilerTarget::PlsPlanV2,
                    RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
                )
                .unwrap();
            }
        }
    }

    #[test]
    fn scientific_covariance_changes_cbsem_plan_and_receipt_identity() {
        let (base_recipe, base_model) = cbsem_recipe_and_model(true);
        let base = compile_analysis_recipe_v4(
            &base_recipe,
            Some(&base_model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        let converted_model = convert_display_covariance_to_model_v4(
            &base_model,
            &ConvertDisplayCovarianceV4 {
                annotation_id: "display-covariance".into(),
                relation_id: "scientific-covariance".into(),
                parameter_id: "scientific-covariance-p".into(),
                parameter_label: "Cov(X,Y)".into(),
                start: Some(0.1),
                lower: None,
                upper: None,
                equality_label: None,
            },
        )
        .unwrap();
        let mut converted_recipe = base_recipe.clone();
        embed(&mut converted_recipe, &converted_model);
        let converted = compile_analysis_recipe_v4(
            &converted_recipe,
            Some(&converted_model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        assert_ne!(base.plan(), converted.plan());
        assert_ne!(
            base.receipt().analytical_identity_sha256(),
            converted.receipt().analytical_identity_sha256()
        );
        let CompiledRecipePlanV4::CbsemPlanV2 { plan } = converted.plan() else {
            unreachable!()
        };
        assert_eq!(plan.covariances().len(), 1);
    }

    #[test]
    fn estimand_and_compiler_mismatches_fail_before_plan_compilation() {
        let (pls_recipe, pls_model) = pls_recipe_and_model();
        assert!(matches!(
            compile_analysis_recipe_v4(
                &pls_recipe,
                Some(&pls_model),
                RecipeV4CompilerTarget::CbsemPlanV2,
                RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::EstimandCompilerMismatch { .. })
        ));
        let (cbsem_recipe, cbsem_model) = cbsem_recipe_and_model(false);
        assert!(matches!(
            compile_analysis_recipe_v4(
                &cbsem_recipe,
                Some(&cbsem_model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::EstimandCompilerMismatch { .. })
        ));
    }

    #[test]
    fn method_mismatch_and_unsupported_scientific_semantics_fail_closed() {
        let (mut wrong_method_recipe, model) = pls_recipe_and_model();
        wrong_method_recipe.settings.method = AnalysisMethod::Cbsem;
        wrong_method_recipe.method_config = Some(MethodConfig::Cbsem {
            model_type: CbsemModelType::Sem,
            estimator: CbsemEstimator::Ml,
            input: CbsemInput::Raw,
            mean_structure: false,
            bootstrap_samples: 0,
            bootstrap_v2: None,
            group_column: None,
            invariance_steps: Vec::new(),
        });
        assert!(matches!(
            compile_analysis_recipe_v4(
                &wrong_method_recipe,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::MethodCompilerMismatch { .. })
        ));

        let (mut recipe, mut model) = pls_recipe_and_model();
        model
            .annotations
            .push(SemAnnotationV4::DisplayOnlyCovariance {
                id: "unsupported-covariance".into(),
                left: "construct:x".into(),
                right: "construct:y".into(),
                label: None,
            });
        model = convert_display_covariance_to_model_v4(
            &model,
            &ConvertDisplayCovarianceV4 {
                annotation_id: "unsupported-covariance".into(),
                relation_id: "unsupported-scientific-covariance".into(),
                parameter_id: "unsupported-scientific-covariance-p".into(),
                parameter_label: "Cov(X,Y)".into(),
                start: None,
                lower: None,
                upper: None,
                equality_label: None,
            },
        )
        .unwrap();
        embed(&mut recipe, &model);
        assert!(matches!(
            compile_analysis_recipe_v4(
                &recipe,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::PlsCompiler(
                CompiledPlsPlanV2Error::Unsupported { .. }
            ))
        ));
    }

    #[test]
    fn exact_cbsem_cfa_case_bootstrap_v2_is_explicit_bounded_and_identity_bound() {
        let (point_recipe, model) = cbsem_cfa_recipe_and_model();
        let target = RecipeV4CompilerTarget::CbsemPlanV2;
        let point = compile_analysis_recipe_v4(
            &point_recipe,
            Some(&model),
            target,
            target.capability_cell(),
        )
        .unwrap();
        let point_wire = serde_json::to_value(&point_recipe).unwrap();
        assert!(point_wire["method_config"].get("bootstrap_v2").is_none());
        assert_eq!(
            point_wire["method_config"]["bootstrap_samples"].as_u64(),
            Some(0)
        );

        for samples in [500, 1_000, 10_000] {
            let mut recipe = point_recipe.clone();
            enable_exact_cbsem_cfa_case_bootstrap_v2(&mut recipe, samples);
            let exact_cell = cbsem_exact_bootstrap_capability_cell_v1();
            let compiled = compile_analysis_recipe_v4(
                &recipe,
                Some(&model),
                target,
                exact_cell.clone(),
            )
            .unwrap();
            assert_eq!(compiled.receipt().capability_cell(), &exact_cell);
            assert_eq!(compiled.plan(), point.plan());
            assert_ne!(
                compiled.receipt().analytical_identity_sha256(),
                point.receipt().analytical_identity_sha256()
            );
            let CompiledRecipePlanV4::CbsemPlanV2 { plan } = compiled.plan() else {
                unreachable!()
            };
            let eligibility = compile_cbsem_exact_case_bootstrap_zero_null_eligibility_v1(plan);
            assert!(!eligibility.is_empty());
            assert!(
                eligibility
                    .windows(2)
                    .all(|pair| pair[0].parameter_id < pair[1].parameter_id)
            );
            assert!(eligibility.iter().any(|row| matches!(
                row.status,
                CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Available
            )));
            assert!(eligibility.iter().any(|row| matches!(
                row.status,
                CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                    reason: CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::NonregularVarianceBoundary
                }
            )));
        }

        assert!(matches!(
            compile_analysis_recipe_v4(
                &point_recipe,
                Some(&model),
                target,
                cbsem_exact_bootstrap_capability_cell_v1(),
            ),
            Err(RecipeV4CompilationError::CapabilityCellMismatch)
        ));

        let mut percentile = point_recipe.clone();
        enable_exact_cbsem_cfa_case_bootstrap_v2(&mut percentile, 1_000);
        let percentile_bytes = serde_json::to_vec(&percentile).unwrap();
        let percentile_compiled =
            compile_analysis_recipe_v4(&percentile, Some(&model), target, target.capability_cell())
                .unwrap();
        let mut studentized = point_recipe.clone();
        enable_exact_cbsem_cfa_case_bootstrap_v2_with_interval(
            &mut studentized,
            1_000,
            CbsemBootstrapInterval::AnalyticStudentizedType7,
        );
        let studentized_compiled = compile_analysis_recipe_v4(
            &studentized,
            Some(&model),
            target,
            target.capability_cell(),
        )
        .unwrap();
        assert_eq!(studentized_compiled.plan(), percentile_compiled.plan());
        assert_ne!(
            studentized_compiled.receipt().analytical_identity_sha256(),
            percentile_compiled.receipt().analytical_identity_sha256()
        );
        assert_eq!(
            serde_json::to_vec(&percentile).unwrap(),
            percentile_bytes,
            "adding the studentized selector must not change percentile recipe bytes"
        );
        assert_eq!(
            serde_json::to_value(&studentized).unwrap()["method_config"]["bootstrap_v2"]["interval"],
            "analytic_studentized_type7"
        );

        let mut bca = point_recipe.clone();
        enable_exact_cbsem_cfa_case_bootstrap_v2_with_interval(
            &mut bca,
            1_000,
            CbsemBootstrapInterval::BcaType7,
        );
        let bca_compiled =
            compile_analysis_recipe_v4(&bca, Some(&model), target, target.capability_cell())
                .unwrap();
        assert_eq!(bca_compiled.plan(), percentile_compiled.plan());
        assert_ne!(
            bca_compiled.receipt().analytical_identity_sha256(),
            percentile_compiled.receipt().analytical_identity_sha256()
        );
        assert_ne!(
            bca_compiled.receipt().analytical_identity_sha256(),
            studentized_compiled.receipt().analytical_identity_sha256()
        );
        assert_eq!(
            serde_json::to_vec(&percentile).unwrap(),
            percentile_bytes,
            "adding the BCa selector must not change percentile recipe bytes"
        );
        assert_eq!(
            serde_json::to_value(&bca).unwrap()["method_config"]["bootstrap_v2"]["interval"],
            "bca_type7"
        );

        let mut generic = point_recipe.clone();
        generic.settings.bootstrap_samples = 1_000;
        let Some(MethodConfig::Cbsem {
            bootstrap_samples, ..
        }) = generic.method_config.as_mut()
        else {
            unreachable!()
        };
        *bootstrap_samples = 1_000;
        assert_cbsem_method_mismatch(&generic, &model);

        for samples in [499, 10_001] {
            let mut outside_bound = point_recipe.clone();
            enable_exact_cbsem_cfa_case_bootstrap_v2(&mut outside_bound, samples);
            assert_cbsem_method_mismatch(&outside_bound, &model);
        }

        let mut count_mismatch = point_recipe.clone();
        enable_exact_cbsem_cfa_case_bootstrap_v2(&mut count_mismatch, 1_000);
        count_mismatch.settings.bootstrap_samples = 1_001;
        assert_cbsem_method_mismatch(&count_mismatch, &model);

        for setting in ["confidence", "studentized", "permutation"] {
            let mut drift = point_recipe.clone();
            enable_exact_cbsem_cfa_case_bootstrap_v2(&mut drift, 1_000);
            match setting {
                "confidence" => drift.settings.confidence_level = 0.90,
                "studentized" => drift.settings.studentized_inner_samples = 99,
                "permutation" => drift.settings.permutation_samples = 99,
                _ => unreachable!(),
            }
            assert_cbsem_method_mismatch(&drift, &model);
        }
    }

    #[test]
    fn exact_cbsem_zero_null_eligibility_uses_compiled_family_and_bound_domain() {
        let (_, mut model) = cbsem_cfa_recipe_and_model();
        let loading_id = model
            .parameters
            .iter()
            .find_map(|parameter| match parameter {
                SemParameterV4::Free {
                    id,
                    target: SemParameterTargetV4::Loading { .. },
                    ..
                } => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        model.constraints.push(SemConstraintV4::Bound {
            id: "zero-null-boundary".into(),
            parameter: loading_id.clone(),
            lower: Some(0.0),
            upper: None,
        });
        model.ensure_valid().unwrap();
        let plan = compile_cbsem_plan_v2(&model).unwrap();
        let eligibility = compile_cbsem_exact_case_bootstrap_zero_null_eligibility_v1(&plan);
        assert!(matches!(
            eligibility
                .iter()
                .find(|row| row.parameter_id == loading_id)
                .unwrap()
                .status,
            CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                reason:
                    CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::ZeroNullOutsideOpenDomain
            }
        ));
        assert!(eligibility.iter().any(|row| matches!(
            row.status,
            CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                reason:
                    CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::NonregularVarianceBoundary
            }
        )));
    }

    #[test]
    fn exact_cbsem_cfa_case_bootstrap_v2_rejects_every_out_of_scope_model_shape() {
        let (base_recipe, base_model) = cbsem_cfa_recipe_and_model();

        let mut mean_structure = base_recipe.clone();
        enable_exact_cbsem_cfa_case_bootstrap_v2(&mut mean_structure, 1_000);
        let Some(MethodConfig::Cbsem {
            mean_structure: enabled,
            ..
        }) = mean_structure.method_config.as_mut()
        else {
            unreachable!()
        };
        *enabled = true;
        assert_cbsem_method_mismatch(&mean_structure, &base_model);

        let mut covariance_input = base_recipe.clone();
        enable_exact_cbsem_cfa_case_bootstrap_v2(&mut covariance_input, 1_000);
        let Some(MethodConfig::Cbsem { input, .. }) = covariance_input.method_config.as_mut()
        else {
            unreachable!()
        };
        *input = CbsemInput::Covariance;
        assert_cbsem_method_mismatch(&covariance_input, &base_model);

        let mut explicit_mean_model = base_model.clone();
        explicit_mean_model.parameters.push(SemParameterV4::Free {
            id: "observed-x1-intercept".into(),
            label: "Intercept(x1)".into(),
            target: SemParameterTargetV4::Intercept {
                variable: "observed:x1".into(),
            },
            start: Some(0.0),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        explicit_mean_model.ensure_valid().unwrap();
        let mut explicit_mean_recipe = base_recipe.clone();
        enable_exact_cbsem_cfa_case_bootstrap_v2(&mut explicit_mean_recipe, 1_000);
        embed(&mut explicit_mean_recipe, &explicit_mean_model);
        assert_cbsem_method_mismatch(&explicit_mean_recipe, &explicit_mean_model);

        let (mut sem_recipe, sem_model) = cbsem_recipe_and_model(false);
        enable_exact_cbsem_cfa_case_bootstrap_v2(&mut sem_recipe, 1_000);
        assert_cbsem_method_mismatch(&sem_recipe, &sem_model);

        let mut mean_replacement_model = base_model.clone();
        let SemDataBindingV4::Raw { missing_data, .. } = &mut mean_replacement_model.data_binding
        else {
            unreachable!()
        };
        *missing_data = MissingDataPolicyV4::MeanReplacement;
        mean_replacement_model.ensure_valid().unwrap();
        let mut mean_replacement_recipe = base_recipe.clone();
        enable_exact_cbsem_cfa_case_bootstrap_v2(&mut mean_replacement_recipe, 1_000);
        mean_replacement_recipe.settings.missing_data = MissingDataPolicy::MeanReplacement;
        embed(&mut mean_replacement_recipe, &mean_replacement_model);
        assert_cbsem_method_mismatch(&mean_replacement_recipe, &mean_replacement_model);

        for binding in ["cluster", "strata"] {
            let mut bound_model = base_model.clone();
            let SemDataBindingV4::Raw {
                cluster_variable,
                strata_variable,
                ..
            } = &mut bound_model.data_binding
            else {
                unreachable!()
            };
            match binding {
                "cluster" => *cluster_variable = Some("observed:x1".into()),
                "strata" => *strata_variable = Some("observed:x1".into()),
                _ => unreachable!(),
            }
            bound_model.ensure_valid().unwrap();
            let mut bound_recipe = base_recipe.clone();
            enable_exact_cbsem_cfa_case_bootstrap_v2(&mut bound_recipe, 1_000);
            embed(&mut bound_recipe, &bound_model);
            assert_cbsem_method_mismatch(&bound_recipe, &bound_model);
        }

        let mut grouped_model = base_model.clone();
        grouped_model.group = crate::SemGroupV4::ObservedGroups {
            grouping_variable: "observed:x1".into(),
            levels: vec![
                crate::SemGroupLevelV4 {
                    id: "a".into(),
                    value: "A".into(),
                    label: "A".into(),
                },
                crate::SemGroupLevelV4 {
                    id: "b".into(),
                    value: "B".into(),
                    label: "B".into(),
                },
            ],
        };
        grouped_model.ensure_valid().unwrap();
        let mut grouped_recipe = base_recipe.clone();
        enable_exact_cbsem_cfa_case_bootstrap_v2(&mut grouped_recipe, 1_000);
        embed(&mut grouped_recipe, &grouped_model);
        assert_cbsem_method_mismatch(&grouped_recipe, &grouped_model);

        let mut weighted_model = base_model.clone();
        bind_weight(
            &mut weighted_model,
            SemWeightBindingV4::Case {
                variable: "observed:case_weight".into(),
            },
        );
        let mut weighted_recipe = base_recipe.clone();
        enable_exact_cbsem_cfa_case_bootstrap_v2(&mut weighted_recipe, 1_000);
        embed(&mut weighted_recipe, &weighted_model);
        assert!(matches!(
            compile_analysis_recipe_v4(
                &weighted_recipe,
                Some(&weighted_model),
                RecipeV4CompilerTarget::CbsemPlanV2,
                RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::WeightCapability(_))
        ));

        let (mut product_recipe, product_model) = cbsem_product_indicator_recipe_and_model();
        enable_exact_cbsem_cfa_case_bootstrap_v2(&mut product_recipe, 1_000);
        assert_cbsem_method_mismatch(&product_recipe, &product_model);
    }

    #[test]
    fn cbsem_continuous_raw_mean_replacement_is_exactly_bounded() {
        let (mut recipe, mut model) = cbsem_recipe_and_model(false);
        let SemDataBindingV4::Raw { missing_data, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *missing_data = MissingDataPolicyV4::MeanReplacement;
        for variable in &mut model.variables {
            if let SemVariableV4::Observed {
                missing_markers, ..
            } = variable
            {
                *missing_markers = vec![".".into(), "N/A".into(), "NA".into()];
            }
        }
        model.ensure_valid().unwrap();
        recipe.settings.missing_data = MissingDataPolicy::MeanReplacement;
        recipe.settings.preprocessing = crate::Preprocessing::Unstandardized;
        embed(&mut recipe, &model);
        let target = RecipeV4CompilerTarget::CbsemPlanV2;
        let artifact =
            compile_analysis_recipe_v4(&recipe, Some(&model), target, target.capability_cell())
                .unwrap();
        let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
            unreachable!()
        };
        assert!(matches!(
            plan.input(),
            crate::CompiledCbsemInputV2::Raw {
                missing_data: MissingDataPolicyV4::MeanReplacement,
                ..
            }
        ));

        let mut mismatched = recipe.clone();
        mismatched.settings.missing_data = MissingDataPolicy::ListwiseDeletion;
        assert!(matches!(
            compile_analysis_recipe_v4(&mismatched, Some(&model), target, target.capability_cell(),),
            Err(RecipeV4CompilationError::MissingDataPolicyMismatch)
        ));

        let mut matrix_model = model.clone();
        let matrix_variables = matrix_model
            .variables
            .iter()
            .filter_map(|variable| match variable {
                SemVariableV4::Observed { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        matrix_model.data_binding = SemDataBindingV4::Covariance {
            dataset_id: "covariance-input".into(),
            variables: matrix_variables,
            means: None,
            standard_deviations: None,
            sample: crate::SemMatrixSampleMetadataV4 {
                sample_size: 100,
                covariance_denominator: crate::SemCovarianceDenominatorV4::MaximumLikelihoodN,
                effective_sample_size: None,
                degrees_of_freedom: None,
                group_sample_sizes: BTreeMap::new(),
            },
        };
        matrix_model.ensure_valid().unwrap();
        let mut matrix_recipe = recipe.clone();
        embed(&mut matrix_recipe, &matrix_model);
        assert!(matches!(
            compile_analysis_recipe_v4(
                &matrix_recipe,
                Some(&matrix_model),
                target,
                target.capability_cell(),
            ),
            Err(RecipeV4CompilationError::MissingDataPolicyMismatch)
        ));

        for setting in ["bootstrap", "studentized", "permutation", "mean_structure"] {
            let mut drift = recipe.clone();
            match setting {
                "bootstrap" => drift.settings.bootstrap_samples = 1,
                "studentized" => drift.settings.studentized_inner_samples = 1,
                "permutation" => drift.settings.permutation_samples = 1,
                "mean_structure" => {
                    let Some(MethodConfig::Cbsem { mean_structure, .. }) =
                        drift.method_config.as_mut()
                    else {
                        unreachable!()
                    };
                    *mean_structure = true;
                }
                _ => unreachable!(),
            }
            assert!(matches!(
                compile_analysis_recipe_v4(&drift, Some(&model), target, target.capability_cell(),),
                Err(RecipeV4CompilationError::MeanReplacementScope(_))
            ));
        }

        for binding_option in ["cluster", "strata"] {
            let mut drift_model = model.clone();
            let SemDataBindingV4::Raw {
                cluster_variable,
                strata_variable,
                ..
            } = &mut drift_model.data_binding
            else {
                unreachable!()
            };
            match binding_option {
                "cluster" => *cluster_variable = Some("observed:x1".into()),
                "strata" => *strata_variable = Some("observed:x2".into()),
                _ => unreachable!(),
            }
            drift_model.ensure_valid().unwrap();
            let mut drift_recipe = recipe.clone();
            embed(&mut drift_recipe, &drift_model);
            assert_cbsem_capability_code(
                &drift_recipe,
                &drift_model,
                "mean_replacement_raw_options_required",
            );
        }

        let mut weighted_model = model.clone();
        bind_weight(
            &mut weighted_model,
            SemWeightBindingV4::Case {
                variable: "observed:case_weight".into(),
            },
        );
        let mut weighted_recipe = recipe.clone();
        embed(&mut weighted_recipe, &weighted_model);
        let error = compile_analysis_recipe_v4(
            &weighted_recipe,
            Some(&weighted_model),
            target,
            target.capability_cell(),
        )
        .unwrap_err();
        let RecipeV4CompilationError::WeightCapability(issue) = error else {
            panic!("expected typed weight capability issue, found {error:?}")
        };
        assert_eq!(issue.code, WeightCapabilityCodeV1::CaseWeightUnsupported);
        assert_eq!(
            issue.target,
            WeightCapabilityTargetV1::CbsemMlMeanReplacementV1
        );

        let mut grouped_model = model.clone();
        grouped_model.group = crate::SemGroupV4::ObservedGroups {
            grouping_variable: "observed:x1".into(),
            levels: vec![
                crate::SemGroupLevelV4 {
                    id: "a".into(),
                    value: "A".into(),
                    label: "A".into(),
                },
                crate::SemGroupLevelV4 {
                    id: "b".into(),
                    value: "B".into(),
                    label: "B".into(),
                },
            ],
        };
        grouped_model.ensure_valid().unwrap();
        let mut grouped_recipe = recipe.clone();
        embed(&mut grouped_recipe, &grouped_model);
        assert_cbsem_capability_code(&grouped_recipe, &grouped_model, "multigroup");

        let mut transformed_model = model.clone();
        let SemVariableV4::Observed {
            source_column,
            transformation_lineage,
            ..
        } = transformed_model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "observed:x1")
            .unwrap()
        else {
            unreachable!()
        };
        *transformation_lineage = vec![crate::ObservedTransformationStepV4 {
            id: "mean-center-x1".into(),
            input_columns: vec![source_column.clone()],
            output_column: source_column.clone(),
            operation: crate::ObservedTransformationOperationV4::MeanCenter,
        }];
        transformed_model.ensure_valid().unwrap();
        let mut transformed_recipe = recipe.clone();
        embed(&mut transformed_recipe, &transformed_model);
        assert_cbsem_capability_code(
            &transformed_recipe,
            &transformed_model,
            "observed_transformation_lineage",
        );

        let (mut pls_recipe, mut pls_model) = pls_recipe_and_model();
        let SemDataBindingV4::Raw { missing_data, .. } = &mut pls_model.data_binding else {
            unreachable!()
        };
        *missing_data = MissingDataPolicyV4::MeanReplacement;
        pls_model.ensure_valid().unwrap();
        pls_recipe.settings.missing_data = MissingDataPolicy::MeanReplacement;
        embed(&mut pls_recipe, &pls_model);
        assert!(matches!(
            compile_analysis_recipe_v4(
                &pls_recipe,
                Some(&pls_model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::MeanReplacementScope(_))
        ));
    }

    #[test]
    fn recipe_weight_preflight_is_kind_specific_and_legacy_case_binding_is_exact() {
        let cases = [
            (
                SemWeightBindingV4::Case {
                    variable: "observed:case_weight".into(),
                },
                WeightCapabilityCodeV1::CaseWeightUnsupported,
            ),
            (
                SemWeightBindingV4::Frequency {
                    variable: "observed:case_weight".into(),
                },
                WeightCapabilityCodeV1::FrequencyWeightUnsupported,
            ),
            (
                SemWeightBindingV4::Sampling {
                    variable: "observed:case_weight".into(),
                    normalization: SamplingWeightNormalizationV4::None,
                },
                WeightCapabilityCodeV1::SamplingWeightUnsupported,
            ),
        ];
        for (binding, expected_code) in cases {
            let (mut recipe, mut model) = pls_recipe_and_model();
            bind_weight(&mut model, binding);
            embed(&mut recipe, &model);
            let target = RecipeV4CompilerTarget::PlsPlanV2;
            let error =
                compile_analysis_recipe_v4(&recipe, Some(&model), target, target.capability_cell())
                    .unwrap_err();
            let RecipeV4CompilationError::WeightCapability(issue) = error else {
                panic!("expected typed weight capability issue, found {error:?}")
            };
            assert_eq!(issue.code, expected_code);
            assert_eq!(issue.target, WeightCapabilityTargetV1::PlsPlanV2);
            assert_eq!(issue.subject, "observed:case_weight");
            assert_eq!(
                issue
                    .declaration
                    .as_ref()
                    .unwrap()
                    .binding()
                    .source_column(),
                "case_weight"
            );
        }

        let (mut ambiguous_recipe, model) = pls_recipe_and_model();
        ambiguous_recipe.settings.case_weight_column = Some(" case_weight ".into());
        let target = RecipeV4CompilerTarget::PlsPlanV2;
        let error = compile_analysis_recipe_v4(
            &ambiguous_recipe,
            Some(&model),
            target,
            target.capability_cell(),
        )
        .unwrap_err();
        let RecipeV4CompilationError::WeightCapability(issue) = error else {
            panic!("expected legacy ambiguity, found {error:?}")
        };
        assert_eq!(
            issue.code,
            WeightCapabilityCodeV1::LegacyCaseWeightBindingAmbiguous
        );
        assert_eq!(issue.subject, " case_weight ");
        assert!(issue.declaration.is_none());

        let (mut exact_recipe, mut exact_model) = pls_recipe_and_model();
        exact_recipe.settings.case_weight_column = Some("case_weight".into());
        bind_weight(
            &mut exact_model,
            SemWeightBindingV4::Case {
                variable: "observed:case_weight".into(),
            },
        );
        embed(&mut exact_recipe, &exact_model);
        let error = compile_analysis_recipe_v4(
            &exact_recipe,
            Some(&exact_model),
            target,
            target.capability_cell(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            RecipeV4CompilationError::WeightCapability(WeightCapabilityIssueV1 {
                code: WeightCapabilityCodeV1::CaseWeightUnsupported,
                ..
            })
        ));

        bind_weight(
            &mut exact_model,
            SemWeightBindingV4::Frequency {
                variable: "observed:case_weight".into(),
            },
        );
        embed(&mut exact_recipe, &exact_model);
        let error = compile_analysis_recipe_v4(
            &exact_recipe,
            Some(&exact_model),
            target,
            target.capability_cell(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            RecipeV4CompilationError::WeightCapability(WeightCapabilityIssueV1 {
                code: WeightCapabilityCodeV1::LegacyCaseWeightBindingAmbiguous,
                ..
            })
        ));
    }

    #[test]
    fn unresolved_legacy_tampered_and_wrong_capability_inputs_fail_closed() {
        let (mut recipe, model) = pls_recipe_and_model();
        assert!(matches!(
            compile_analysis_recipe_v4(
                &recipe,
                None,
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::UnresolvedModelReference)
        ));
        let pending_source = legacy_recipe(AnalysisMethod::PlsPm, MethodConfig::PlsAlgorithm);
        let pending = migrate_analysis_recipe_to_v4_pending(&pending_source).unwrap();
        assert!(matches!(
            compile_analysis_recipe_v4(
                &pending,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::LegacyEstimandUnspecified)
        ));
        if let AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256, ..
        } = &mut recipe.model_binding
        {
            *scientific_sha256 = "0".repeat(64);
        }
        assert!(
            compile_analysis_recipe_v4(
                &recipe,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            )
            .is_err()
        );

        let (recipe, model) = pls_recipe_and_model();
        assert!(matches!(
            compile_analysis_recipe_v4(
                &recipe,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::CapabilityCellMismatch)
        ));
    }

    #[test]
    fn project_model_reference_requires_explicit_resolution_and_exact_scientific_hash() {
        let (mut recipe, model) = pls_recipe_and_model();
        recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: model.id.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        assert!(matches!(
            compile_analysis_recipe_v4(
                &recipe,
                None,
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::UnresolvedModelReference)
        ));
        compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();
        if let AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            scientific_sha256, ..
        } = &mut recipe.model_binding
        {
            *scientific_sha256 = "0".repeat(64);
        }
        assert!(matches!(
            compile_analysis_recipe_v4(
                &recipe,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::ModelScientificDigestMismatch)
        ));
    }

    #[test]
    fn archive_receipt_revalidation_rejects_tampering_and_unknown_fields() {
        let (recipe, model) = pls_recipe_and_model();
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();
        validate_compiled_analysis_recipe_v4(&artifact, &recipe, Some(&model)).unwrap();

        let mut value = serde_json::to_value(&artifact).unwrap();
        value["receipt"]["plan_sha256"] = serde_json::json!("0".repeat(64));
        let tampered: CompiledAnalysisRecipeV4 = serde_json::from_value(value).unwrap();
        assert!(matches!(
            validate_compiled_analysis_recipe_v4(&tampered, &recipe, Some(&model)),
            Err(RecipeV4CompilationError::ArtifactMismatch)
        ));

        let mut unknown = serde_json::to_value(&artifact).unwrap();
        unknown["receipt"]["unknown"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CompiledAnalysisRecipeV4>(unknown).is_err());
        let mut nested_unknown = serde_json::to_value(&artifact).unwrap();
        nested_unknown["plan"]["plan"]["unknown"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CompiledAnalysisRecipeV4>(nested_unknown).is_err());

        let mut recipe_unknown = serde_json::to_value(&recipe).unwrap();
        recipe_unknown["unknown"] = serde_json::json!(true);
        assert!(serde_json::from_value::<AnalysisRecipeV4>(recipe_unknown).is_err());
    }

    #[test]
    fn invalid_nonfinite_settings_return_a_typed_error_without_hashing_panics() {
        let (mut recipe, model) = pls_recipe_and_model();
        recipe.settings.tolerance = f64::NAN;
        assert!(matches!(
            compile_analysis_recipe_v4(
                &recipe,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::InvalidSettings(_))
        ));
    }

    #[test]
    fn product_indicator_exact_target_compiles_revalidates_and_preserves_identity() {
        let (recipe, model) = cbsem_product_indicator_recipe_and_model();
        let target = RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1;
        let artifact =
            compile_analysis_recipe_v4(&recipe, Some(&model), target, target.capability_cell())
                .unwrap();
        validate_compiled_analysis_recipe_v4(&artifact, &recipe, Some(&model)).unwrap();
        assert_eq!(artifact.receipt().compiler_target(), target);
        assert_eq!(
            artifact.receipt().capability_cell(),
            &target.capability_cell()
        );
        assert_ne!(
            target.capability_cell(),
            CapabilityCellReferenceV2 {
                registry_schema_version: 2,
                capability_id: "smartpls.cbsem_moderator".into(),
                cell_id: "qpls3.cbsem.moderator".into(),
                capability_version: "cbsem_moderator_v1".into(),
            },
            "the unregistered Internal product-indicator source slice must never inherit the existing LMS cell identity or evidence"
        );
        let CompiledRecipePlanV4::CbsemProductIndicatorPlanV1 { plan } = artifact.plan() else {
            unreachable!()
        };
        assert!(plan.expanded_plan().derived_terms().is_empty());
        ensure_cbsem_ml_exact_parameter_table_v4_capability_v2(
            plan.expanded_plan(),
            CbsemMlExactCapabilityProfileV2::CovarianceStructure,
        )
        .unwrap();

        let mut tampered = serde_json::to_value(&artifact).unwrap();
        tampered["receipt"]["capability_cell"]["cell_id"] = serde_json::json!(CBSEM_ML_CELL_ID);
        let tampered: CompiledAnalysisRecipeV4 = serde_json::from_value(tampered).unwrap();
        assert!(matches!(
            validate_compiled_analysis_recipe_v4(&tampered, &recipe, Some(&model)),
            Err(RecipeV4CompilationError::CapabilityCellMismatch)
        ));
    }

    #[test]
    fn product_indicator_generic_target_and_wrong_capability_cell_fail_closed() {
        let (recipe, model) = cbsem_product_indicator_recipe_and_model();
        assert!(matches!(
            compile_analysis_recipe_v4(
                &recipe,
                Some(&model),
                RecipeV4CompilerTarget::CbsemPlanV2,
                RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::CbsemEstimatorCapability(_))
        ));
        assert!(matches!(
            compile_analysis_recipe_v4(
                &recipe,
                Some(&model),
                RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1,
                RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
            ),
            Err(RecipeV4CompilationError::CapabilityCellMismatch)
        ));
    }

    #[test]
    fn product_indicator_target_enforces_estimand_method_and_resampling_contracts() {
        let target = RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1;
        let (recipe, model) = cbsem_product_indicator_recipe_and_model();

        let mut wrong_estimand = recipe.clone();
        wrong_estimand.estimand_confirmation = LegacyEstimandConfirmationV4::ConfirmedComposite;
        assert!(matches!(
            compile_analysis_recipe_v4(
                &wrong_estimand,
                Some(&model),
                target,
                target.capability_cell(),
            ),
            Err(RecipeV4CompilationError::EstimandCompilerMismatch { .. })
        ));

        let mut mean_structure = recipe.clone();
        let Some(MethodConfig::Cbsem {
            mean_structure: mean,
            ..
        }) = mean_structure.method_config.as_mut()
        else {
            unreachable!()
        };
        *mean = true;
        assert!(matches!(
            compile_analysis_recipe_v4(
                &mean_structure,
                Some(&model),
                target,
                target.capability_cell(),
            ),
            Err(RecipeV4CompilationError::MethodCompilerMismatch { .. })
        ));

        for setting in ["bootstrap", "studentized_inner", "permutation"] {
            let mut resampling = recipe.clone();
            match setting {
                "bootstrap" => resampling.settings.bootstrap_samples = 1,
                "studentized_inner" => resampling.settings.studentized_inner_samples = 1,
                "permutation" => resampling.settings.permutation_samples = 1,
                _ => unreachable!(),
            }
            assert!(matches!(
                compile_analysis_recipe_v4(
                    &resampling,
                    Some(&model),
                    target,
                    target.capability_cell(),
                ),
                Err(RecipeV4CompilationError::MethodCompilerMismatch { .. })
            ));
        }

        let mut max_iterations = recipe.clone();
        max_iterations.settings.max_iterations += 1;
        assert!(matches!(
            compile_analysis_recipe_v4(
                &max_iterations,
                Some(&model),
                target,
                target.capability_cell(),
            ),
            Err(RecipeV4CompilationError::MethodCompilerMismatch { .. })
        ));

        let mut tolerance = recipe;
        tolerance.settings.tolerance = CBSEM_PRODUCT_INDICATOR_TOLERANCE_V1 * 10.0;
        assert!(matches!(
            compile_analysis_recipe_v4(&tolerance, Some(&model), target, target.capability_cell(),),
            Err(RecipeV4CompilationError::MethodCompilerMismatch { .. })
        ));
    }

    #[test]
    fn non_product_missing_settings_and_multiple_derived_terms_remain_fail_closed() {
        let (recipe, mut polynomial) = cbsem_product_indicator_recipe_and_model();
        polynomial.derived_terms[0] = SemDerivedTermV4::Polynomial {
            id: "term:x_squared".into(),
            output: "derived:x_by_m".into(),
            source: "construct:x".into(),
            degree: 2,
        };
        polynomial.ensure_valid().unwrap();
        let mut polynomial_recipe = recipe.clone();
        embed(&mut polynomial_recipe, &polynomial);
        assert!(matches!(
            compile_analysis_recipe_v4(
                &polynomial_recipe,
                Some(&polynomial),
                RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1,
                RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1.capability_cell(),
            ),
            Err(RecipeV4CompilationError::CbsemProductIndicatorCompiler(
                CompiledCbsemProductIndicatorErrorV1::Unsupported { .. }
            ))
        ));

        let (mut missing_recipe, mut missing) = cbsem_product_indicator_recipe_and_model();
        let SemDerivedTermV4::Interaction {
            product_indicator, ..
        } = &mut missing.derived_terms[0]
        else {
            unreachable!()
        };
        *product_indicator = None;
        let valid_scientific_sha256 = match &missing_recipe.model_binding {
            AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                scientific_sha256, ..
            } => scientific_sha256.clone(),
            _ => unreachable!(),
        };
        missing_recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: missing.id.clone(),
            scientific_sha256: valid_scientific_sha256,
        };
        assert!(matches!(
            compile_analysis_recipe_v4(
                &missing_recipe,
                Some(&missing),
                RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1,
                RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1.capability_cell(),
            ),
            Err(RecipeV4CompilationError::InvalidSemModel(_))
        ));

        let (mut multiple_recipe, mut multiple) = cbsem_product_indicator_recipe_and_model();
        multiple.variables.push(SemVariableV4::Derived {
            id: "derived:x_by_m_second".into(),
            label: "X × M second".into(),
        });
        multiple.relations.push(SemRelationV4::Structural {
            id: "relation:interaction_effect_second".into(),
            source: "derived:x_by_m_second".into(),
            target: "construct:y".into(),
            parameter: "parameter:interaction_effect_second".into(),
            role: crate::StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        multiple.parameters.push(SemParameterV4::Free {
            id: "parameter:interaction_effect_second".into(),
            label: "X × M second -> Y".into(),
            target: SemParameterTargetV4::Regression {
                source: "derived:x_by_m_second".into(),
                target: "construct:y".into(),
            },
            start: Some(0.1),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        let SemDerivedTermV4::Interaction {
            focal_relation,
            product_indicator,
            ..
        } = multiple.derived_terms[0].clone()
        else {
            unreachable!()
        };
        multiple.derived_terms.push(SemDerivedTermV4::Interaction {
            id: "term:x_by_m_second".into(),
            output: "derived:x_by_m_second".into(),
            predictor: "construct:x".into(),
            moderator: "construct:m".into(),
            focal_relation,
            method: InteractionMethodV4::ProductIndicator,
            product_indicator,
        });
        multiple.ensure_valid().unwrap();
        embed(&mut multiple_recipe, &multiple);
        assert!(matches!(
            compile_analysis_recipe_v4(
                &multiple_recipe,
                Some(&multiple),
                RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1,
                RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1.capability_cell(),
            ),
            Err(RecipeV4CompilationError::CbsemProductIndicatorCompiler(
                CompiledCbsemProductIndicatorErrorV1::Unsupported { .. }
            ))
        ));
    }

    #[test]
    fn wrong_interaction_method_does_not_bypass_product_target_rejection() {
        let (mut recipe, mut model) = cbsem_product_indicator_recipe_and_model();
        let SemDerivedTermV4::Interaction {
            method,
            product_indicator,
            ..
        } = &mut model.derived_terms[0]
        else {
            unreachable!()
        };
        *method = InteractionMethodV4::TwoStage;
        *product_indicator = None;
        model.ensure_valid().unwrap();
        embed(&mut recipe, &model);
        let error = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1,
            RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1.capability_cell(),
        )
        .unwrap_err();
        let RecipeV4CompilationError::CbsemProductIndicatorCompiler(
            CompiledCbsemProductIndicatorErrorV1::Unsupported { issues },
        ) = error
        else {
            panic!("unexpected error: {error:?}")
        };
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "product_indicator_method_required")
        );
    }

    #[test]
    fn active_recipe_schema_remains_v3_and_v4_compiler_is_opt_in_only() {
        assert_eq!(ANALYSIS_RECIPE_SCHEMA_VERSION, 3);
        assert_eq!(crate::ANALYSIS_RECIPE_V4_SCHEMA_VERSION, 4);
    }
}
