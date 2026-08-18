//! Internal/Labs execution and canonical-result boundary for genuine PLS
//! model comparison.
//!
//! This adapter is deliberately separate from `run_pls_analysis`: it accepts
//! two exact point-estimate recipes, binds them to one resident dataset and one
//! deterministic shared-fold contract, and never substitutes a comparison of
//! previously saved reports.

use qpls_core::{
    AnalysisRecipe, CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION, CanonicalChartDisplayOptions,
    CanonicalColumnRole, CanonicalColumnType, CanonicalMissingReason, CanonicalNoticeSeverity,
    CanonicalResultCell, CanonicalResultColumn, CanonicalResultDocumentV2,
    CanonicalResultExclusion, CanonicalResultNotice, CanonicalResultPresentationV2,
    CanonicalResultProvenanceV2, CanonicalResultRow, CanonicalResultSection, CanonicalResultTable,
    CapabilityCellReferenceV2, sha256_serialized, validate_canonical_result_document_v2,
};
use qpls_data::Dataset;
use qpls_estimation::{
    PLS_MODEL_COMPARISON_AKAIKE_WEIGHT_VERSION_V1, PLS_MODEL_COMPARISON_BIC_VERSION_V1,
    PLS_MODEL_COMPARISON_CVPAT_VERSION_V1, PLS_MODEL_COMPARISON_FOLD_ASSIGNMENT_VERSION_V1,
    PLS_MODEL_COMPARISON_METHOD_VERSION_V1, PLS_MODEL_COMPARISON_PREDICTION_VERSION_V1,
    PlsModelComparisonConfigV1, PlsModelComparisonErrorV1, PlsModelComparisonModelRoleV1,
    PlsModelComparisonPhaseV1, PlsModelComparisonProgressV1, PlsModelComparisonResultV1,
    compare_pls_models_v1_with_control,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PLS_MODEL_COMPARISON_EXECUTION_REQUEST_SCHEMA_VERSION_V1: u32 = 1;
pub const PLS_MODEL_COMPARISON_EXECUTION_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub const PLS_MODEL_COMPARISON_EXECUTION_ADAPTER_VERSION_V1: &str =
    "internal_labs_pls_model_comparison_execution_v1";
pub const PLS_MODEL_COMPARISON_CAPABILITY_ID_V1: &str = "smartpls.pls_model_comparison";
pub const PLS_MODEL_COMPARISON_CAPABILITY_CELL_ID_V1: &str = "qpls3.comparison.pls_models";
pub const PLS_MODEL_COMPARISON_CAPABILITY_VERSION_V1: &str = "pls_model_comparison_v1";

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const JAVASCRIPT_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlsModelComparisonSharedFoldContractV1 {
    pub assignment_version: String,
    pub folds: usize,
    pub repeats: usize,
    pub seed: u64,
    pub confidence_level: f64,
}

impl PlsModelComparisonSharedFoldContractV1 {
    pub fn from_config(config: PlsModelComparisonConfigV1) -> Self {
        Self {
            assignment_version: PLS_MODEL_COMPARISON_FOLD_ASSIGNMENT_VERSION_V1.into(),
            folds: config.folds,
            repeats: config.repeats,
            seed: config.seed,
            confidence_level: config.confidence_level,
        }
    }

    fn config(&self) -> PlsModelComparisonConfigV1 {
        PlsModelComparisonConfigV1 {
            folds: self.folds,
            repeats: self.repeats,
            seed: self.seed,
            confidence_level: self.confidence_level,
        }
    }
}

/// Strict two-model request. The recipe hashes are supplied independently and
/// re-derived before execution so a caller cannot change a model or setting
/// after constructing the request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InternalLabsPlsModelComparisonRequestV1 {
    pub schema_version: u32,
    pub surface: String,
    pub experimental_labs_enabled: bool,
    pub qualification_ready: bool,
    pub capability_cell: CapabilityCellReferenceV2,
    pub method_version: String,
    pub dataset_id: String,
    pub dataset_fingerprint: String,
    pub established_recipe: AnalysisRecipe,
    pub established_recipe_sha256: String,
    pub alternative_recipe: AnalysisRecipe,
    pub alternative_recipe_sha256: String,
    pub shared_folds: PlsModelComparisonSharedFoldContractV1,
}

impl InternalLabsPlsModelComparisonRequestV1 {
    pub fn exact_internal_labs(
        dataset: &Dataset,
        established_recipe: AnalysisRecipe,
        alternative_recipe: AnalysisRecipe,
        config: PlsModelComparisonConfigV1,
    ) -> Self {
        let established_recipe_sha256 = sha256_serialized(&established_recipe);
        let alternative_recipe_sha256 = sha256_serialized(&alternative_recipe);
        Self {
            schema_version: PLS_MODEL_COMPARISON_EXECUTION_REQUEST_SCHEMA_VERSION_V1,
            surface: INTERNAL_LABS_SURFACE.into(),
            experimental_labs_enabled: true,
            qualification_ready: false,
            capability_cell: comparison_capability_cell(),
            method_version: PLS_MODEL_COMPARISON_METHOD_VERSION_V1.into(),
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            established_recipe,
            established_recipe_sha256,
            alternative_recipe,
            alternative_recipe_sha256,
            shared_folds: PlsModelComparisonSharedFoldContractV1::from_config(config),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlsModelComparisonExecutionProvenanceV1 {
    pub adapter_version: String,
    pub request_sha256: String,
    pub dataset_id: String,
    pub dataset_fingerprint: String,
    pub established_recipe_id: String,
    pub established_recipe_sha256: String,
    pub established_model_id: String,
    pub established_scientific_model_sha256: String,
    pub alternative_recipe_id: String,
    pub alternative_recipe_sha256: String,
    pub alternative_model_id: String,
    pub alternative_scientific_model_sha256: String,
    pub fold_assignment_version: String,
    pub fold_assignment_digest: String,
    pub prediction_method_version: String,
    pub cvpat_method_version: String,
    pub bic_method_version: String,
    pub akaike_weight_method_version: String,
    pub workers: usize,
    pub surface: String,
    pub qualification_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlsModelComparisonExecutionResultV1 {
    pub schema_version: u32,
    pub provenance: PlsModelComparisonExecutionProvenanceV1,
    pub analytical_result: PlsModelComparisonResultV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlsModelComparisonRunContextV1 {
    pub run_id: Uuid,
    pub project_id: Uuid,
    pub started_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlsModelComparisonRunnerProgressV1 {
    pub phase: String,
    pub completed_units: u64,
    pub total_units: u64,
    pub repeat: Option<usize>,
    pub fold: Option<usize>,
    pub model: Option<String>,
}

impl From<PlsModelComparisonProgressV1> for PlsModelComparisonRunnerProgressV1 {
    fn from(value: PlsModelComparisonProgressV1) -> Self {
        Self {
            phase: phase_name(value.phase).into(),
            completed_units: value.completed_units,
            total_units: value.total_units,
            repeat: value.repeat,
            fold: value.fold,
            model: value.model.map(model_role_name).map(str::to_owned),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlsModelComparisonExecutionErrorV1 {
    #[error("PLS model comparison is available only through Experimental Labs")]
    InternalLabsRequired,
    #[error("PLS model-comparison request schema must equal 1")]
    RequestSchema,
    #[error("PLS model-comparison capability-cell identity is not exact")]
    CapabilityIdentity,
    #[error("PLS model-comparison method identity is not exact")]
    MethodIdentity,
    #[error("PLS model-comparison requests must remain qualification_ready=false")]
    QualificationBoundary,
    #[error("resident dataset id differs from the exact comparison request")]
    DatasetIdMismatch,
    #[error("resident dataset fingerprint differs from the exact comparison request")]
    DatasetFingerprintMismatch,
    #[error("{model} recipe digest differs from the exact request binding")]
    RecipeDigestMismatch { model: &'static str },
    #[error("PLS model comparison v1 executes serially and requires workers=1 in both recipes")]
    WorkerContract,
    #[error("shared-fold assignment method identity is not exact")]
    FoldAssignmentIdentity,
    #[error("shared-fold seed exceeds the canonical-result safe-integer boundary")]
    UnsafeSeed,
    #[error("PLS model comparison was cancelled")]
    Cancelled,
    #[error(transparent)]
    Scientific(PlsModelComparisonErrorV1),
    #[error("canonical PLS model-comparison result is invalid: {0}")]
    Canonical(String),
}

impl PlsModelComparisonExecutionErrorV1 {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InternalLabsRequired => "pls_model_comparison.internal_labs_required",
            Self::RequestSchema => "pls_model_comparison.request_schema_mismatch",
            Self::CapabilityIdentity => "pls_model_comparison.capability_identity_mismatch",
            Self::MethodIdentity => "pls_model_comparison.method_identity_mismatch",
            Self::QualificationBoundary => "pls_model_comparison.qualification_boundary",
            Self::DatasetIdMismatch => "pls_model_comparison.dataset_id_mismatch",
            Self::DatasetFingerprintMismatch => "pls_model_comparison.dataset_fingerprint_mismatch",
            Self::RecipeDigestMismatch { .. } => "pls_model_comparison.recipe_digest_mismatch",
            Self::WorkerContract => "pls_model_comparison.worker_contract",
            Self::FoldAssignmentIdentity => "pls_model_comparison.fold_identity_mismatch",
            Self::UnsafeSeed => "pls_model_comparison.unsafe_seed",
            Self::Cancelled => "pls_model_comparison.cancelled",
            Self::Scientific(PlsModelComparisonErrorV1::Cancelled) => {
                "pls_model_comparison.cancelled"
            }
            Self::Scientific(_) => "pls_model_comparison.scientific_contract",
            Self::Canonical(_) => "pls_model_comparison.canonical_result_invalid",
        }
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(
            self,
            Self::Cancelled | Self::Scientific(PlsModelComparisonErrorV1::Cancelled)
        )
    }
}

pub fn comparison_capability_cell() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_MODEL_COMPARISON_CAPABILITY_ID_V1.into(),
        cell_id: PLS_MODEL_COMPARISON_CAPABILITY_CELL_ID_V1.into(),
        capability_version: PLS_MODEL_COMPARISON_CAPABILITY_VERSION_V1.into(),
    }
}

pub fn validate_internal_labs_pls_model_comparison_request_v1(
    dataset: &Dataset,
    request: &InternalLabsPlsModelComparisonRequestV1,
) -> Result<(), PlsModelComparisonExecutionErrorV1> {
    if request.surface != INTERNAL_LABS_SURFACE || !request.experimental_labs_enabled {
        return Err(PlsModelComparisonExecutionErrorV1::InternalLabsRequired);
    }
    if request.schema_version != PLS_MODEL_COMPARISON_EXECUTION_REQUEST_SCHEMA_VERSION_V1 {
        return Err(PlsModelComparisonExecutionErrorV1::RequestSchema);
    }
    if request.capability_cell != comparison_capability_cell() {
        return Err(PlsModelComparisonExecutionErrorV1::CapabilityIdentity);
    }
    if request.method_version != PLS_MODEL_COMPARISON_METHOD_VERSION_V1 {
        return Err(PlsModelComparisonExecutionErrorV1::MethodIdentity);
    }
    if request.qualification_ready {
        return Err(PlsModelComparisonExecutionErrorV1::QualificationBoundary);
    }
    if request.dataset_id != dataset.id.to_string() {
        return Err(PlsModelComparisonExecutionErrorV1::DatasetIdMismatch);
    }
    if request.dataset_fingerprint != dataset.fingerprint.0 {
        return Err(PlsModelComparisonExecutionErrorV1::DatasetFingerprintMismatch);
    }
    for (model, recipe, expected) in [
        (
            "established",
            &request.established_recipe,
            &request.established_recipe_sha256,
        ),
        (
            "alternative",
            &request.alternative_recipe,
            &request.alternative_recipe_sha256,
        ),
    ] {
        if sha256_serialized(recipe) != *expected {
            return Err(PlsModelComparisonExecutionErrorV1::RecipeDigestMismatch { model });
        }
    }
    if request.established_recipe.settings.workers != 1
        || request.alternative_recipe.settings.workers != 1
    {
        return Err(PlsModelComparisonExecutionErrorV1::WorkerContract);
    }
    if request.shared_folds.assignment_version != PLS_MODEL_COMPARISON_FOLD_ASSIGNMENT_VERSION_V1 {
        return Err(PlsModelComparisonExecutionErrorV1::FoldAssignmentIdentity);
    }
    if request.shared_folds.seed > JAVASCRIPT_MAX_SAFE_INTEGER {
        return Err(PlsModelComparisonExecutionErrorV1::UnsafeSeed);
    }
    Ok(())
}

pub fn run_internal_labs_pls_model_comparison_v1(
    dataset: &Dataset,
    request: &InternalLabsPlsModelComparisonRequestV1,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(PlsModelComparisonRunnerProgressV1) + Sync,
) -> Result<PlsModelComparisonExecutionResultV1, PlsModelComparisonExecutionErrorV1> {
    if should_cancel() {
        return Err(PlsModelComparisonExecutionErrorV1::Cancelled);
    }
    validate_internal_labs_pls_model_comparison_request_v1(dataset, request)?;
    let analytical_result = compare_pls_models_v1_with_control(
        dataset,
        &request.established_recipe,
        &request.alternative_recipe,
        request.shared_folds.config(),
        |update| {
            progress(update.clone().into());
            !should_cancel()
        },
    )
    .map_err(PlsModelComparisonExecutionErrorV1::Scientific)?;
    if should_cancel() {
        return Err(PlsModelComparisonExecutionErrorV1::Cancelled);
    }
    if analytical_result.method_version != request.method_version
        || analytical_result.surface != INTERNAL_LABS_SURFACE
        || analytical_result.qualified
        || analytical_result.dataset_fingerprint != request.dataset_fingerprint
        || analytical_result.fold_plan.assignment_version != request.shared_folds.assignment_version
        || analytical_result.fold_plan.seed != request.shared_folds.seed
        || analytical_result.fold_plan.folds != request.shared_folds.folds
        || analytical_result.fold_plan.repeats != request.shared_folds.repeats
        || analytical_result.prediction_method_version != PLS_MODEL_COMPARISON_PREDICTION_VERSION_V1
        || analytical_result.cvpat.method_version != PLS_MODEL_COMPARISON_CVPAT_VERSION_V1
        || analytical_result.bic_method_version != PLS_MODEL_COMPARISON_BIC_VERSION_V1
        || analytical_result.akaike_weight_method_version
            != PLS_MODEL_COMPARISON_AKAIKE_WEIGHT_VERSION_V1
    {
        return Err(PlsModelComparisonExecutionErrorV1::Scientific(
            PlsModelComparisonErrorV1::PredictionContract {
                subject: "execution provenance".into(),
                detail: "analytical result differs from its exact Internal/Labs request".into(),
            },
        ));
    }
    let provenance = PlsModelComparisonExecutionProvenanceV1 {
        adapter_version: PLS_MODEL_COMPARISON_EXECUTION_ADAPTER_VERSION_V1.into(),
        request_sha256: sha256_serialized(request),
        dataset_id: request.dataset_id.clone(),
        dataset_fingerprint: request.dataset_fingerprint.clone(),
        established_recipe_id: request.established_recipe.id.to_string(),
        established_recipe_sha256: request.established_recipe_sha256.clone(),
        established_model_id: analytical_result.established.model_id.clone(),
        established_scientific_model_sha256: recorded_sha256(
            &analytical_result.established.scientific_model_digest,
        )?,
        alternative_recipe_id: request.alternative_recipe.id.to_string(),
        alternative_recipe_sha256: request.alternative_recipe_sha256.clone(),
        alternative_model_id: analytical_result.alternative.model_id.clone(),
        alternative_scientific_model_sha256: recorded_sha256(
            &analytical_result.alternative.scientific_model_digest,
        )?,
        fold_assignment_version: analytical_result.fold_plan.assignment_version.clone(),
        fold_assignment_digest: analytical_result.fold_plan.assignment_digest.clone(),
        prediction_method_version: analytical_result.prediction_method_version.clone(),
        cvpat_method_version: analytical_result.cvpat.method_version.clone(),
        bic_method_version: analytical_result.bic_method_version.clone(),
        akaike_weight_method_version: analytical_result.akaike_weight_method_version.clone(),
        workers: 1,
        surface: INTERNAL_LABS_SURFACE.into(),
        qualification_ready: false,
    };
    Ok(PlsModelComparisonExecutionResultV1 {
        schema_version: PLS_MODEL_COMPARISON_EXECUTION_RESULT_SCHEMA_VERSION_V1,
        provenance,
        analytical_result,
    })
}

pub fn build_pls_model_comparison_canonical_result_v2(
    context: &PlsModelComparisonRunContextV1,
    result: &PlsModelComparisonExecutionResultV1,
) -> Result<CanonicalResultDocumentV2, PlsModelComparisonExecutionErrorV1> {
    if result.schema_version != PLS_MODEL_COMPARISON_EXECUTION_RESULT_SCHEMA_VERSION_V1
        || result.provenance.adapter_version != PLS_MODEL_COMPARISON_EXECUTION_ADAPTER_VERSION_V1
        || result.provenance.surface != INTERNAL_LABS_SURFACE
        || result.provenance.qualification_ready
        || result.provenance.workers != 1
        || result.analytical_result.qualified
    {
        return Err(PlsModelComparisonExecutionErrorV1::Canonical(
            "execution result crossed its Internal/Labs or qualification boundary".into(),
        ));
    }
    let provenance = &result.provenance;
    let analytical = &result.analytical_result;
    if provenance.dataset_fingerprint != analytical.dataset_fingerprint
        || provenance.established_recipe_id != analytical.established.recipe_id
        || provenance.established_model_id != analytical.established.model_id
        || provenance.established_scientific_model_sha256
            != recorded_sha256(&analytical.established.scientific_model_digest)?
        || provenance.alternative_recipe_id != analytical.alternative.recipe_id
        || provenance.alternative_model_id != analytical.alternative.model_id
        || provenance.alternative_scientific_model_sha256
            != recorded_sha256(&analytical.alternative.scientific_model_digest)?
        || provenance.fold_assignment_version != analytical.fold_plan.assignment_version
        || provenance.fold_assignment_digest != analytical.fold_plan.assignment_digest
        || provenance.prediction_method_version != analytical.prediction_method_version
        || provenance.cvpat_method_version != analytical.cvpat.method_version
        || provenance.bic_method_version != analytical.bic_method_version
        || provenance.akaike_weight_method_version != analytical.akaike_weight_method_version
    {
        return Err(PlsModelComparisonExecutionErrorV1::Canonical(
            "immutable execution provenance differs from the analytical comparison payload".into(),
        ));
    }
    for digest in [
        &provenance.request_sha256,
        &provenance.established_recipe_sha256,
        &provenance.alternative_recipe_sha256,
        &provenance.established_scientific_model_sha256,
        &provenance.alternative_scientific_model_sha256,
    ] {
        recorded_sha256(digest)?;
    }
    let capability_cell = comparison_capability_cell();
    let capability_cells = Some(vec![capability_cell.clone()]);
    let dataset_fingerprint = recorded_sha256(&provenance.dataset_fingerprint)?;
    let fold_digest = recorded_sha256(&provenance.fold_assignment_digest)?;
    let combined_model_digest = sha256_serialized(&[
        provenance.established_scientific_model_sha256.as_str(),
        provenance.alternative_scientific_model_sha256.as_str(),
    ]);
    let combined_recipe_identity = sha256_serialized(&[
        provenance.established_recipe_id.as_str(),
        provenance.alternative_recipe_id.as_str(),
    ]);

    let run_details = CanonicalResultTable {
        id: "comparison_run_details".into(),
        title: "Comparison run details".into(),
        description: Some(
            "Immutable two-model, dataset, shared-fold, method, and qualification-boundary provenance."
                .into(),
        ),
        columns: vec![
            text_column("surface", "Surface", "Product surface authorized for this run.", CanonicalColumnRole::Provenance),
            boolean_column("qualification_ready", "Qualification ready", "Whether this result is eligible for qualification or Standard exposure.", CanonicalColumnRole::Diagnostic),
            text_column("request_sha256", "Request SHA-256", "Digest of the complete strict two-model request.", CanonicalColumnRole::Provenance),
            text_column("dataset_id", "Dataset ID", "Exact resident dataset identifier.", CanonicalColumnRole::Provenance),
            text_column("dataset_fingerprint", "Dataset fingerprint", "Exact resident dataset SHA-256.", CanonicalColumnRole::Provenance),
            text_column("established_recipe_id", "Established recipe ID", "Exact established-model recipe identifier.", CanonicalColumnRole::Provenance),
            text_column("established_recipe_sha256", "Established recipe SHA-256", "Digest of the complete established-model recipe.", CanonicalColumnRole::Provenance),
            text_column("established_model_id", "Established model ID", "Established scientific model identifier.", CanonicalColumnRole::Provenance),
            text_column("established_model_sha256", "Established model SHA-256", "Established scientific-model digest.", CanonicalColumnRole::Provenance),
            text_column("alternative_recipe_id", "Alternative recipe ID", "Exact alternative-model recipe identifier.", CanonicalColumnRole::Provenance),
            text_column("alternative_recipe_sha256", "Alternative recipe SHA-256", "Digest of the complete alternative-model recipe.", CanonicalColumnRole::Provenance),
            text_column("alternative_model_id", "Alternative model ID", "Alternative scientific model identifier.", CanonicalColumnRole::Provenance),
            text_column("alternative_model_sha256", "Alternative model SHA-256", "Alternative scientific-model digest.", CanonicalColumnRole::Provenance),
            text_column("fold_assignment_version", "Fold assignment version", "Deterministic shared-fold assignment method.", CanonicalColumnRole::Provenance),
            text_column("fold_assignment_sha256", "Fold assignment SHA-256", "Digest of the exact shared-fold ledger.", CanonicalColumnRole::Provenance),
            number_column("folds", "Folds", "Number of folds per repeat.", CanonicalColumnRole::Provenance),
            number_column("repeats", "Repeats", "Number of repeated fold assignments.", CanonicalColumnRole::Provenance),
            number_column("seed", "Seed", "Seed used for exact shared folds and point fits.", CanonicalColumnRole::Provenance),
            number_column("complete_rows", "Complete rows", "Rows retained by the shared complete-case comparison contract.", CanonicalColumnRole::Diagnostic),
            text_column("prediction_version", "Prediction version", "PLSpredict outcome-comparison method identity.", CanonicalColumnRole::Provenance),
            text_column("cvpat_version", "CVPAT version", "Paired model-loss CVPAT method identity.", CanonicalColumnRole::Provenance),
            text_column("bic_version", "BIC version", "Prediction-oriented BIC method identity.", CanonicalColumnRole::Provenance),
            text_column("akaike_weight_version", "Akaike-weight version", "Two-candidate weight method identity.", CanonicalColumnRole::Provenance),
        ],
        rows: vec![CanonicalResultRow {
            id: "run".into(),
            cells: vec![
                text(&provenance.surface),
                boolean(provenance.qualification_ready),
                text(&provenance.request_sha256),
                text(&provenance.dataset_id),
                text(&dataset_fingerprint),
                text(&provenance.established_recipe_id),
                text(&provenance.established_recipe_sha256),
                text(&provenance.established_model_id),
                text(&provenance.established_scientific_model_sha256),
                text(&provenance.alternative_recipe_id),
                text(&provenance.alternative_recipe_sha256),
                text(&provenance.alternative_model_id),
                text(&provenance.alternative_scientific_model_sha256),
                text(&provenance.fold_assignment_version),
                text(&fold_digest),
                number(analytical.fold_plan.folds as f64),
                number(analytical.fold_plan.repeats as f64),
                number(analytical.fold_plan.seed as f64),
                number(analytical.fold_plan.complete_rows.len() as f64),
                text(&provenance.prediction_method_version),
                text(&provenance.cvpat_method_version),
                text(&provenance.bic_method_version),
                text(&provenance.akaike_weight_method_version),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let prediction_accuracy = CanonicalResultTable {
        id: "prediction_accuracy".into(),
        title: "PLSpredict outcome comparison".into(),
        description: Some(
            "Indicator-level prediction errors for both candidates on the same held-out cases and folds."
                .into(),
        ),
        columns: vec![
            text_column("construct", "Construct", "Common reflective endogenous construct.", CanonicalColumnRole::Label),
            text_column("indicator", "Indicator", "Predicted outcome indicator.", CanonicalColumnRole::Label),
            number_column("observations", "Predictions", "Held-out predictions accumulated across repeats.", CanonicalColumnRole::Diagnostic),
            number_column("established_sse", "Established SSE", "Established-model squared error sum.", CanonicalColumnRole::Estimate),
            number_column("established_rmse", "Established RMSE", "Established-model root mean squared error.", CanonicalColumnRole::Estimate),
            number_column("established_mae", "Established MAE", "Established-model mean absolute error.", CanonicalColumnRole::Estimate),
            number_column("alternative_sse", "Alternative SSE", "Alternative-model squared error sum.", CanonicalColumnRole::Estimate),
            number_column("alternative_rmse", "Alternative RMSE", "Alternative-model root mean squared error.", CanonicalColumnRole::Estimate),
            number_column("alternative_mae", "Alternative MAE", "Alternative-model mean absolute error.", CanonicalColumnRole::Estimate),
            number_column("indicator_average_sse", "IA SSE", "Indicator-average benchmark squared error sum.", CanonicalColumnRole::Estimate),
            number_column("indicator_average_rmse", "IA RMSE", "Indicator-average benchmark root mean squared error.", CanonicalColumnRole::Estimate),
            number_column("indicator_average_mae", "IA MAE", "Indicator-average benchmark mean absolute error.", CanonicalColumnRole::Estimate),
            number_column("q2_predict_established", "Established Q2 predict", "Established predictive relevance relative to the indicator-average benchmark.", CanonicalColumnRole::Estimate),
            number_column("q2_predict_alternative", "Alternative Q2 predict", "Alternative predictive relevance relative to the indicator-average benchmark.", CanonicalColumnRole::Estimate),
            text_column("lower_rmse_model", "Lower RMSE", "Candidate with lower RMSE.", CanonicalColumnRole::Decision),
            text_column("lower_mae_model", "Lower MAE", "Candidate with lower MAE.", CanonicalColumnRole::Decision),
        ],
        rows: analytical
            .indicator_predictions
            .iter()
            .enumerate()
            .map(|(index, row)| CanonicalResultRow {
                id: format!("indicator_{index:04}"),
                cells: vec![
                    text(&row.construct),
                    text(&row.indicator),
                    number(row.established.observations as f64),
                    number(row.established.squared_error_sum),
                    number(row.established.rmse),
                    number(row.established.mae),
                    number(row.alternative.squared_error_sum),
                    number(row.alternative.rmse),
                    number(row.alternative.mae),
                    number(row.indicator_average.squared_error_sum),
                    number(row.indicator_average.rmse),
                    number(row.indicator_average.mae),
                    optional_number(row.q_squared_predict_established),
                    optional_number(row.q_squared_predict_alternative),
                    text(&row.lower_rmse_model),
                    text(&row.lower_mae_model),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let fold_losses = CanonicalResultTable {
        id: "shared_fold_losses".into(),
        title: "Shared-fold losses".into(),
        description: Some(
            "Paired model losses for every repeat and fold; held-out source-row identities prove the shared assignment."
                .into(),
        ),
        columns: vec![
            number_column("repeat", "Repeat", "Zero-based repeat index.", CanonicalColumnRole::Label),
            number_column("fold", "Fold", "Zero-based fold index.", CanonicalColumnRole::Label),
            text_column("test_rows", "Held-out rows", "Exact ordered source-row indices assigned to this test fold.", CanonicalColumnRole::Provenance),
            number_column("test_count", "Test rows", "Number of held-out rows.", CanonicalColumnRole::Diagnostic),
            number_column("established_mean_loss", "Established loss", "Established-model mean squared case loss.", CanonicalColumnRole::Estimate),
            number_column("alternative_mean_loss", "Alternative loss", "Alternative-model mean squared case loss.", CanonicalColumnRole::Estimate),
            number_column("loss_difference", "Alternative minus established", "Paired average loss difference.", CanonicalColumnRole::Estimate),
        ],
        rows: analytical
            .fold_losses
            .iter()
            .map(|row| CanonicalResultRow {
                id: format!("repeat_{:04}_fold_{:04}", row.repeat, row.fold),
                cells: vec![
                    number(row.repeat as f64),
                    number(row.fold as f64),
                    text(
                        row.test_rows
                            .iter()
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join(","),
                    ),
                    number(row.test_rows.len() as f64),
                    number(row.established_mean_loss),
                    number(row.alternative_mean_loss),
                    number(row.average_loss_difference),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let case_losses = CanonicalResultTable {
        id: "paired_case_losses".into(),
        title: "Paired case losses".into(),
        description: Some(
            "Per-case losses averaged across exact repeated folds and used by paired CVPAT.".into(),
        ),
        columns: vec![
            number_column(
                "source_row",
                "Source row",
                "Zero-based source-row identity.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "repeats",
                "Repeats",
                "Number of held-out predictions averaged for the case.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "established_mean_loss",
                "Established loss",
                "Established-model mean case loss.",
                CanonicalColumnRole::Estimate,
            ),
            number_column(
                "alternative_mean_loss",
                "Alternative loss",
                "Alternative-model mean case loss.",
                CanonicalColumnRole::Estimate,
            ),
            number_column(
                "loss_difference",
                "Alternative minus established",
                "Paired case-loss difference used by CVPAT.",
                CanonicalColumnRole::Estimate,
            ),
        ],
        rows: analytical
            .case_losses
            .iter()
            .map(|row| CanonicalResultRow {
                id: format!("case_{:08}", row.source_row),
                cells: vec![
                    number(row.source_row as f64),
                    number(row.repeats as f64),
                    number(row.established_mean_loss),
                    number(row.alternative_mean_loss),
                    number(row.loss_difference),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let cvpat = &analytical.cvpat;
    let cvpat_table = CanonicalResultTable {
        id: "paired_cvpat".into(),
        title: "Paired CVPAT between models".into(),
        description: Some(
            "Paired case-loss comparison using alternative minus established as the directional contrast."
                .into(),
        ),
        columns: vec![
            text_column("method_version", "Method version", "Frozen paired-CVPAT method identity.", CanonicalColumnRole::Provenance),
            text_column("loss", "Loss", "Loss function used for the paired comparison.", CanonicalColumnRole::Provenance),
            text_column("target_scope", "Target scope", "Outcome scope included in case loss.", CanonicalColumnRole::Provenance),
            number_column("observations", "Cases", "Paired complete cases.", CanonicalColumnRole::Diagnostic),
            number_column("established_mean_loss", "Established loss", "Established-model mean case loss.", CanonicalColumnRole::Estimate),
            number_column("alternative_mean_loss", "Alternative loss", "Alternative-model mean case loss.", CanonicalColumnRole::Estimate),
            number_column("average_loss_difference", "Alternative minus established", "Average paired case-loss difference.", CanonicalColumnRole::Estimate),
            number_column("sample_variance", "Sample variance", "Sample variance of paired case-loss differences.", CanonicalColumnRole::Uncertainty),
            number_column("standard_error", "Standard error", "Standard error of the average paired loss difference.", CanonicalColumnRole::Uncertainty),
            number_column("t_statistic", "t statistic", "Paired CVPAT t statistic.", CanonicalColumnRole::Estimate),
            number_column("degrees_of_freedom", "Degrees of freedom", "Paired-test degrees of freedom.", CanonicalColumnRole::Diagnostic),
            number_column("p_one_sided", "p (alternative lower)", "One-sided probability for lower alternative-model loss.", CanonicalColumnRole::Uncertainty),
            number_column("p_two_sided", "p (two-sided)", "Two-sided paired-test probability.", CanonicalColumnRole::Uncertainty),
            number_column("confidence_level", "Confidence level", "Confidence level for the paired loss-difference interval.", CanonicalColumnRole::Provenance),
            number_column("ci_lower", "CI lower", "Lower confidence bound for alternative minus established.", CanonicalColumnRole::Uncertainty),
            number_column("ci_upper", "CI upper", "Upper confidence bound for alternative minus established.", CanonicalColumnRole::Uncertainty),
            text_column("lower_loss_model", "Lower-loss model", "Candidate with lower average case loss.", CanonicalColumnRole::Decision),
            text_column("directional_decision", "Directional decision", "Decision under the frozen directional contract.", CanonicalColumnRole::Decision),
            text_column("status", "Status", "Availability status.", CanonicalColumnRole::Diagnostic),
            text_column("unavailable_reason", "Unavailable reason", "Typed reason when inference is unavailable.", CanonicalColumnRole::Diagnostic),
        ],
        rows: vec![CanonicalResultRow {
            id: "paired_model_loss".into(),
            cells: vec![
                text(&cvpat.method_version),
                text(&cvpat.loss),
                text(&cvpat.target_scope),
                number(cvpat.observations as f64),
                number(cvpat.established_mean_loss),
                number(cvpat.alternative_mean_loss),
                number(cvpat.average_loss_difference),
                optional_number(cvpat.sample_variance_of_case_differences),
                optional_number(cvpat.standard_error),
                optional_number(cvpat.t_statistic),
                number(cvpat.degrees_of_freedom as f64),
                optional_number(cvpat.p_value_one_sided_alternative_lower),
                optional_number(cvpat.p_value_two_sided),
                number(cvpat.confidence_level),
                optional_number(cvpat.confidence_interval_lower),
                optional_number(cvpat.confidence_interval_upper),
                text(&cvpat.lower_loss_model),
                text(&cvpat.directional_decision),
                text(&cvpat.status),
                optional_text(cvpat.unavailable_reason.as_deref()),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let bic_table = CanonicalResultTable {
        id: "prediction_oriented_bic".into(),
        title: "Prediction-oriented BIC and two-candidate weights".into(),
        description: Some(
            "Equation-level residual-SSE BIC variants and their two-candidate normalized weights; no generic likelihood BIC is inferred."
                .into(),
        ),
        columns: vec![
            text_column("construct", "Construct", "Common endogenous equation.", CanonicalColumnRole::Label),
            number_column("sample_size", "Sample size", "Complete cases used for the equation-level criterion.", CanonicalColumnRole::Diagnostic),
            number_column("established_sse", "Established SSE", "Established equation residual SSE.", CanonicalColumnRole::Estimate),
            number_column("alternative_sse", "Alternative SSE", "Alternative equation residual SSE.", CanonicalColumnRole::Estimate),
            number_column("established_parameters", "Established parameters", "Established equation predictor count used in the penalty.", CanonicalColumnRole::Provenance),
            number_column("alternative_parameters", "Alternative parameters", "Alternative equation predictor count used in the penalty.", CanonicalColumnRole::Provenance),
            number_column("established_bic", "Established BIC", "Established prediction-oriented BIC.", CanonicalColumnRole::Estimate),
            number_column("alternative_bic", "Alternative BIC", "Alternative prediction-oriented BIC.", CanonicalColumnRole::Estimate),
            number_column("established_delta", "Established delta BIC", "Established distance from the lower candidate BIC.", CanonicalColumnRole::Estimate),
            number_column("alternative_delta", "Alternative delta BIC", "Alternative distance from the lower candidate BIC.", CanonicalColumnRole::Estimate),
            number_column("established_weight", "Established weight", "Two-candidate normalized weight derived from BIC delta.", CanonicalColumnRole::Estimate),
            number_column("alternative_weight", "Alternative weight", "Two-candidate normalized weight derived from BIC delta.", CanonicalColumnRole::Estimate),
            text_column("lower_bic_model", "Lower BIC", "Candidate with lower prediction-oriented BIC.", CanonicalColumnRole::Decision),
        ],
        rows: analytical
            .bic
            .iter()
            .enumerate()
            .map(|(index, row)| CanonicalResultRow {
                id: format!("equation_{index:04}"),
                cells: vec![
                    text(&row.construct),
                    number(row.sample_size as f64),
                    number(row.established_sse),
                    number(row.alternative_sse),
                    number(row.established_parameter_count as f64),
                    number(row.alternative_parameter_count as f64),
                    number(row.established_bic),
                    number(row.alternative_bic),
                    number(row.established_delta_bic),
                    number(row.alternative_delta_bic),
                    number(row.established_akaike_weight),
                    number(row.alternative_akaike_weight),
                    text(&row.lower_bic_model),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let mut notices = vec![CanonicalResultNotice {
        id: "internal_labs_boundary".into(),
        code: "pls_model_comparison_internal_labs_only".into(),
        severity: CanonicalNoticeSeverity::Information,
        message: "This result is Internal/Labs only, qualification_ready=false, and unavailable in Standard Calculate."
            .into(),
        section_ids: vec!["run_details".into()],
        table_ids: vec!["comparison_run_details".into()],
    }];
    notices.extend(
        analytical
            .warnings
            .iter()
            .enumerate()
            .map(|(index, warning)| CanonicalResultNotice {
                id: format!("method_warning_{index:04}"),
                code: "pls_model_comparison_method_warning".into(),
                severity: CanonicalNoticeSeverity::Warning,
                message: warning.clone(),
                section_ids: Vec::new(),
                table_ids: Vec::new(),
            }),
    );

    let document = CanonicalResultDocumentV2 {
        schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
        document_id: format!("result_{}", context.run_id),
        title: "PLS model comparison".into(),
        provenance: CanonicalResultProvenanceV2 {
            run_id: context.run_id.to_string(),
            project_id: context.project_id.to_string(),
            model_id: format!("pls_models:{combined_model_digest}"),
            model_digest: combined_model_digest,
            dataset_id: provenance.dataset_id.clone(),
            dataset_fingerprint,
            recipe_id: format!("pls_models:{combined_recipe_identity}"),
            recipe_digest: provenance.request_sha256.clone(),
            capability_cell: capability_cell.clone(),
            method_version: analytical.method_version.clone(),
            engine_version: provenance.adapter_version.clone(),
            seed: Some(analytical.fold_plan.seed as i64),
            workers: 1,
            started_at: context.started_at.clone(),
            completed_at: context.completed_at.clone(),
        },
        capability_cells: capability_cells.clone(),
        general_sem_results: None,
        sections: vec![
            CanonicalResultSection {
                id: "run_details".into(),
                title: "Run details".into(),
                description: Some("Exact source, method, and shared-fold provenance.".into()),
                table_ids: vec!["comparison_run_details".into()],
                chart_ids: Vec::new(),
                capability_cells: capability_cells.clone(),
            },
            CanonicalResultSection {
                id: "prediction_results".into(),
                title: "Prediction results".into(),
                description: Some("Shared-fold prediction accuracy and paired losses.".into()),
                table_ids: vec![
                    "prediction_accuracy".into(),
                    "shared_fold_losses".into(),
                    "paired_case_losses".into(),
                    "paired_cvpat".into(),
                ],
                chart_ids: Vec::new(),
                capability_cells: capability_cells.clone(),
            },
            CanonicalResultSection {
                id: "model_selection".into(),
                title: "Prediction-oriented model selection".into(),
                description: Some(
                    "Documented equation-level BIC variants and two-candidate weights.".into(),
                ),
                table_ids: vec!["prediction_oriented_bic".into()],
                chart_ids: Vec::new(),
                capability_cells: capability_cells.clone(),
            },
        ],
        tables: vec![
            run_details,
            prediction_accuracy,
            fold_losses,
            case_losses,
            cvpat_table,
            bic_table,
        ],
        charts: Vec::new(),
        notices,
        exclusions: vec![CanonicalResultExclusion {
            id: "generic_information_criteria".into(),
            capability_cell: Some(capability_cell),
            title: "Generic information criteria are excluded".into(),
            reason: "This contract does not infer likelihood BIC, whole-model BIC, AIC, GM, or any undocumented model-selection criterion."
                .into(),
        }],
        footnotes: Vec::new(),
        presentation: CanonicalResultPresentationV2 {
            default_section_id: Some("prediction_results".into()),
            default_table_id: Some("prediction_accuracy".into()),
            precision: 4,
            missing_value_label: "—".into(),
            chart_defaults: CanonicalChartDisplayOptions::default(),
        },
    };
    let validation = validate_canonical_result_document_v2(&document);
    if !validation.passed {
        return Err(PlsModelComparisonExecutionErrorV1::Canonical(
            validation.errors.join("; "),
        ));
    }
    Ok(document)
}

fn recorded_sha256(value: &str) -> Result<String, PlsModelComparisonExecutionErrorV1> {
    let candidate = value.rsplit_once(':').map_or(value, |(_, suffix)| suffix);
    if candidate.len() == 64
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(candidate.to_owned())
    } else {
        Err(PlsModelComparisonExecutionErrorV1::Canonical(
            "scientific provenance contains an invalid SHA-256 value".into(),
        ))
    }
}

fn phase_name(phase: PlsModelComparisonPhaseV1) -> &'static str {
    match phase {
        PlsModelComparisonPhaseV1::Validating => "validating",
        PlsModelComparisonPhaseV1::AssigningFolds => "assigning_folds",
        PlsModelComparisonPhaseV1::EstimatingEstablished => "estimating_established",
        PlsModelComparisonPhaseV1::EstimatingAlternative => "estimating_alternative",
        PlsModelComparisonPhaseV1::ComputingPairedCvpat => "computing_paired_cvpat",
        PlsModelComparisonPhaseV1::EstimatingInSampleBic => "estimating_in_sample_bic",
        PlsModelComparisonPhaseV1::Assembling => "assembling",
    }
}

fn model_role_name(role: PlsModelComparisonModelRoleV1) -> &'static str {
    match role {
        PlsModelComparisonModelRoleV1::Established => "established",
        PlsModelComparisonModelRoleV1::Alternative => "alternative",
    }
}

fn text_column(
    id: &str,
    label: &str,
    description: &str,
    role: CanonicalColumnRole,
) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Text,
        description: description.into(),
        role: Some(role),
        unit: None,
        default_precision: None,
    }
}

fn number_column(
    id: &str,
    label: &str,
    description: &str,
    role: CanonicalColumnRole,
) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Number,
        description: description.into(),
        role: Some(role),
        unit: None,
        default_precision: Some(6),
    }
}

fn boolean_column(
    id: &str,
    label: &str,
    description: &str,
    role: CanonicalColumnRole,
) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Boolean,
        description: description.into(),
        role: Some(role),
        unit: None,
        default_precision: None,
    }
}

fn text(value: impl Into<String>) -> CanonicalResultCell {
    CanonicalResultCell::Text {
        value: value.into(),
    }
}

fn number(value: f64) -> CanonicalResultCell {
    CanonicalResultCell::Number {
        value,
        display: None,
    }
}

fn boolean(value: bool) -> CanonicalResultCell {
    CanonicalResultCell::Boolean { value }
}

fn optional_number(value: Option<f64>) -> CanonicalResultCell {
    value.map_or_else(
        || CanonicalResultCell::Missing {
            reason: CanonicalMissingReason::Undefined,
            display: None,
        },
        number,
    )
}

fn optional_text(value: Option<&str>) -> CanonicalResultCell {
    value.map_or_else(
        || CanonicalResultCell::Missing {
            reason: CanonicalMissingReason::Undefined,
            display: None,
        },
        text,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{AnalysisSettings, Construct, MeasurementMode, ModelSpec, StructuralPath};
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    fn fixture() -> (Dataset, InternalLabsPlsModelComparisonRequestV1) {
        let mut csv = String::from("x1,x2,z1,z2,y1,y2\n");
        for row in 0..48 {
            let t = row as f64 / 7.0;
            let x = (t * 0.7).sin() + row as f64 * 0.015;
            let z = (t * 1.1).cos() - row as f64 * 0.009;
            let noise = ((row * 17 % 13) as f64 - 6.0) * 0.018;
            let y = 0.62 * x + 0.56 * z + noise;
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                x + noise * 0.2,
                x * 0.93 - noise * 0.15,
                z - noise * 0.12,
                z * 1.04 + noise * 0.18,
                y + noise * 0.25,
                y * 0.96 - noise * 0.2
            ));
        }
        let bytes = csv.into_bytes();
        let dataset = import_delimited_bytes(
            &bytes,
            "runner-model-comparison.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let constructs = vec![
            Construct {
                id: "x".into(),
                name: "X".into(),
                short_name: "X".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into()],
            },
            Construct {
                id: "z".into(),
                name: "Z".into(),
                short_name: "Z".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["z1".into(), "z2".into()],
            },
            Construct {
                id: "y".into(),
                name: "Y".into(),
                short_name: "Y".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["y1".into(), "y2".into()],
            },
        ];
        let model = |name: &str, include_z: bool| ModelSpec {
            id: Uuid::new_v4(),
            name: name.into(),
            constructs: if include_z {
                constructs.clone()
            } else {
                constructs
                    .iter()
                    .filter(|construct| construct.id != "z")
                    .cloned()
                    .collect()
            },
            paths: if include_z {
                vec![
                    StructuralPath {
                        source: "x".into(),
                        target: "y".into(),
                    },
                    StructuralPath {
                        source: "z".into(),
                        target: "y".into(),
                    },
                ]
            } else {
                vec![StructuralPath {
                    source: "x".into(),
                    target: "y".into(),
                }]
            },
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let settings = AnalysisSettings {
            seed: 991,
            workers: 1,
            ..AnalysisSettings::default()
        };
        let mut established =
            AnalysisRecipe::new(&bytes, model("Established", false), settings.clone());
        let mut alternative = AnalysisRecipe::new(&bytes, model("Alternative", true), settings);
        established.dataset_fingerprint = dataset.fingerprint.0.clone();
        alternative.dataset_fingerprint = dataset.fingerprint.0.clone();
        let request = InternalLabsPlsModelComparisonRequestV1::exact_internal_labs(
            &dataset,
            established,
            alternative,
            PlsModelComparisonConfigV1 {
                folds: 4,
                repeats: 2,
                seed: 7_301,
                confidence_level: 0.95,
            },
        );
        (dataset, request)
    }

    #[test]
    fn strict_request_runs_real_models_and_builds_exact_canonical_tables() {
        let (dataset, request) = fixture();
        let updates = Mutex::new(Vec::new());
        let result = run_internal_labs_pls_model_comparison_v1(
            &dataset,
            &request,
            || false,
            |update| updates.lock().unwrap().push(update),
        )
        .unwrap();
        assert_eq!(
            result.provenance.request_sha256,
            sha256_serialized(&request)
        );
        assert_eq!(result.analytical_result.completed_fold_pairs, 8);
        assert!(!updates.into_inner().unwrap().is_empty());

        let document = build_pls_model_comparison_canonical_result_v2(
            &PlsModelComparisonRunContextV1 {
                run_id: Uuid::from_u128(301),
                project_id: Uuid::from_u128(302),
                started_at: "2026-08-15T00:00:00Z".into(),
                completed_at: "2026-08-15T00:00:01Z".into(),
            },
            &result,
        )
        .unwrap();
        assert!(validate_canonical_result_document_v2(&document).passed);
        assert_eq!(document.tables.len(), 6);
        assert!(document.tables.iter().all(|table| {
            table.capability_cells.as_ref() == Some(&vec![comparison_capability_cell()])
        }));
        assert_eq!(
            document
                .tables
                .iter()
                .find(|table| table.id == "shared_fold_losses")
                .unwrap()
                .rows
                .len(),
            8
        );
        assert_eq!(
            document.provenance.recipe_digest,
            sha256_serialized(&request)
        );
        assert_eq!(
            document.provenance.capability_cell,
            comparison_capability_cell()
        );

        let mut tampered = result;
        tampered.provenance.fold_assignment_digest = format!("sha256:{}", "0".repeat(64));
        assert!(matches!(
            build_pls_model_comparison_canonical_result_v2(
                &PlsModelComparisonRunContextV1 {
                    run_id: Uuid::from_u128(303),
                    project_id: Uuid::from_u128(304),
                    started_at: "2026-08-15T00:00:00Z".into(),
                    completed_at: "2026-08-15T00:00:01Z".into(),
                },
                &tampered,
            ),
            Err(PlsModelComparisonExecutionErrorV1::Canonical(_))
        ));
    }

    #[test]
    fn access_hash_worker_fold_and_qualification_boundaries_fail_closed() {
        let (dataset, request) = fixture();
        let mut standard = request.clone();
        standard.surface = "standard".into();
        assert!(matches!(
            validate_internal_labs_pls_model_comparison_request_v1(&dataset, &standard),
            Err(PlsModelComparisonExecutionErrorV1::InternalLabsRequired)
        ));

        let mut tampered = request.clone();
        tampered.established_recipe.model.name = "Changed after hashing".into();
        assert!(matches!(
            validate_internal_labs_pls_model_comparison_request_v1(&dataset, &tampered),
            Err(PlsModelComparisonExecutionErrorV1::RecipeDigestMismatch {
                model: "established"
            })
        ));

        let mut workers = request.clone();
        workers.alternative_recipe.settings.workers = 2;
        workers.alternative_recipe_sha256 = sha256_serialized(&workers.alternative_recipe);
        assert!(matches!(
            validate_internal_labs_pls_model_comparison_request_v1(&dataset, &workers),
            Err(PlsModelComparisonExecutionErrorV1::WorkerContract)
        ));

        let mut folds = request.clone();
        folds.shared_folds.assignment_version = "generic_kfold".into();
        assert!(matches!(
            validate_internal_labs_pls_model_comparison_request_v1(&dataset, &folds),
            Err(PlsModelComparisonExecutionErrorV1::FoldAssignmentIdentity)
        ));

        let mut promoted = request;
        promoted.qualification_ready = true;
        assert!(matches!(
            validate_internal_labs_pls_model_comparison_request_v1(&dataset, &promoted),
            Err(PlsModelComparisonExecutionErrorV1::QualificationBoundary)
        ));
    }

    #[test]
    fn cancellation_is_authoritative_and_never_returns_a_partial_result() {
        let (dataset, request) = fixture();
        let cancel = AtomicBool::new(false);
        let result = run_internal_labs_pls_model_comparison_v1(
            &dataset,
            &request,
            || cancel.load(Ordering::Acquire),
            |update| {
                if update.phase == "estimating_alternative" {
                    cancel.store(true, Ordering::Release);
                }
            },
        );
        assert!(cancel.load(Ordering::Acquire));
        assert!(result.unwrap_err().is_cancelled());
    }
}
