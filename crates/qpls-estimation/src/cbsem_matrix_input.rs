use crate::continuous_raw_mean_replacement_v1::prepare_continuous_raw_mean_replacement_v1_after_integrity_with_control;
use crate::{
    CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1,
    CBSEM_ML_OPTIMIZER_FINITE_DIFFERENCE_RELATIVE_STEP,
    CBSEM_ML_OPTIMIZER_OBJECTIVE_STAGNATION_TOLERANCE,
    CBSEM_ML_OPTIMIZER_STAGNATION_GRADIENT_NORM_TOLERANCE, CbsemAnalysis,
    CbsemExactCaseBootstrapDeleteOneRefitV1, CbsemExactCaseBootstrapRefitV1,
    CbsemExactCaseBootstrapRefitWithAnalyticStandardErrorsV1,
    CbsemExactCaseBootstrapSamplingReceiptV1, CbsemExactParameterTableErrorV3,
    ContinuousRawMeanReplacementErrorV1, ContinuousRawMeanReplacementVariableBindingV1,
    EstimationError, MeanReplacementReceiptV1,
    cbsem_exact_parameter_table::{
        CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3, CbsemExactLatentCovarianceKindV3,
        CbsemExactParameterRowV3, CbsemExactParameterSpecificationV3, CbsemExactParameterTargetV3,
        estimate_cbsem_ml_exact_parameter_table_v3_point_refit_with_analytic_standard_errors_with_control,
        estimate_cbsem_ml_exact_parameter_table_v3_point_refit_with_control,
        estimate_cbsem_ml_exact_parameter_table_v3_with_control,
        estimate_cbsem_ml_exact_parameter_table_v4_with_control, exact_free_dimension_members_v3,
    },
};
use arrow::array::{Array, Float64Array, Int64Array};
use qpls_core::{
    AnalysisRecipeV4, CompiledAnalysisRecipeV4, CompiledCbsemInputV2, CompiledCbsemParameterRoleV2,
    CompiledCbsemParameterStatusV2, CompiledCbsemPlanV2, CompiledRecipePlanV4, Construct,
    FactorIdentificationV4, FactorMeanPolicyV4, MeasurementMode, ModelSpec, ObservedRoleV4,
    ObservedScaleV4, Preprocessing, RecipeV4CompilationError, SemCovarianceDenominatorV4,
    SemEndpointV4, SemGroupV4, SemParameterTargetV4, SemVariableV4, StructuralPath,
    validate_compiled_analysis_recipe_v4,
};
use qpls_data::{
    ColumnType, DataKind, Dataset, DatasetDescriptor, ScaleType, dataset_from_descriptor,
    write_arrow,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub const CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V2: &str = "cbsem_ml_compiled_moment_input_v2";
pub const CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3: &str =
    CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3;
pub const CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4: &str = "cbsem_ml_compiled_moment_input_v4";
pub const CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1: &str =
    "cbsem_ml_compiled_moment_input_mean_replacement_v1";
pub const CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2: u32 = 2;
pub const CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3: u32 = 3;
pub const CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4: u32 = 4;
/// The optimizer can accept an objective-stagnation solution only while its
/// gradient norm is below this threshold. Raw-scale parameter distance is not
/// bounded by that condition without an independently qualified Hessian
/// condition number, so raw/matrix equivalence is instead evaluated on the
/// standardized solution and implied covariance under this absolute envelope.
pub const CBSEM_RAW_MATRIX_STANDARDIZED_ABS_TOLERANCE_V2: f64 =
    CBSEM_ML_OPTIMIZER_STAGNATION_GRADIENT_NORM_TOLERANCE;
/// Relative numerical envelope inherited from the central finite-difference
/// step used by the optimizer.
pub const CBSEM_RAW_MATRIX_STANDARDIZED_REL_TOLERANCE_V2: f64 =
    CBSEM_ML_OPTIMIZER_FINITE_DIFFERENCE_RELATIVE_STEP;
/// Absolute objective envelope is exactly the optimizer's objective-stagnation
/// threshold; the finite-difference step supplies its relative component.
pub const CBSEM_RAW_MATRIX_OBJECTIVE_ABS_TOLERANCE_V2: f64 =
    CBSEM_ML_OPTIMIZER_OBJECTIVE_STAGNATION_TOLERANCE;
pub const CBSEM_RAW_MATRIX_OBJECTIVE_REL_TOLERANCE_V2: f64 =
    CBSEM_ML_OPTIMIZER_FINITE_DIFFERENCE_RELATIVE_STEP;
const MATRIX_SYMMETRY_ABS_TOLERANCE: f64 = 1e-10;
const CORRELATION_DIAGONAL_ABS_TOLERANCE: f64 = 1e-10;
const POSITIVE_DEFINITE_RELATIVE_TOLERANCE: f64 = 1e-12;
const MEAN_REPLACEMENT_CANCELLATION_POLL_INTERVAL_V1: usize = 1_024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemMomentInputKindV2 {
    Raw,
    Covariance,
    Correlation,
}

pub fn cbsem_raw_matrix_standardized_values_equivalent_v2(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs()
            <= CBSEM_RAW_MATRIX_STANDARDIZED_ABS_TOLERANCE_V2
                + CBSEM_RAW_MATRIX_STANDARDIZED_REL_TOLERANCE_V2 * left.abs().max(right.abs())
}

pub fn cbsem_raw_matrix_objectives_equivalent_v2(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs()
            <= CBSEM_RAW_MATRIX_OBJECTIVE_ABS_TOLERANCE_V2
                + CBSEM_RAW_MATRIX_OBJECTIVE_REL_TOLERANCE_V2 * left.abs().max(right.abs())
}

impl CbsemMomentInputKindV2 {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Covariance => "covariance",
            Self::Correlation => "correlation",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemMomentInputProvenanceV2 {
    pub kind: CbsemMomentInputKindV2,
    pub dataset_id: String,
    pub dataset_fingerprint: String,
    pub declared_sample_size: Option<usize>,
    pub used_sample_size: usize,
    pub omitted_observations: usize,
    pub covariance_denominator: SemCovarianceDenominatorV4,
    pub variable_ids: Vec<String>,
    pub source_columns: Vec<String>,
    pub standard_deviations: Option<BTreeMap<String, f64>>,
    pub canonical_ml_covariance_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_observed_means_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_data_treatment: Option<MeanReplacementReceiptV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemMeanCellV4 {
    pub variable: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemCompiledMomentResultV2 {
    pub schema_version: u32,
    pub method_version: String,
    pub compiler_analytical_identity_sha256: String,
    pub plan_sha256: String,
    pub model_scientific_sha256: String,
    pub input: CbsemMomentInputProvenanceV2,
    /// Canonically ordered ML covariance (denominator n) actually consumed by
    /// the optimizer. It is included so persistence/export qualification can
    /// prove that no matrix transformation was implicit.
    pub covariance_ml: Vec<Vec<f64>>,
    /// Existing estimator parameter names mapped back to stable SemModelV4
    /// parameter ids. No result parameter is allowed to be anonymous.
    pub parameter_ids: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observed_means: Vec<CbsemMeanCellV4>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implied_means: Vec<CbsemMeanCellV4>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub residual_means: Vec<CbsemMeanCellV4>,
    pub analysis: CbsemAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemMatrixModelIssueV2 {
    pub code: String,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemCompiledMomentProgressV2 {
    pub phase: String,
    pub completed_units: u64,
    pub total_units: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum CbsemCompiledMomentErrorV2 {
    #[error("CB-SEM moment-input execution was cancelled")]
    Cancelled,
    #[error(transparent)]
    CompiledArtifact(#[from] RecipeV4CompilationError),
    #[error("compiled recipe does not contain a CB-SEM plan")]
    WrongCompiledPlan,
    #[error("dataset fingerprint differs from the compiled recipe receipt")]
    DatasetFingerprintMismatch,
    #[error("dataset bytes or schema do not reproduce their declared fingerprint: {0}")]
    DatasetIntegrity(String),
    #[error("SemModelV4 data binding dataset id {expected} differs from dataset {actual}")]
    DatasetIdMismatch { expected: String, actual: String },
    #[error("SemModelV4 input kind {expected:?} differs from dataset kind {actual:?}")]
    InputKindMismatch {
        expected: CbsemMomentInputKindV2,
        actual: DataKind,
    },
    #[error("compiled moment-input CB-SEM requires unstandardized preprocessing")]
    UnsupportedPreprocessing,
    #[error("matrix metadata sample_size {declared} differs from dataset sample_size {actual:?}")]
    SampleSizeMismatch {
        declared: usize,
        actual: Option<usize>,
    },
    #[error("matrix binding variables do not exactly match the dataset matrix labels")]
    MatrixVariableBindingMismatch,
    #[error(
        "matrix input must be square with shape {expected} x {expected}; found {rows} x {columns}"
    )]
    MatrixShape {
        expected: usize,
        rows: usize,
        columns: usize,
    },
    #[error("matrix cell ({row}, {column}) is missing or nonnumeric")]
    MatrixCellInvalid { row: usize, column: usize },
    #[error("matrix cell ({row}, {column}) is not finite")]
    MatrixCellNonFinite { row: usize, column: usize },
    #[error("matrix is not symmetric at ({row}, {column})")]
    MatrixNotSymmetric { row: usize, column: usize },
    #[error("correlation diagonal at index {index} must equal one")]
    CorrelationDiagonalInvalid { index: usize },
    #[error("correlation at ({row}, {column}) must be between -1 and 1")]
    CorrelationOutOfRange { row: usize, column: usize },
    #[error("covariance matrix is not strictly positive definite at pivot {pivot}")]
    MatrixNotPositiveDefinite { pivot: usize },
    #[error("raw input contains a non-finite value in {column} at row {row}")]
    RawValueNonFinite { column: String, row: usize },
    #[error(
        "exact CB-SEM case bootstrap requires raw listwise single-group CFA without weights, clusters, strata, or outer resampling"
    )]
    ExactCaseBootstrapUnsupported,
    #[error(
        "exact CB-SEM case-bootstrap source exceeds the modeled-variable workload limit V<={maximum} (actual V={actual})"
    )]
    ExactCaseBootstrapModeledVariableLimit { actual: usize, maximum: usize },
    #[error(
        "exact CB-SEM case-bootstrap source exceeds the parameter workload limits P<={maximum_free_parameter_rows}, D<={maximum_optimizer_dimensions} (actual P={actual_free_parameter_rows}, D={actual_optimizer_dimensions})"
    )]
    ExactCaseBootstrapParameterDimensionLimit {
        actual_free_parameter_rows: usize,
        maximum_free_parameter_rows: usize,
        actual_optimizer_dimensions: usize,
        maximum_optimizer_dimensions: usize,
    },
    #[error(
        "exact CB-SEM case-bootstrap source exceeds the complete-case workload limit N<={maximum} (actual N={actual})"
    )]
    ExactCaseBootstrapCompleteCaseLimit { actual: usize, maximum: usize },
    #[error("exact CB-SEM case-bootstrap refit requires at least ten sampled rows (found {0})")]
    ExactCaseBootstrapInsufficientObservations(usize),
    #[error(
        "exact CB-SEM case-bootstrap draw length {actual} differs from the validated complete-case sample size {expected}"
    )]
    ExactCaseBootstrapDrawSizeMismatch { expected: usize, actual: usize },
    #[error(
        "exact CB-SEM case-bootstrap draw position {draw_position} references complete-case sampling position {sampling_position}, but the validated universe has {complete_case_sample_size} rows"
    )]
    ExactCaseBootstrapSamplingPositionOutOfRange {
        draw_position: usize,
        sampling_position: usize,
        complete_case_sample_size: usize,
    },
    #[error(
        "exact CB-SEM delete-one position {omitted_position} is outside the validated complete-case universe of {complete_case_sample_size} rows"
    )]
    ExactCaseBootstrapOmittedPositionOutOfRange {
        omitted_position: usize,
        complete_case_sample_size: usize,
    },
    #[error(
        "exact CB-SEM case-bootstrap draw position {draw_position} references source row {source_row}, but the source has {source_row_count} rows"
    )]
    ExactCaseBootstrapIndexOutOfRange {
        draw_position: usize,
        source_row: usize,
        source_row_count: usize,
    },
    #[error(
        "exact CB-SEM case-bootstrap source row {source_row} is not listwise complete for column {column}"
    )]
    ExactCaseBootstrapIncompleteRow { source_row: usize, column: String },
    #[error("exact CB-SEM case-bootstrap refit parameter identity is inconsistent: {0}")]
    ExactCaseBootstrapParameterIdentity(String),
    #[error("exact CB-SEM case-bootstrap refit unexpectedly contained nested inference")]
    ExactCaseBootstrapNestedInference,
    #[error(transparent)]
    MeanReplacement(#[from] ContinuousRawMeanReplacementErrorV1),
    #[error("at least ten complete observations are required (found {0})")]
    InsufficientObservations(usize),
    #[error("compiled CB-SEM plan contains {count} unsupported scientific feature(s)", count = .issues.len())]
    UnsupportedPlan {
        issues: Vec<CbsemMatrixModelIssueV2>,
    },
    #[error("CB-SEM moment-input optimizer did not converge")]
    NonConvergence,
    #[error(transparent)]
    ExactParameterTable(#[from] CbsemExactParameterTableErrorV3),
    #[error(transparent)]
    Estimation(#[from] EstimationError),
}

#[derive(Debug)]
struct PreparedMomentsV2 {
    provenance: CbsemMomentInputProvenanceV2,
    covariance_ml: Vec<Vec<f64>>,
    observed_means: Option<Vec<f64>>,
}

#[derive(Debug)]
pub(crate) struct ExactProjectionV3 {
    pub(crate) model: ModelSpec,
    pub(crate) indicator_names: Vec<String>,
    pub(crate) parameter_rows: Vec<CbsemExactParameterRowV3>,
    pub(crate) parameter_ids: BTreeMap<String, String>,
    pub(crate) mean_structure: bool,
}

/// Executes the bounded compiled-plan ML slice for raw observations,
/// covariance matrices, or scaled correlation matrices. The artifact is
/// deterministically recompiled before use, and the dataset is fingerprinted
/// again from its current in-memory bytes so a stale or tampered payload fails
/// before estimation.
pub fn estimate_cbsem_ml_compiled_moments_v2(
    dataset: &Dataset,
    artifact: &CompiledAnalysisRecipeV4,
    source_recipe: &AnalysisRecipeV4,
    resolved_model: &qpls_core::SemModelV4,
) -> Result<CbsemCompiledMomentResultV2, CbsemCompiledMomentErrorV2> {
    estimate_cbsem_ml_compiled_moments_v2_with_control(
        dataset,
        artifact,
        source_recipe,
        resolved_model,
        || false,
        |_| {},
    )
}

/// Cancellable form used by the internal native Recipe-v4 job worker. A
/// cancellation check occurs before every immutable boundary and inside each
/// optimizer iteration; a cancelled call never constructs a result payload.
pub fn estimate_cbsem_ml_compiled_moments_v2_with_control(
    dataset: &Dataset,
    artifact: &CompiledAnalysisRecipeV4,
    source_recipe: &AnalysisRecipeV4,
    resolved_model: &qpls_core::SemModelV4,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(CbsemCompiledMomentProgressV2) + Sync,
) -> Result<CbsemCompiledMomentResultV2, CbsemCompiledMomentErrorV2> {
    cbsem_moment_checkpoint(&should_cancel, &progress, "integrity", 0, 3)?;
    validate_compiled_analysis_recipe_v4(artifact, source_recipe, Some(resolved_model))?;
    if artifact.receipt().dataset_fingerprint() != dataset.fingerprint.0.as_str() {
        return Err(CbsemCompiledMomentErrorV2::DatasetFingerprintMismatch);
    }
    let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
        return Err(CbsemCompiledMomentErrorV2::WrongCompiledPlan);
    };
    let actual_dataset_id = dataset.id.to_string();
    if plan.input().dataset_id() != actual_dataset_id {
        return Err(CbsemCompiledMomentErrorV2::DatasetIdMismatch {
            expected: plan.input().dataset_id().into(),
            actual: actual_dataset_id,
        });
    }
    validate_dataset_integrity(dataset)?;
    cbsem_moment_checkpoint(&should_cancel, &progress, "integrity", 1, 3)?;
    if source_recipe.settings.preprocessing != Preprocessing::Unstandardized {
        return Err(CbsemCompiledMomentErrorV2::UnsupportedPreprocessing);
    }
    let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
        return Err(CbsemCompiledMomentErrorV2::WrongCompiledPlan);
    };
    let projection = build_exact_projection_v3(plan, source_recipe)?;
    cbsem_moment_checkpoint(&should_cancel, &progress, "projection", 2, 3)?;
    let prepared = prepare_moments(
        plan,
        dataset,
        &projection.indicator_names,
        projection.mean_structure,
        &should_cancel,
    )?;
    cbsem_moment_checkpoint(&should_cancel, &progress, "moments", 3, 3)?;
    let estimator_progress = |completed_units, total_units| {
        progress(CbsemCompiledMomentProgressV2 {
            phase: "estimation".into(),
            completed_units,
            total_units,
        });
    };
    let mean_replacement = prepared.provenance.missing_data_treatment.is_some();
    let (analysis, implied_mean_values, schema_version, method_version) = if projection
        .mean_structure
    {
        let observed = prepared.observed_means.as_deref().ok_or_else(|| {
                CbsemCompiledMomentErrorV2::UnsupportedPlan {
                    issues: vec![CbsemMatrixModelIssueV2 {
                        code: "raw_observed_means_missing".into(),
                        subject: "data_binding".into(),
                        message: "Raw-data mean structure requires observed means from the exact listwise-complete rows."
                            .into(),
                    }],
                }
            })?;
        let result = estimate_cbsem_ml_exact_parameter_table_v4_with_control(
            &projection.model,
            &projection.indicator_names,
            &prepared.covariance_ml,
            observed,
            prepared.provenance.used_sample_size,
            prepared.provenance.kind.as_str(),
            &projection.parameter_rows,
            &should_cancel,
            &estimator_progress,
        )
        .map_err(map_exact_parameter_error)?;
        (
            result.analysis,
            result.implied_means,
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3,
            CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
        )
    } else {
        let analysis = estimate_cbsem_ml_exact_parameter_table_v3_with_control(
            &projection.model,
            &projection.indicator_names,
            &prepared.covariance_ml,
            prepared.provenance.used_sample_size,
            prepared.provenance.kind.as_str(),
            &projection.parameter_rows,
            !mean_replacement,
            &should_cancel,
            &estimator_progress,
        )
        .map_err(map_exact_parameter_error)?;
        let (schema_version, method_version) = if mean_replacement {
            (
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4,
                CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1,
            )
        } else {
            (
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
            )
        };
        (analysis, Vec::new(), schema_version, method_version)
    };
    cbsem_moment_checkpoint(&should_cancel, &progress, "result", 1, 1)?;
    if !analysis.converged {
        return Err(CbsemCompiledMomentErrorV2::NonConvergence);
    }
    let observed_means = prepared.observed_means.unwrap_or_default();
    let residual_mean_values = observed_means
        .iter()
        .zip(&implied_mean_values)
        .map(|(observed, implied)| observed - implied)
        .collect::<Vec<_>>();
    Ok(CbsemCompiledMomentResultV2 {
        schema_version,
        method_version: method_version.into(),
        compiler_analytical_identity_sha256: artifact.receipt().analytical_identity_sha256().into(),
        plan_sha256: artifact.receipt().plan_sha256().into(),
        model_scientific_sha256: artifact.receipt().model_scientific_sha256().into(),
        input: prepared.provenance,
        covariance_ml: prepared.covariance_ml,
        parameter_ids: projection.parameter_ids,
        observed_means: mean_cells(&projection.indicator_names, &observed_means),
        implied_means: mean_cells(&projection.indicator_names, &implied_mean_values),
        residual_means: mean_cells(&projection.indicator_names, &residual_mean_values),
        analysis,
    })
}

/// Caller-supplied upper bounds for fail-closed source preparation. The
/// bounded preparer derives every count with the same projection and listwise
/// internals used to construct the reusable source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CbsemExactCaseBootstrapSourceWorkloadLimitsV1 {
    pub maximum_complete_case_sample_size: usize,
    pub maximum_modeled_variable_count: usize,
    pub maximum_free_parameter_row_count: usize,
    pub maximum_optimizer_dimension_count: usize,
}

/// Validated, immutable source for exact covariance-structure CFA bootstrap
/// refits. Dataset bytes, compiler/model bindings, projection, listwise row
/// eligibility, and numeric source values are checked once before scheduling;
/// individual replicates only map preplanned sampling positions and refit.
#[derive(Debug, Clone)]
pub struct CbsemExactCaseBootstrapSourceV1 {
    source_dataset_id: String,
    source_dataset_fingerprint: String,
    compiler_analytical_identity_sha256: String,
    plan_sha256: String,
    model_scientific_sha256: String,
    source_row_count: usize,
    complete_case_universe_sha256: String,
    complete_source_row_indices: Vec<usize>,
    complete_rows: Vec<Vec<f64>>,
    model: ModelSpec,
    indicator_names: Vec<String>,
    parameter_rows: Vec<CbsemExactParameterRowV3>,
    modeled_variable_count: usize,
    free_parameter_row_count: usize,
    optimizer_dimension_count: usize,
}

impl CbsemExactCaseBootstrapSourceV1 {
    pub fn source_dataset_id(&self) -> &str {
        &self.source_dataset_id
    }

    pub fn source_dataset_fingerprint(&self) -> &str {
        &self.source_dataset_fingerprint
    }

    pub fn compiler_analytical_identity_sha256(&self) -> &str {
        &self.compiler_analytical_identity_sha256
    }

    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub fn model_scientific_sha256(&self) -> &str {
        &self.model_scientific_sha256
    }

    pub fn source_row_count(&self) -> usize {
        self.source_row_count
    }

    pub fn complete_case_sample_size(&self) -> usize {
        self.complete_rows.len()
    }

    pub fn modeled_variable_count(&self) -> usize {
        self.modeled_variable_count
    }

    pub fn free_parameter_row_count(&self) -> usize {
        self.free_parameter_row_count
    }

    /// Number of independent free dimensions optimized by the exact plan.
    /// This may be smaller than the free parameter-row count when equality
    /// labels bind multiple rows to the same optimizer dimension.
    pub fn optimizer_dimension_count(&self) -> usize {
        self.optimizer_dimension_count
    }

    pub fn complete_case_universe_sha256(&self) -> &str {
        &self.complete_case_universe_sha256
    }

    pub fn sampling_receipt(
        &self,
        sampling_positions: &[usize],
    ) -> Result<CbsemExactCaseBootstrapSamplingReceiptV1, CbsemCompiledMomentErrorV2> {
        exact_case_bootstrap_sampling_receipt_with_control(self, sampling_positions, &|| false)
            .map(|(receipt, _)| receipt)
    }
}

pub fn cbsem_exact_case_bootstrap_modeled_variable_count_v1(
    model: &qpls_core::SemModelV4,
) -> usize {
    model
        .variables
        .iter()
        .filter(|variable| matches!(variable, SemVariableV4::Observed { .. }))
        .count()
}

fn exact_case_bootstrap_sampling_receipt_with_control(
    source: &CbsemExactCaseBootstrapSourceV1,
    sampling_positions: &[usize],
    should_cancel: &impl Fn() -> bool,
) -> Result<(CbsemExactCaseBootstrapSamplingReceiptV1, Vec<usize>), CbsemCompiledMomentErrorV2> {
    let complete_case_sample_size = source.complete_case_sample_size();
    exact_case_sampling_receipt_with_expected_length(
        source,
        sampling_positions,
        complete_case_sample_size,
        should_cancel,
    )
}

fn exact_case_sampling_receipt_with_expected_length(
    source: &CbsemExactCaseBootstrapSourceV1,
    sampling_positions: &[usize],
    expected_length: usize,
    should_cancel: &impl Fn() -> bool,
) -> Result<(CbsemExactCaseBootstrapSamplingReceiptV1, Vec<usize>), CbsemCompiledMomentErrorV2> {
    let complete_case_sample_size = source.complete_case_sample_size();
    if sampling_positions.len() != expected_length {
        return Err(
            CbsemCompiledMomentErrorV2::ExactCaseBootstrapDrawSizeMismatch {
                expected: expected_length,
                actual: sampling_positions.len(),
            },
        );
    }
    let mut source_row_indices = Vec::with_capacity(sampling_positions.len());
    for (draw_position, sampling_position) in sampling_positions.iter().copied().enumerate() {
        if should_cancel() {
            return Err(CbsemCompiledMomentErrorV2::Cancelled);
        }
        let source_row = source
            .complete_source_row_indices
            .get(sampling_position)
            .copied()
            .ok_or(
                CbsemCompiledMomentErrorV2::ExactCaseBootstrapSamplingPositionOutOfRange {
                    draw_position,
                    sampling_position,
                    complete_case_sample_size,
                },
            )?;
        source_row_indices.push(source_row);
    }
    let receipt = CbsemExactCaseBootstrapSamplingReceiptV1 {
        complete_case_sample_size,
        sampling_positions_digest_method:
            CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1.into(),
        sampling_positions_sha256: cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
            complete_case_sample_size,
            sampling_positions,
        ),
        sample_indices_digest_method: CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1.into(),
        sample_indices_sha256: cbsem_exact_case_bootstrap_index_digest_v1(
            &source.source_dataset_fingerprint,
            source.source_row_count,
            &source_row_indices,
        ),
    };
    Ok((receipt, source_row_indices))
}

pub fn prepare_cbsem_ml_exact_case_bootstrap_source_v1(
    dataset: &Dataset,
    artifact: &CompiledAnalysisRecipeV4,
    source_recipe: &AnalysisRecipeV4,
    resolved_model: &qpls_core::SemModelV4,
) -> Result<CbsemExactCaseBootstrapSourceV1, CbsemCompiledMomentErrorV2> {
    prepare_cbsem_ml_exact_case_bootstrap_source_v1_with_control(
        dataset,
        artifact,
        source_recipe,
        resolved_model,
        || false,
        |_| {},
    )
}

pub fn prepare_cbsem_ml_exact_case_bootstrap_source_v1_with_control(
    dataset: &Dataset,
    artifact: &CompiledAnalysisRecipeV4,
    source_recipe: &AnalysisRecipeV4,
    resolved_model: &qpls_core::SemModelV4,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(CbsemCompiledMomentProgressV2) + Sync,
) -> Result<CbsemExactCaseBootstrapSourceV1, CbsemCompiledMomentErrorV2> {
    prepare_cbsem_ml_exact_case_bootstrap_source_v1_impl(
        dataset,
        artifact,
        source_recipe,
        resolved_model,
        None,
        &should_cancel,
        &progress,
    )
}

pub fn prepare_cbsem_ml_exact_case_bootstrap_source_v1_with_workload_limits_and_control(
    dataset: &Dataset,
    artifact: &CompiledAnalysisRecipeV4,
    source_recipe: &AnalysisRecipeV4,
    resolved_model: &qpls_core::SemModelV4,
    workload_limits: CbsemExactCaseBootstrapSourceWorkloadLimitsV1,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(CbsemCompiledMomentProgressV2) + Sync,
) -> Result<CbsemExactCaseBootstrapSourceV1, CbsemCompiledMomentErrorV2> {
    prepare_cbsem_ml_exact_case_bootstrap_source_v1_impl(
        dataset,
        artifact,
        source_recipe,
        resolved_model,
        Some(workload_limits),
        &should_cancel,
        &progress,
    )
}

fn exact_case_bootstrap_source_parameter_dimensions_v1(
    parameter_rows: &mut [CbsemExactParameterRowV3],
) -> Result<(usize, usize), CbsemCompiledMomentErrorV2> {
    parameter_rows.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    let free_parameter_row_count = parameter_rows
        .iter()
        .filter(|row| {
            matches!(
                &row.specification,
                CbsemExactParameterSpecificationV3::Free { .. }
            )
        })
        .count();
    let optimizer_dimension_count = exact_free_dimension_members_v3(parameter_rows)
        .map_err(map_exact_parameter_error)?
        .len();
    Ok((free_parameter_row_count, optimizer_dimension_count))
}

fn prepare_cbsem_ml_exact_case_bootstrap_source_v1_impl(
    dataset: &Dataset,
    artifact: &CompiledAnalysisRecipeV4,
    source_recipe: &AnalysisRecipeV4,
    resolved_model: &qpls_core::SemModelV4,
    workload_limits: Option<CbsemExactCaseBootstrapSourceWorkloadLimitsV1>,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(CbsemCompiledMomentProgressV2) + Sync),
) -> Result<CbsemExactCaseBootstrapSourceV1, CbsemCompiledMomentErrorV2> {
    cbsem_moment_checkpoint(should_cancel, progress, "integrity", 0, 4)?;
    validate_compiled_analysis_recipe_v4(artifact, source_recipe, Some(resolved_model))?;
    if artifact.receipt().dataset_fingerprint() != dataset.fingerprint.0.as_str() {
        return Err(CbsemCompiledMomentErrorV2::DatasetFingerprintMismatch);
    }
    let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
        return Err(CbsemCompiledMomentErrorV2::WrongCompiledPlan);
    };
    let actual_dataset_id = dataset.id.to_string();
    if plan.input().dataset_id() != actual_dataset_id {
        return Err(CbsemCompiledMomentErrorV2::DatasetIdMismatch {
            expected: plan.input().dataset_id().into(),
            actual: actual_dataset_id,
        });
    }
    let modeled_variable_count =
        cbsem_exact_case_bootstrap_modeled_variable_count_v1(resolved_model);
    if let Some(limits) = workload_limits {
        if modeled_variable_count > limits.maximum_modeled_variable_count {
            return Err(
                CbsemCompiledMomentErrorV2::ExactCaseBootstrapModeledVariableLimit {
                    actual: modeled_variable_count,
                    maximum: limits.maximum_modeled_variable_count,
                },
            );
        }
    }
    if workload_limits.is_none() {
        validate_dataset_integrity(dataset)?;
    }
    let source_dataset_fingerprint = dataset
        .fingerprint
        .0
        .strip_prefix("v2:")
        .unwrap_or(&dataset.fingerprint.0);
    if source_dataset_fingerprint.len() != 64
        || !source_dataset_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CbsemCompiledMomentErrorV2::DatasetIntegrity(
            "exact case-bootstrap source fingerprint has no canonical lowercase SHA-256 payload"
                .into(),
        ));
    }
    if dataset.schema.kind != DataKind::Raw
        || dataset.schema.sample_size.is_some()
        || source_recipe.settings.preprocessing != Preprocessing::Unstandardized
        || source_recipe.settings.bootstrap_samples != 0
        || source_recipe.settings.studentized_inner_samples != 0
        || source_recipe.settings.permutation_samples != 0
    {
        return Err(CbsemCompiledMomentErrorV2::ExactCaseBootstrapUnsupported);
    }
    cbsem_moment_checkpoint(should_cancel, progress, "integrity", 1, 4)?;

    let CompiledCbsemInputV2::Raw {
        missing_data,
        weight,
        cluster_variable,
        strata_variable,
        ..
    } = plan.input()
    else {
        return Err(CbsemCompiledMomentErrorV2::ExactCaseBootstrapUnsupported);
    };
    if !matches!(
        missing_data,
        qpls_core::MissingDataPolicyV4::ListwiseDeletion
    ) || weight.is_some()
        || cluster_variable.is_some()
        || strata_variable.is_some()
        || !matches!(plan.group(), SemGroupV4::SingleGroup)
    {
        return Err(CbsemCompiledMomentErrorV2::ExactCaseBootstrapUnsupported);
    }
    let mut projection = build_exact_projection_v3(plan, source_recipe)?;
    if projection.mean_structure || !projection.model.paths.is_empty() {
        return Err(CbsemCompiledMomentErrorV2::ExactCaseBootstrapUnsupported);
    }
    let bounded_parameter_dimensions = if let Some(limits) = workload_limits {
        let (free_parameter_row_count, optimizer_dimension_count) =
            exact_case_bootstrap_source_parameter_dimensions_v1(&mut projection.parameter_rows)?;
        if free_parameter_row_count > limits.maximum_free_parameter_row_count
            || optimizer_dimension_count > limits.maximum_optimizer_dimension_count
        {
            return Err(
                CbsemCompiledMomentErrorV2::ExactCaseBootstrapParameterDimensionLimit {
                    actual_free_parameter_rows: free_parameter_row_count,
                    maximum_free_parameter_rows: limits.maximum_free_parameter_row_count,
                    actual_optimizer_dimensions: optimizer_dimension_count,
                    maximum_optimizer_dimensions: limits.maximum_optimizer_dimension_count,
                },
            );
        }
        Some((free_parameter_row_count, optimizer_dimension_count))
    } else {
        None
    };
    if workload_limits.is_some() {
        validate_dataset_integrity(dataset)?;
    }
    cbsem_moment_checkpoint(should_cancel, progress, "projection", 2, 4)?;

    let positions = resolve_raw_columns(dataset, &projection.indicator_names)?;
    let mut complete_source_row_indices = Vec::new();
    let mut complete_rows = Vec::new();
    for source_row in 0..dataset.batch.num_rows() {
        if source_row % MEAN_REPLACEMENT_CANCELLATION_POLL_INTERVAL_V1 == 0 {
            if should_cancel() {
                return Err(CbsemCompiledMomentErrorV2::Cancelled);
            }
            progress(CbsemCompiledMomentProgressV2 {
                phase: "listwise_source".into(),
                completed_units: source_row as u64,
                total_units: dataset.batch.num_rows() as u64,
            });
        }
        if positions
            .iter()
            .any(|position| dataset.batch.column(*position).is_null(source_row))
        {
            continue;
        }
        if let Some(limits) = workload_limits {
            if complete_rows.len() == limits.maximum_complete_case_sample_size {
                return Err(
                    CbsemCompiledMomentErrorV2::ExactCaseBootstrapCompleteCaseLimit {
                        actual: complete_rows.len() + 1,
                        maximum: limits.maximum_complete_case_sample_size,
                    },
                );
            }
        }
        let mut values = Vec::with_capacity(positions.len());
        for (column_index, position) in positions.iter().copied().enumerate() {
            let value = numeric_cell(dataset.batch.column(position).as_ref(), source_row).ok_or(
                CbsemCompiledMomentErrorV2::MatrixCellInvalid {
                    row: source_row,
                    column: column_index,
                },
            )?;
            if !value.is_finite() {
                return Err(CbsemCompiledMomentErrorV2::RawValueNonFinite {
                    column: projection.indicator_names[column_index].clone(),
                    row: source_row,
                });
            }
            values.push(value);
        }
        complete_source_row_indices.push(source_row);
        complete_rows.push(values);
    }
    if complete_rows.len() < 10 {
        return Err(
            CbsemCompiledMomentErrorV2::ExactCaseBootstrapInsufficientObservations(
                complete_rows.len(),
            ),
        );
    }
    cbsem_moment_checkpoint(should_cancel, progress, "listwise_source", 4, 4)?;
    let complete_case_universe_sha256 = cbsem_exact_case_bootstrap_complete_case_universe_digest_v1(
        source_dataset_fingerprint,
        dataset.batch.num_rows(),
        &complete_source_row_indices,
    );
    let (free_parameter_row_count, optimizer_dimension_count) =
        if let Some(dimensions) = bounded_parameter_dimensions {
            dimensions
        } else {
            exact_case_bootstrap_source_parameter_dimensions_v1(&mut projection.parameter_rows)?
        };

    Ok(CbsemExactCaseBootstrapSourceV1 {
        source_dataset_id: dataset.id.to_string(),
        source_dataset_fingerprint: source_dataset_fingerprint.into(),
        compiler_analytical_identity_sha256: artifact.receipt().analytical_identity_sha256().into(),
        plan_sha256: artifact.receipt().plan_sha256().into(),
        model_scientific_sha256: artifact.receipt().model_scientific_sha256().into(),
        source_row_count: dataset.batch.num_rows(),
        complete_case_universe_sha256,
        complete_source_row_indices,
        complete_rows,
        model: projection.model,
        indicator_names: projection.indicator_names,
        parameter_rows: projection.parameter_rows,
        modeled_variable_count,
        free_parameter_row_count,
        optimizer_dimension_count,
    })
}

/// Refit the cached exact-v3 CFA source over ordered, with-replacement
/// positions in its validated complete-case universe.
pub fn estimate_cbsem_ml_exact_case_resample_v1(
    source: &CbsemExactCaseBootstrapSourceV1,
    sampling_positions: &[usize],
) -> Result<CbsemExactCaseBootstrapRefitV1, CbsemCompiledMomentErrorV2> {
    estimate_cbsem_ml_exact_case_resample_v1_with_control(
        source,
        sampling_positions,
        || false,
        |_| {},
    )
}

pub fn estimate_cbsem_ml_exact_case_resample_v1_with_control(
    source: &CbsemExactCaseBootstrapSourceV1,
    sampling_positions: &[usize],
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(CbsemCompiledMomentProgressV2) + Sync,
) -> Result<CbsemExactCaseBootstrapRefitV1, CbsemCompiledMomentErrorV2> {
    estimate_cbsem_ml_exact_case_resample_impl_with_control(
        source,
        sampling_positions,
        source.complete_case_sample_size(),
        false,
        should_cancel,
        progress,
    )
    .map(|(refit, _)| refit)
}

/// Opt-in exact refit for the analytically studentized scheduler. Point
/// estimation remains successful when expected-information SEs are typed
/// unavailable.
pub fn estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1(
    source: &CbsemExactCaseBootstrapSourceV1,
    sampling_positions: &[usize],
) -> Result<CbsemExactCaseBootstrapRefitWithAnalyticStandardErrorsV1, CbsemCompiledMomentErrorV2> {
    estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control(
        source,
        sampling_positions,
        || false,
        |_| {},
    )
}

pub fn estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control(
    source: &CbsemExactCaseBootstrapSourceV1,
    sampling_positions: &[usize],
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(CbsemCompiledMomentProgressV2) + Sync,
) -> Result<CbsemExactCaseBootstrapRefitWithAnalyticStandardErrorsV1, CbsemCompiledMomentErrorV2> {
    let (refit, standard_errors) = estimate_cbsem_ml_exact_case_resample_impl_with_control(
        source,
        sampling_positions,
        source.complete_case_sample_size(),
        true,
        should_cancel,
        progress,
    )?;
    let standard_errors = standard_errors.ok_or_else(|| {
        CbsemCompiledMomentErrorV2::ExactCaseBootstrapParameterIdentity(
            "opt-in exact refit omitted its analytical standard-error outcome".into(),
        )
    })?;
    Ok(CbsemExactCaseBootstrapRefitWithAnalyticStandardErrorsV1 {
        refit,
        standard_errors,
    })
}

/// Fit the exact validated CFA once after deleting one complete case. The
/// retained N-1 rows use the ML covariance denominator N-1; no bootstrap,
/// score/LM, nested inference, retry, or analytical-SE work is performed.
pub fn estimate_cbsem_ml_exact_case_delete_one_v1(
    source: &CbsemExactCaseBootstrapSourceV1,
    omitted_complete_case_position: usize,
) -> Result<CbsemExactCaseBootstrapDeleteOneRefitV1, CbsemCompiledMomentErrorV2> {
    estimate_cbsem_ml_exact_case_delete_one_v1_with_control(
        source,
        omitted_complete_case_position,
        || false,
        |_| {},
    )
}

pub fn estimate_cbsem_ml_exact_case_delete_one_v1_with_control(
    source: &CbsemExactCaseBootstrapSourceV1,
    omitted_complete_case_position: usize,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(CbsemCompiledMomentProgressV2) + Sync,
) -> Result<CbsemExactCaseBootstrapDeleteOneRefitV1, CbsemCompiledMomentErrorV2> {
    let complete_case_sample_size = source.complete_case_sample_size();
    let retained_observations = complete_case_sample_size.saturating_sub(1);
    if retained_observations < 10 {
        return Err(
            CbsemCompiledMomentErrorV2::ExactCaseBootstrapInsufficientObservations(
                retained_observations,
            ),
        );
    }
    let omitted_source_row_index = source
        .complete_source_row_indices
        .get(omitted_complete_case_position)
        .copied()
        .ok_or(
            CbsemCompiledMomentErrorV2::ExactCaseBootstrapOmittedPositionOutOfRange {
                omitted_position: omitted_complete_case_position,
                complete_case_sample_size,
            },
        )?;
    let retained_positions = (0..complete_case_sample_size)
        .filter(|position| *position != omitted_complete_case_position)
        .collect::<Vec<_>>();
    let (refit, standard_errors) = estimate_cbsem_ml_exact_case_resample_impl_with_control(
        source,
        &retained_positions,
        retained_observations,
        false,
        should_cancel,
        progress,
    )?;
    if standard_errors.is_some() || refit.resampled_observations != retained_observations {
        return Err(
            CbsemCompiledMomentErrorV2::ExactCaseBootstrapParameterIdentity(
                "delete-one refit unexpectedly emitted SEs or the wrong retained row count".into(),
            ),
        );
    }
    Ok(CbsemExactCaseBootstrapDeleteOneRefitV1 {
        method_version: CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1.into(),
        estimator_method_version: refit.estimator_method_version,
        source_dataset_id: refit.source_dataset_id,
        source_dataset_fingerprint: refit.source_dataset_fingerprint,
        compiler_analytical_identity_sha256: refit.compiler_analytical_identity_sha256,
        plan_sha256: refit.plan_sha256,
        model_scientific_sha256: refit.model_scientific_sha256,
        source_row_count: refit.source_row_count,
        complete_case_sample_size,
        complete_case_universe_sha256: refit.complete_case_universe_sha256,
        omitted_complete_case_position,
        omitted_source_row_index,
        retained_observations,
        covariance_denominator: refit.covariance_denominator,
        sampling_positions_digest_method: refit.sampling_positions_digest_method,
        retained_sampling_positions_sha256: refit.sampling_positions_sha256,
        sample_indices_digest_method: refit.sample_indices_digest_method,
        retained_sample_indices_sha256: refit.sample_indices_sha256,
        free_parameters: refit.free_parameters,
        iterations: refit.iterations,
        objective: refit.objective,
        gradient_norm: refit.gradient_norm,
    })
}

fn estimate_cbsem_ml_exact_case_resample_impl_with_control(
    source: &CbsemExactCaseBootstrapSourceV1,
    sampling_positions: &[usize],
    expected_sampling_length: usize,
    request_analytic_standard_errors: bool,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(CbsemCompiledMomentProgressV2) + Sync,
) -> Result<
    (
        CbsemExactCaseBootstrapRefitV1,
        Option<crate::CbsemExactCaseBootstrapRefitStandardErrorsV1>,
    ),
    CbsemCompiledMomentErrorV2,
> {
    let (sampling_receipt, _) = exact_case_sampling_receipt_with_expected_length(
        source,
        sampling_positions,
        expected_sampling_length,
        &should_cancel,
    )?;
    let complete_case_sample_size = sampling_receipt.complete_case_sample_size;
    let covariance_ml = covariance_ml_from_sampling_positions_with_control(
        &source.complete_rows,
        sampling_positions,
        &should_cancel,
    )?;
    ensure_strict_positive_definite(&covariance_ml)?;
    cbsem_moment_checkpoint(&should_cancel, &progress, "moments", 1, 2)?;
    let estimator_progress = |completed_units, total_units| {
        progress(CbsemCompiledMomentProgressV2 {
            phase: "estimation".into(),
            completed_units,
            total_units,
        });
    };
    let point = if request_analytic_standard_errors {
        estimate_cbsem_ml_exact_parameter_table_v3_point_refit_with_analytic_standard_errors_with_control(
            &source.model,
            &source.indicator_names,
            &covariance_ml,
            &source.parameter_rows,
            complete_case_sample_size,
            &should_cancel,
            &estimator_progress,
        )
    } else {
        estimate_cbsem_ml_exact_parameter_table_v3_point_refit_with_control(
            &source.model,
            &source.indicator_names,
            &covariance_ml,
            &source.parameter_rows,
            &should_cancel,
            &estimator_progress,
        )
    }
    .map_err(map_exact_parameter_error)?;
    let expected_parameter_ids = source
        .parameter_rows
        .iter()
        .filter(|row| {
            matches!(
                &row.specification,
                CbsemExactParameterSpecificationV3::Free { .. }
            )
        })
        .map(|row| row.stable_id.as_str())
        .collect::<Vec<_>>();
    let actual_parameter_ids = point
        .free_parameters
        .iter()
        .map(|parameter| parameter.parameter_id.as_str())
        .collect::<Vec<_>>();
    if expected_parameter_ids.is_empty() || actual_parameter_ids != expected_parameter_ids {
        return Err(
            CbsemCompiledMomentErrorV2::ExactCaseBootstrapParameterIdentity(
                "point-only refit did not preserve exact compiled free-row identity and order"
                    .into(),
            ),
        );
    }
    cbsem_moment_checkpoint(&should_cancel, &progress, "result", 2, 2)?;

    let standard_errors = point.standard_errors;
    let refit = CbsemExactCaseBootstrapRefitV1 {
        method_version: CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1.into(),
        estimator_method_version: point.method_version,
        source_dataset_id: source.source_dataset_id.clone(),
        source_dataset_fingerprint: source.source_dataset_fingerprint.clone(),
        compiler_analytical_identity_sha256: source.compiler_analytical_identity_sha256.clone(),
        plan_sha256: source.plan_sha256.clone(),
        model_scientific_sha256: source.model_scientific_sha256.clone(),
        source_row_count: source.source_row_count,
        complete_case_sample_size,
        complete_case_universe_digest_method:
            CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1.into(),
        complete_case_universe_sha256: source.complete_case_universe_sha256.clone(),
        resampled_observations: sampling_positions.len(),
        covariance_denominator: SemCovarianceDenominatorV4::MaximumLikelihoodN,
        sample_indices_digest_method: sampling_receipt.sample_indices_digest_method,
        sampling_positions_digest_method: sampling_receipt.sampling_positions_digest_method,
        sampling_positions_sha256: sampling_receipt.sampling_positions_sha256,
        sample_indices_sha256: sampling_receipt.sample_indices_sha256,
        free_parameters: point.free_parameters,
        iterations: point.iterations,
        objective: point.objective,
        gradient_norm: point.gradient_norm,
    };
    Ok((refit, standard_errors))
}

pub fn cbsem_exact_case_bootstrap_complete_case_universe_digest_v1(
    source_dataset_fingerprint: &str,
    source_row_count: usize,
    complete_source_row_indices: &[usize],
) -> String {
    let mut digest = Sha256::new();
    digest.update(CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1.as_bytes());
    digest.update([0]);
    digest.update((source_dataset_fingerprint.len() as u64).to_le_bytes());
    digest.update(source_dataset_fingerprint.as_bytes());
    digest.update((source_row_count as u64).to_le_bytes());
    digest.update((complete_source_row_indices.len() as u64).to_le_bytes());
    for source_row in complete_source_row_indices {
        digest.update((*source_row as u64).to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

pub fn cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
    complete_case_sample_size: usize,
    sampling_positions: &[usize],
) -> String {
    let mut digest = Sha256::new();
    digest.update(CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1.as_bytes());
    digest.update([0]);
    digest.update((complete_case_sample_size as u64).to_le_bytes());
    digest.update((sampling_positions.len() as u64).to_le_bytes());
    for position in sampling_positions {
        digest.update((*position as u64).to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

pub fn cbsem_exact_case_bootstrap_index_digest_v1(
    source_dataset_fingerprint: &str,
    source_row_count: usize,
    source_row_indices: &[usize],
) -> String {
    let mut digest = Sha256::new();
    digest.update(CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1.as_bytes());
    digest.update([0]);
    digest.update((source_dataset_fingerprint.len() as u64).to_le_bytes());
    digest.update(source_dataset_fingerprint.as_bytes());
    digest.update((source_row_count as u64).to_le_bytes());
    digest.update((source_row_indices.len() as u64).to_le_bytes());
    for source_row in source_row_indices {
        digest.update((*source_row as u64).to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn map_exact_parameter_error(error: CbsemExactParameterTableErrorV3) -> CbsemCompiledMomentErrorV2 {
    match error {
        CbsemExactParameterTableErrorV3::Cancelled => CbsemCompiledMomentErrorV2::Cancelled,
        other => CbsemCompiledMomentErrorV2::ExactParameterTable(other),
    }
}

fn map_mean_replacement_error(
    error: ContinuousRawMeanReplacementErrorV1,
) -> CbsemCompiledMomentErrorV2 {
    match error {
        ContinuousRawMeanReplacementErrorV1::Cancelled => CbsemCompiledMomentErrorV2::Cancelled,
        other => CbsemCompiledMomentErrorV2::MeanReplacement(other),
    }
}

fn mean_cells(names: &[String], values: &[f64]) -> Vec<CbsemMeanCellV4> {
    names
        .iter()
        .zip(values)
        .map(|(variable, value)| CbsemMeanCellV4 {
            variable: variable.clone(),
            value: *value,
        })
        .collect()
}

fn cbsem_moment_checkpoint(
    should_cancel: &impl Fn() -> bool,
    progress: &impl Fn(CbsemCompiledMomentProgressV2),
    phase: &str,
    completed_units: u64,
    total_units: u64,
) -> Result<(), CbsemCompiledMomentErrorV2> {
    if should_cancel() {
        return Err(CbsemCompiledMomentErrorV2::Cancelled);
    }
    progress(CbsemCompiledMomentProgressV2 {
        phase: phase.into(),
        completed_units: completed_units.min(total_units),
        total_units: total_units.max(1),
    });
    Ok(())
}

pub(crate) fn validate_dataset_integrity(
    dataset: &Dataset,
) -> Result<(), CbsemCompiledMomentErrorV2> {
    let bytes = write_arrow(&dataset.batch)
        .map_err(|error| CbsemCompiledMomentErrorV2::DatasetIntegrity(error.to_string()))?;
    dataset_from_descriptor(DatasetDescriptor::from(dataset), &bytes)
        .map(|_| ())
        .map_err(|error| CbsemCompiledMomentErrorV2::DatasetIntegrity(error.to_string()))
}

fn build_exact_projection_v3(
    plan: &CompiledCbsemPlanV2,
    source_recipe: &AnalysisRecipeV4,
) -> Result<ExactProjectionV3, CbsemCompiledMomentErrorV2> {
    let mean_structure = matches!(
        source_recipe.method_config.as_ref(),
        Some(qpls_core::MethodConfig::Cbsem {
            mean_structure: true,
            ..
        })
    );
    let mean_replacement = source_recipe.settings.missing_data
        == qpls_core::MissingDataPolicy::MeanReplacement
        && matches!(
            plan.input(),
            CompiledCbsemInputV2::Raw {
                missing_data: qpls_core::MissingDataPolicyV4::MeanReplacement,
                ..
            }
        );
    build_exact_projection_v3_with_scope(
        plan,
        ExactProjectionScopeV3 {
            model_id: source_recipe.id,
            mean_structure,
            mean_replacement,
            grouping_variable: None,
            metadata: Some(&source_recipe.metadata),
        },
    )
}

pub(crate) fn build_exact_two_group_cfa_projection_v1(
    plan: &CompiledCbsemPlanV2,
    grouping_variable: &str,
) -> Result<ExactProjectionV3, CbsemCompiledMomentErrorV2> {
    build_exact_projection_v3_with_scope(
        plan,
        ExactProjectionScopeV3 {
            model_id: uuid::Uuid::from_u128(0xCB5E_2000_0000_0000_0000_0000_0000_0001),
            mean_structure: false,
            mean_replacement: false,
            grouping_variable: Some(grouping_variable),
            metadata: None,
        },
    )
}

struct ExactProjectionScopeV3<'a> {
    model_id: uuid::Uuid,
    mean_structure: bool,
    mean_replacement: bool,
    grouping_variable: Option<&'a str>,
    metadata: Option<&'a BTreeMap<String, String>>,
}

fn build_exact_projection_v3_with_scope(
    plan: &CompiledCbsemPlanV2,
    scope: ExactProjectionScopeV3<'_>,
) -> Result<ExactProjectionV3, CbsemCompiledMomentErrorV2> {
    let mut issues = Vec::new();
    let ExactProjectionScopeV3 {
        model_id,
        mean_structure,
        mean_replacement,
        grouping_variable,
        metadata,
    } = scope;
    let group_surface_matches = match (plan.group(), grouping_variable) {
        (SemGroupV4::SingleGroup, None) => true,
        (
            SemGroupV4::ObservedGroups {
                grouping_variable: declared,
                ..
            },
            Some(expected),
        ) => declared == expected,
        _ => false,
    };
    if !group_surface_matches {
        issue(
            &mut issues,
            "group_unsupported",
            "group",
            "Compiled moment-input ML is single-group only.",
        );
    }
    if mean_structure {
        if !matches!(plan.input(), CompiledCbsemInputV2::Raw { .. }) {
            issue(
                &mut issues,
                "mean_structure_raw_input_required",
                "data_binding",
                "Observed intercepts and latent means are executable only from raw continuous data in this slice.",
            );
        }
        if !plan.regressions().is_empty() {
            issue(
                &mut issues,
                "mean_structure_cfa_required",
                "structural_model",
                "Raw-data mean structure is limited to CFA; general structural intercepts and latent outcome means remain blocked.",
            );
        }
    }
    if !plan.constraints().is_empty() {
        issue(
            &mut issues,
            "constraints_unsupported",
            "constraints",
            "Explicit constraint objects are outside this slice. Use fixed/free parameter status, finite parameter bounds, and equality_label; linear/effects-coding constraints remain unsupported.",
        );
    }
    for key in [
        "cbsem_imply_all_exogenous_latent_correlations",
        "cbsem_imply_causal_indicator_correlations",
        "cbsem_fix_causal_indicator_variances_to_one",
    ] {
        if metadata
            .and_then(|metadata| metadata.get(key))
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
        {
            issue(
                &mut issues,
                "special_assumption_requires_materialized_parameters",
                key,
                "A CB-SEM Special Assumption cannot remain metadata-only. The compiler must materialize every implied covariance or fixed variance as a stable parameter-table row before estimation.",
            );
        }
    }
    if !plan.derived_terms().is_empty() || !plan.derived_variables().is_empty() {
        issue(
            &mut issues,
            "derived_terms_unsupported",
            "derived_terms",
            "Derived variables and interactions are outside this bounded optimizer slice.",
        );
    }
    if !plan.causal_measurements().is_empty() || !plan.composites().is_empty() {
        issue(
            &mut issues,
            "composites_unsupported",
            "measurement_model",
            "Compiled CB-SEM moment-input ML accepts common factors only.",
        );
    }

    let variables = plan
        .variables()
        .iter()
        .map(|variable| (variable.id(), variable))
        .collect::<HashMap<_, _>>();
    let parameters = plan
        .parameters()
        .iter()
        .map(|parameter| (parameter.id(), parameter))
        .collect::<HashMap<_, _>>();
    let mut observed_sources = BTreeMap::new();
    for variable in plan.variables() {
        if let SemVariableV4::Observed {
            id,
            source_column,
            scale,
            role,
            categories,
            value_labels,
            missing_markers,
            transformation_lineage,
            ..
        } = variable
        {
            if grouping_variable == Some(id.as_str()) {
                continue;
            }
            if *scale != ObservedScaleV4::Continuous
                || *role != ObservedRoleV4::Indicator
                || !categories.is_empty()
                || !value_labels.is_empty()
                || (!missing_markers.is_empty() && !mean_replacement)
                || !transformation_lineage.is_empty()
            {
                issue(
                    &mut issues,
                    "observed_variable_unsupported",
                    id,
                    "Observed moment-input variables must be untransformed continuous indicators; missing-marker provenance is accepted only for exact raw mean replacement.",
                );
            }
            if observed_sources
                .insert(id.clone(), source_column.clone())
                .is_some()
            {
                issue(
                    &mut issues,
                    "observed_variable_duplicate",
                    id,
                    "Observed variable ids must be unique.",
                );
            }
        }
    }
    let duplicate_source = observed_sources
        .values()
        .fold(HashMap::<&str, usize>::new(), |mut counts, value| {
            *counts.entry(value).or_default() += 1;
            counts
        })
        .into_iter()
        .find_map(|(source, count)| (count > 1).then_some(source));
    if let Some(source) = duplicate_source {
        issue(
            &mut issues,
            "source_column_duplicate",
            source,
            "Each observed variable must bind a distinct source column.",
        );
    }

    let mut loadings_by_factor = BTreeMap::<String, Vec<(String, String)>>::new();
    let mut measured_counts = HashMap::<String, usize>::new();
    for loading in plan.loadings() {
        *measured_counts
            .entry(loading.indicator().to_owned())
            .or_default() += 1;
        loadings_by_factor
            .entry(loading.factor().to_owned())
            .or_default()
            .push((
                loading.indicator().to_owned(),
                loading.parameter_id().to_owned(),
            ));
    }
    for (indicator, count) in &measured_counts {
        if *count != 1 {
            issue(
                &mut issues,
                "cross_loading_unsupported",
                indicator,
                "Each indicator must load on exactly one factor in this bounded slice.",
            );
        }
    }
    let measured = measured_counts.keys().cloned().collect::<BTreeSet<_>>();
    let observed = observed_sources.keys().cloned().collect::<BTreeSet<_>>();
    if measured != observed {
        issue(
            &mut issues,
            "unmeasured_observed_variable",
            "variables",
            "Every bound observed variable must be measured exactly once.",
        );
    }

    let factor_ids = plan
        .factors()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let factor_set = factor_ids.iter().cloned().collect::<HashSet<_>>();
    let endogenous = plan
        .regressions()
        .iter()
        .map(|regression| regression.target().to_owned())
        .collect::<HashSet<_>>();
    let mut constructs = Vec::new();
    let mut parameter_rows = Vec::new();
    let mut parameter_ids = BTreeMap::new();
    let mut consumed_parameter_ids = HashSet::new();
    let mut latent_mean_marker_observed_ids = HashSet::new();

    for factor_id in &factor_ids {
        let Some(SemVariableV4::CommonFactor {
            label,
            identification,
            mean_policy,
            disturbance_policy,
            ..
        }) = variables.get(factor_id.as_str()).copied()
        else {
            issue(
                &mut issues,
                "factor_missing",
                factor_id,
                "Compiled factor is missing from the variable table.",
            );
            continue;
        };
        let marker = match identification {
            FactorIdentificationV4::MarkerLoading { indicator } => Some(indicator.as_str()),
            FactorIdentificationV4::FixedVariance => None,
            FactorIdentificationV4::EffectsCoding => {
                issue(
                    &mut issues,
                    "effects_coding_unsupported",
                    factor_id,
                    "Effects-coding identification requires a linear constraint and remains outside this exact parameter-table slice.",
                );
                continue;
            }
        };
        let latent_mean_parameter_id = match mean_policy {
            FactorMeanPolicyV4::FixedZero => None,
            FactorMeanPolicyV4::Estimated { parameter } if mean_structure => {
                if marker.is_none() {
                    issue(
                        &mut issues,
                        "latent_mean_marker_identification_required",
                        factor_id,
                        "An estimated latent mean requires marker-loading identification and a fixed marker-indicator intercept.",
                    );
                }
                Some(parameter.as_str())
            }
            FactorMeanPolicyV4::Estimated { .. } => {
                issue(
                    &mut issues,
                    "mean_structure_unsupported",
                    factor_id,
                    "Factor means must be fixed at zero when mean_structure is disabled.",
                );
                None
            }
            FactorMeanPolicyV4::ReferenceGroup { .. } => {
                issue(
                    &mut issues,
                    "latent_mean_reference_group_unsupported",
                    factor_id,
                    "Reference-group latent means require multigroup estimation and remain blocked.",
                );
                None
            }
        };
        let mut factor_loadings = loadings_by_factor.remove(factor_id).unwrap_or_default();
        factor_loadings.sort_by(|left, right| left.0.cmp(&right.0));
        if factor_loadings.len() < 2
            || marker
                .is_some_and(|marker| !factor_loadings.iter().any(|item| item.0.as_str() == marker))
        {
            issue(
                &mut issues,
                "measurement_block_invalid",
                factor_id,
                "Every factor requires at least two indicators and marker identification must name one of them.",
            );
        }
        let mut indicator_ids = factor_loadings
            .iter()
            .map(|item| item.0.clone())
            .collect::<Vec<_>>();
        indicator_ids.sort();
        if let Some(marker) = marker {
            if let Some(position) = indicator_ids.iter().position(|id| id == marker) {
                indicator_ids.swap(0, position);
            }
        }
        let indicator_columns = indicator_ids
            .iter()
            .filter_map(|id| observed_sources.get(id).cloned())
            .collect::<Vec<_>>();
        constructs.push(Construct {
            id: factor_id.clone(),
            name: label.clone(),
            short_name: label.clone(),
            mode: MeasurementMode::Reflective,
            indicators: indicator_columns,
        });

        for (indicator_id, parameter_id) in factor_loadings {
            let Some(parameter) = parameters.get(parameter_id.as_str()).copied() else {
                continue;
            };
            consumed_parameter_ids.insert(parameter_id.clone());
            let Some(source) = observed_sources.get(&indicator_id) else {
                continue;
            };
            let internal_name = format!("{factor_id}=~{source}");
            insert_parameter_identity(
                &mut parameter_ids,
                &internal_name,
                &parameter_id,
                &mut issues,
            );
            if marker.is_some_and(|marker| indicator_id == marker) {
                if !matches!(
                    parameter.specification(),
                    CompiledCbsemParameterStatusV2::Fixed { value }
                        if value.to_bits() == 1.0_f64.to_bits()
                ) {
                    issue(
                        &mut issues,
                        "marker_loading_invalid",
                        &parameter_id,
                        "The marker loading must be fixed exactly to one.",
                    );
                }
            }
            push_exact_parameter_row(
                parameter,
                internal_name,
                CbsemExactParameterTargetV3::Loading {
                    construct: factor_id.clone(),
                    indicator: source.clone(),
                },
                &mut parameter_rows,
                &mut issues,
            );
        }

        if let Some(mean_parameter_id) = latent_mean_parameter_id {
            if let Some(marker) = marker {
                latent_mean_marker_observed_ids.insert(marker.to_owned());
            }
            if let Some(parameter) = parameters.get(mean_parameter_id).copied() {
                if parameter.target()
                    != &(SemParameterTargetV4::Mean {
                        variable: factor_id.clone(),
                    })
                    || !matches!(
                        parameter.specification(),
                        CompiledCbsemParameterStatusV2::Free { .. }
                    )
                {
                    issue(
                        &mut issues,
                        "latent_mean_parameter_invalid",
                        mean_parameter_id,
                        "An estimated factor mean requires one matching free Mean parameter; fixed latent zero is represented by FactorMeanPolicyV4::FixedZero.",
                    );
                }
                consumed_parameter_ids.insert(mean_parameter_id.to_owned());
                let internal_name = format!("{factor_id}~1");
                insert_parameter_identity(
                    &mut parameter_ids,
                    &internal_name,
                    mean_parameter_id,
                    &mut issues,
                );
                push_exact_parameter_row(
                    parameter,
                    internal_name,
                    CbsemExactParameterTargetV3::LatentMean {
                        factor: factor_id.clone(),
                    },
                    &mut parameter_rows,
                    &mut issues,
                );
            } else {
                issue(
                    &mut issues,
                    "latent_mean_parameter_missing",
                    mean_parameter_id,
                    "The factor mean policy must reference one explicit latent-mean parameter row.",
                );
            }
        }

        let variance_parameter_id = match disturbance_policy {
            qpls_core::FactorDisturbancePolicyV4::ExogenousVariance { parameter }
            | qpls_core::FactorDisturbancePolicyV4::EndogenousDisturbance { parameter } => {
                parameter
            }
            qpls_core::FactorDisturbancePolicyV4::FixedZero { parameter } => {
                issue(
                    &mut issues,
                    "fixed_zero_disturbance_unsupported",
                    parameter,
                    "Fixed-zero factor disturbances are outside this bounded slice.",
                );
                parameter
            }
        };
        if let Some(parameter) = parameters.get(variance_parameter_id.as_str()).copied() {
            consumed_parameter_ids.insert(variance_parameter_id.clone());
            let internal_name = format!("{factor_id}~~{factor_id}");
            insert_parameter_identity(
                &mut parameter_ids,
                &internal_name,
                variance_parameter_id,
                &mut issues,
            );
            if matches!(identification, FactorIdentificationV4::FixedVariance)
                && !matches!(
                    parameter.specification(),
                    CompiledCbsemParameterStatusV2::Fixed { value }
                        if value.to_bits() == 1.0_f64.to_bits()
                )
            {
                issue(
                    &mut issues,
                    "fixed_variance_identification_invalid",
                    variance_parameter_id,
                    "Fixed-variance identification requires the factor variance fixed exactly to one.",
                );
            }
            push_exact_parameter_row(
                parameter,
                internal_name,
                CbsemExactParameterTargetV3::FactorVariance {
                    factor: factor_id.clone(),
                },
                &mut parameter_rows,
                &mut issues,
            );
        }
    }
    if !loadings_by_factor.is_empty() {
        for factor in loadings_by_factor.keys() {
            issue(
                &mut issues,
                "loading_construct_unsupported",
                factor,
                "Loadings must target a compiled common factor.",
            );
        }
    }

    let mut paths = Vec::new();
    for regression in plan.regressions() {
        if !factor_set.contains(regression.source())
            || !factor_set.contains(regression.target())
            || regression.intercept_parameter_id().is_some()
        {
            issue(
                &mut issues,
                "regression_unsupported",
                regression.relation_id(),
                "Structural regressions must connect common factors without intercepts.",
            );
            continue;
        }
        paths.push(StructuralPath {
            source: regression.source().into(),
            target: regression.target().into(),
        });
        if let Some(parameter) = parameters.get(regression.parameter_id()).copied() {
            consumed_parameter_ids.insert(regression.parameter_id().into());
            let internal_name = format!("{}~{}", regression.target(), regression.source());
            insert_parameter_identity(
                &mut parameter_ids,
                &internal_name,
                regression.parameter_id(),
                &mut issues,
            );
            push_exact_parameter_row(
                parameter,
                internal_name,
                CbsemExactParameterTargetV3::Regression {
                    source: regression.source().into(),
                    target: regression.target().into(),
                },
                &mut parameter_rows,
                &mut issues,
            );
        }
    }

    for covariance in plan.covariances() {
        let resolved = match (covariance.left(), covariance.right()) {
            (SemEndpointV4::Variable(left), SemEndpointV4::Variable(right))
                if factor_set.contains(left)
                    && factor_set.contains(right)
                    && !endogenous.contains(left)
                    && !endogenous.contains(right) =>
            {
                let pair = canonical_string_pair(left, right);
                Some((
                    format!("{}~~{}", pair.0, pair.1),
                    CbsemExactParameterTargetV3::FactorCovariance {
                        left: pair.0,
                        right: pair.1,
                        kind: CbsemExactLatentCovarianceKindV3::Exogenous,
                    },
                ))
            }
            (SemEndpointV4::DisturbanceOf(left), SemEndpointV4::DisturbanceOf(right))
                if factor_set.contains(left)
                    && factor_set.contains(right)
                    && endogenous.contains(left)
                    && endogenous.contains(right) =>
            {
                let pair = canonical_string_pair(left, right);
                Some((
                    format!("{}~~{}", pair.0, pair.1),
                    CbsemExactParameterTargetV3::FactorCovariance {
                        left: pair.0,
                        right: pair.1,
                        kind: CbsemExactLatentCovarianceKindV3::Disturbance,
                    },
                ))
            }
            (SemEndpointV4::ResidualOf(left), SemEndpointV4::ResidualOf(right)) => {
                match (observed_sources.get(left), observed_sources.get(right)) {
                    (Some(left_source), Some(right_source)) => {
                        let pair = canonical_string_pair(left_source, right_source);
                        Some((
                            format!("{}~~{}", pair.0, pair.1),
                            CbsemExactParameterTargetV3::ResidualCovariance {
                                left: pair.0,
                                right: pair.1,
                            },
                        ))
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        let Some((internal_name, target)) = resolved else {
            issue(
                &mut issues,
                "covariance_endpoint_semantics_unsupported",
                covariance.relation_id(),
                "Use Variable endpoints for two exogenous factors, DisturbanceOf endpoints for two endogenous factors, or ResidualOf endpoints for two measured indicators. Mixed, observed-variable, and endogenous latent-variable covariance endpoints are not silently reinterpreted.",
            );
            continue;
        };
        if let Some(parameter) = parameters.get(covariance.parameter_id()).copied() {
            consumed_parameter_ids.insert(covariance.parameter_id().into());
            insert_parameter_identity(
                &mut parameter_ids,
                &internal_name,
                covariance.parameter_id(),
                &mut issues,
            );
            push_exact_parameter_row(
                parameter,
                internal_name,
                target,
                &mut parameter_rows,
                &mut issues,
            );
        }
    }

    for (observed_id, source) in &observed_sources {
        let endpoint = SemEndpointV4::ResidualOf(observed_id.clone());
        let candidates = plan
            .parameters()
            .iter()
            .filter(|parameter| {
                parameter.target()
                    == &SemParameterTargetV4::Variance {
                        endpoint: endpoint.clone(),
                    }
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            issue(
                &mut issues,
                "residual_variance_missing",
                observed_id,
                "Every indicator requires exactly one explicit residual variance.",
            );
            continue;
        }
        let parameter = candidates[0];
        consumed_parameter_ids.insert(parameter.id().into());
        let internal_name = format!("{source}~~{source}");
        insert_parameter_identity(
            &mut parameter_ids,
            &internal_name,
            parameter.id(),
            &mut issues,
        );
        push_exact_parameter_row(
            parameter,
            internal_name,
            CbsemExactParameterTargetV3::ResidualVariance {
                indicator: source.clone(),
            },
            &mut parameter_rows,
            &mut issues,
        );
    }

    if mean_structure {
        for (observed_id, source) in &observed_sources {
            let candidates = plan
                .parameters()
                .iter()
                .filter(|parameter| {
                    parameter.target()
                        == &SemParameterTargetV4::Intercept {
                            variable: observed_id.clone(),
                        }
                })
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
                issue(
                    &mut issues,
                    if candidates.is_empty() {
                        "observed_intercept_missing"
                    } else {
                        "observed_intercept_duplicate"
                    },
                    observed_id,
                    "Every measured indicator requires exactly one explicit observed-intercept parameter when mean_structure is enabled.",
                );
                continue;
            }
            let parameter = candidates[0];
            if latent_mean_marker_observed_ids.contains(observed_id)
                && !matches!(
                    parameter.specification(),
                    CompiledCbsemParameterStatusV2::Fixed { .. }
                )
            {
                issue(
                    &mut issues,
                    "latent_mean_marker_intercept_must_be_fixed",
                    parameter.id(),
                    "The intercept of each latent-mean marker indicator must be fixed to anchor factor location.",
                );
            }
            consumed_parameter_ids.insert(parameter.id().into());
            let internal_name = format!("{source}~1");
            insert_parameter_identity(
                &mut parameter_ids,
                &internal_name,
                parameter.id(),
                &mut issues,
            );
            push_exact_parameter_row(
                parameter,
                internal_name,
                CbsemExactParameterTargetV3::ObservedIntercept {
                    indicator: source.clone(),
                },
                &mut parameter_rows,
                &mut issues,
            );
        }
    }

    for parameter in plan.parameters() {
        if parameter.role() == CompiledCbsemParameterRoleV2::Loading
            && matches!(
                parameter.specification(),
                CompiledCbsemParameterStatusV2::Fixed { .. }
            )
        {
            // Valid marker parameters were consumed above. Any other fixed
            // loading remains an unsupported extra and is reported below.
        }
        if !consumed_parameter_ids.contains(parameter.id()) {
            issue(
                &mut issues,
                "parameter_unsupported",
                parameter.id(),
                "This parameter is represented by the plan but not consumed by the bounded optimizer.",
            );
        }
    }

    if !issues.is_empty() {
        issues.sort_by(|left, right| {
            (&left.code, &left.subject, &left.message).cmp(&(
                &right.code,
                &right.subject,
                &right.message,
            ))
        });
        issues.dedup();
        return Err(CbsemCompiledMomentErrorV2::UnsupportedPlan { issues });
    }
    let indicator_names = observed_sources.values().cloned().collect::<Vec<_>>();
    Ok(ExactProjectionV3 {
        model: ModelSpec {
            id: model_id,
            name: plan.model_name().into(),
            constructs,
            paths,
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        },
        indicator_names,
        parameter_rows,
        parameter_ids,
        mean_structure,
    })
}

fn push_exact_parameter_row(
    parameter: &qpls_core::CompiledCbsemParameterRowV2,
    internal_name: String,
    target: CbsemExactParameterTargetV3,
    rows: &mut Vec<CbsemExactParameterRowV3>,
    issues: &mut Vec<CbsemMatrixModelIssueV2>,
) {
    if !parameter.group_overrides().is_empty() {
        issue(
            issues,
            "parameter_group_overrides_unsupported",
            parameter.id(),
            "Single-group exact parameter-table execution does not accept group overrides.",
        );
        return;
    }
    let specification = match parameter.specification() {
        CompiledCbsemParameterStatusV2::Free {
            start,
            lower,
            upper,
            equality_label,
        } => CbsemExactParameterSpecificationV3::Free {
            start: *start,
            lower: *lower,
            upper: *upper,
            equality_label: equality_label.clone(),
        },
        CompiledCbsemParameterStatusV2::Fixed { value } => {
            CbsemExactParameterSpecificationV3::Fixed { value: *value }
        }
        CompiledCbsemParameterStatusV2::Derived { .. } => {
            issue(
                issues,
                "derived_parameter_unsupported",
                parameter.id(),
                "Derived parameter expressions are outside this exact numerical parameter-table slice.",
            );
            return;
        }
    };
    rows.push(CbsemExactParameterRowV3 {
        stable_id: parameter.id().into(),
        name: internal_name,
        target,
        specification,
    });
}

fn insert_parameter_identity(
    identities: &mut BTreeMap<String, String>,
    internal_name: &str,
    stable_id: &str,
    issues: &mut Vec<CbsemMatrixModelIssueV2>,
) {
    if let Some(previous) = identities.insert(internal_name.into(), stable_id.into()) {
        issue(
            issues,
            "parameter_result_name_duplicate",
            internal_name,
            format!(
                "Stable parameters {previous} and {stable_id} map to the same result name; rename the colliding factor or source column."
            ),
        );
    }
}

fn canonical_string_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.into(), right.into())
    } else {
        (right.into(), left.into())
    }
}

fn prepare_moments(
    plan: &CompiledCbsemPlanV2,
    dataset: &Dataset,
    canonical_source_columns: &[String],
    mean_structure: bool,
    should_cancel: &impl Fn() -> bool,
) -> Result<PreparedMomentsV2, CbsemCompiledMomentErrorV2> {
    let actual_id = dataset.id.to_string();
    if plan.input().dataset_id() != actual_id {
        return Err(CbsemCompiledMomentErrorV2::DatasetIdMismatch {
            expected: plan.input().dataset_id().into(),
            actual: actual_id,
        });
    }
    match plan.input() {
        CompiledCbsemInputV2::Raw { .. } => {
            if dataset.schema.kind != DataKind::Raw {
                return Err(CbsemCompiledMomentErrorV2::InputKindMismatch {
                    expected: CbsemMomentInputKindV2::Raw,
                    actual: dataset.schema.kind,
                });
            }
            prepare_raw_moments(
                plan,
                dataset,
                canonical_source_columns,
                mean_structure,
                should_cancel,
            )
        }
        CompiledCbsemInputV2::Covariance {
            variables,
            means,
            standard_deviations,
            sample,
            ..
        } => {
            if mean_structure {
                return Err(CbsemCompiledMomentErrorV2::UnsupportedPlan {
                    issues: vec![CbsemMatrixModelIssueV2 {
                        code: "mean_structure_raw_input_required".into(),
                        subject: "data_binding".into(),
                        message: "Observed intercepts and latent means require raw continuous data; covariance means remain blocked."
                            .into(),
                    }],
                });
            }
            if dataset.schema.kind != DataKind::Covariance {
                return Err(CbsemCompiledMomentErrorV2::InputKindMismatch {
                    expected: CbsemMomentInputKindV2::Covariance,
                    actual: dataset.schema.kind,
                });
            }
            if means.is_some() || standard_deviations.is_some() {
                return Err(CbsemCompiledMomentErrorV2::UnsupportedPlan {
                    issues: vec![CbsemMatrixModelIssueV2 {
                        code: "covariance_moments_unsupported".into(),
                        subject: "data_binding".into(),
                        message:
                            "Covariance input does not accept means or separate scale metadata."
                                .into(),
                    }],
                });
            }
            prepare_matrix_moments(
                plan,
                dataset,
                variables,
                sample.sample_size,
                sample.covariance_denominator,
                None,
                CbsemMomentInputKindV2::Covariance,
                canonical_source_columns,
            )
        }
        CompiledCbsemInputV2::Correlation {
            variables,
            means,
            standard_deviations,
            sample,
            ..
        } => {
            if mean_structure {
                return Err(CbsemCompiledMomentErrorV2::UnsupportedPlan {
                    issues: vec![CbsemMatrixModelIssueV2 {
                        code: "mean_structure_raw_input_required".into(),
                        subject: "data_binding".into(),
                        message: "Observed intercepts and latent means require raw continuous data; correlation means remain blocked."
                            .into(),
                    }],
                });
            }
            if dataset.schema.kind != DataKind::Correlation {
                return Err(CbsemCompiledMomentErrorV2::InputKindMismatch {
                    expected: CbsemMomentInputKindV2::Correlation,
                    actual: dataset.schema.kind,
                });
            }
            if means.is_some() || standard_deviations.is_none() {
                return Err(CbsemCompiledMomentErrorV2::UnsupportedPlan {
                    issues: vec![CbsemMatrixModelIssueV2 {
                        code: "correlation_scale_metadata_required".into(),
                        subject: "data_binding".into(),
                        message: "Correlation input requires standard deviations and does not accept means."
                            .into(),
                    }],
                });
            }
            prepare_matrix_moments(
                plan,
                dataset,
                variables,
                sample.sample_size,
                sample.covariance_denominator,
                standard_deviations.as_ref(),
                CbsemMomentInputKindV2::Correlation,
                canonical_source_columns,
            )
        }
    }
}

fn prepare_raw_moments(
    plan: &CompiledCbsemPlanV2,
    dataset: &Dataset,
    canonical_source_columns: &[String],
    mean_structure: bool,
    should_cancel: &impl Fn() -> bool,
) -> Result<PreparedMomentsV2, CbsemCompiledMomentErrorV2> {
    let CompiledCbsemInputV2::Raw {
        missing_data,
        weight,
        cluster_variable,
        strata_variable,
        ..
    } = plan.input()
    else {
        unreachable!()
    };
    if matches!(
        missing_data,
        qpls_core::MissingDataPolicyV4::MeanReplacement
    ) {
        if weight.is_some() || cluster_variable.is_some() || strata_variable.is_some() {
            return Err(CbsemCompiledMomentErrorV2::UnsupportedPlan {
                issues: vec![CbsemMatrixModelIssueV2 {
                    code: "raw_data_options_unsupported".into(),
                    subject: "data_binding".into(),
                    message: "Compiled raw mean replacement does not support weights, clusters, or strata."
                        .into(),
                }],
            });
        }
        if mean_structure {
            return Err(CbsemCompiledMomentErrorV2::UnsupportedPlan {
                issues: vec![CbsemMatrixModelIssueV2 {
                    code: "mean_replacement_mean_structure_unsupported".into(),
                    subject: "method_config".into(),
                    message:
                        "The bounded mean-replacement v1 slice estimates covariance structure only."
                            .into(),
                }],
            });
        }
        return prepare_raw_mean_replacement_moments(
            plan,
            dataset,
            canonical_source_columns,
            should_cancel,
        );
    }
    if !matches!(
        missing_data,
        qpls_core::MissingDataPolicyV4::ListwiseDeletion
    ) || weight.is_some()
        || cluster_variable.is_some()
        || strata_variable.is_some()
    {
        return Err(CbsemCompiledMomentErrorV2::UnsupportedPlan {
            issues: vec![CbsemMatrixModelIssueV2 {
                code: "raw_data_options_unsupported".into(),
                subject: "data_binding".into(),
                message: "Compiled raw ML requires listwise deletion without weights, clusters, or strata."
                    .into(),
            }],
        });
    }
    if dataset.schema.sample_size.is_some() {
        return Err(CbsemCompiledMomentErrorV2::SampleSizeMismatch {
            declared: dataset.batch.num_rows(),
            actual: dataset.schema.sample_size,
        });
    }
    let positions = resolve_raw_columns(dataset, canonical_source_columns)?;
    let mut complete_rows = Vec::new();
    for row in 0..dataset.batch.num_rows() {
        if positions
            .iter()
            .any(|position| dataset.batch.column(*position).is_null(row))
        {
            continue;
        }
        let mut values = Vec::with_capacity(positions.len());
        for (column_index, position) in positions.iter().enumerate() {
            let value = numeric_cell(dataset.batch.column(*position).as_ref(), row).ok_or(
                CbsemCompiledMomentErrorV2::MatrixCellInvalid {
                    row,
                    column: column_index,
                },
            )?;
            if !value.is_finite() {
                return Err(CbsemCompiledMomentErrorV2::RawValueNonFinite {
                    column: canonical_source_columns[column_index].clone(),
                    row,
                });
            }
            values.push(value);
        }
        complete_rows.push(values);
    }
    if complete_rows.len() < 10 {
        return Err(CbsemCompiledMomentErrorV2::InsufficientObservations(
            complete_rows.len(),
        ));
    }
    let covariance_ml = covariance_ml_from_rows(&complete_rows);
    let observed_means = mean_structure.then(|| means_from_rows(&complete_rows));
    ensure_strict_positive_definite(&covariance_ml)?;
    let variable_ids = canonical_observed_ids(plan, canonical_source_columns)?;
    let covariance_hash = covariance_sha256(
        CbsemMomentInputKindV2::Raw,
        complete_rows.len(),
        &variable_ids,
        SemCovarianceDenominatorV4::MaximumLikelihoodN,
        &covariance_ml,
    );
    let means_hash = observed_means
        .as_ref()
        .map(|means| observed_means_sha256(complete_rows.len(), &variable_ids, means));
    Ok(PreparedMomentsV2 {
        provenance: CbsemMomentInputProvenanceV2 {
            kind: CbsemMomentInputKindV2::Raw,
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            declared_sample_size: None,
            used_sample_size: complete_rows.len(),
            omitted_observations: dataset.batch.num_rows() - complete_rows.len(),
            covariance_denominator: SemCovarianceDenominatorV4::MaximumLikelihoodN,
            variable_ids,
            source_columns: canonical_source_columns.to_vec(),
            standard_deviations: None,
            canonical_ml_covariance_sha256: covariance_hash,
            canonical_observed_means_sha256: means_hash,
            missing_data_treatment: None,
        },
        covariance_ml,
        observed_means,
    })
}

fn prepare_raw_mean_replacement_moments(
    plan: &CompiledCbsemPlanV2,
    dataset: &Dataset,
    canonical_source_columns: &[String],
    should_cancel: &impl Fn() -> bool,
) -> Result<PreparedMomentsV2, CbsemCompiledMomentErrorV2> {
    if dataset.schema.sample_size.is_some() {
        return Err(CbsemCompiledMomentErrorV2::SampleSizeMismatch {
            declared: dataset.batch.num_rows(),
            actual: dataset.schema.sample_size,
        });
    }
    let observed = plan
        .variables()
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id,
                source_column,
                missing_markers,
                ..
            } => Some((source_column.as_str(), (id.as_str(), missing_markers))),
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    let bindings = canonical_source_columns
        .iter()
        .map(|source_column| {
            let (variable_id, missing_markers) = observed
                .get(source_column.as_str())
                .copied()
                .ok_or(CbsemCompiledMomentErrorV2::MatrixVariableBindingMismatch)?;
            Ok(ContinuousRawMeanReplacementVariableBindingV1 {
                variable_id: variable_id.into(),
                source_column: source_column.clone(),
                missing_markers: missing_markers.clone(),
            })
        })
        .collect::<Result<Vec<_>, CbsemCompiledMomentErrorV2>>()?;
    // The execution entrypoint reproduced the dataset fingerprint immediately
    // before projection; use the internal seam to avoid serializing Arrow a
    // second time while preserving that mandatory precondition.
    let prepared = prepare_continuous_raw_mean_replacement_v1_after_integrity_with_control(
        dataset,
        &bindings,
        should_cancel,
    )
    .map_err(map_mean_replacement_error)?;
    if prepared.rows().len() < 10 {
        return Err(CbsemCompiledMomentErrorV2::InsufficientObservations(
            prepared.rows().len(),
        ));
    }
    let covariance_ml = prepared
        .covariance_ml_with_control(should_cancel)
        .map_err(map_mean_replacement_error)?;
    let (rows, receipt) = prepared.into_parts();
    ensure_strict_positive_definite(&covariance_ml)?;
    let variable_ids = receipt
        .variables
        .iter()
        .map(|variable| variable.variable_id.clone())
        .collect::<Vec<_>>();
    let covariance_hash = covariance_sha256_with_control(
        CbsemMomentInputKindV2::Raw,
        rows.len(),
        &variable_ids,
        SemCovarianceDenominatorV4::MaximumLikelihoodN,
        &covariance_ml,
        should_cancel,
    )?;
    Ok(PreparedMomentsV2 {
        provenance: CbsemMomentInputProvenanceV2 {
            kind: CbsemMomentInputKindV2::Raw,
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            declared_sample_size: None,
            used_sample_size: rows.len(),
            omitted_observations: 0,
            covariance_denominator: SemCovarianceDenominatorV4::MaximumLikelihoodN,
            variable_ids,
            source_columns: canonical_source_columns.to_vec(),
            standard_deviations: None,
            canonical_ml_covariance_sha256: covariance_hash,
            canonical_observed_means_sha256: None,
            missing_data_treatment: Some(receipt),
        },
        covariance_ml,
        observed_means: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn prepare_matrix_moments(
    plan: &CompiledCbsemPlanV2,
    dataset: &Dataset,
    binding_variables: &[String],
    declared_sample_size: usize,
    denominator: SemCovarianceDenominatorV4,
    standard_deviations: Option<&BTreeMap<String, f64>>,
    kind: CbsemMomentInputKindV2,
    canonical_source_columns: &[String],
) -> Result<PreparedMomentsV2, CbsemCompiledMomentErrorV2> {
    if dataset.schema.sample_size != Some(declared_sample_size) {
        return Err(CbsemCompiledMomentErrorV2::SampleSizeMismatch {
            declared: declared_sample_size,
            actual: dataset.schema.sample_size,
        });
    }
    if declared_sample_size < 10 {
        return Err(CbsemCompiledMomentErrorV2::InsufficientObservations(
            declared_sample_size,
        ));
    }
    let variable_to_source = observed_source_map(plan);
    let binding_sources = binding_variables
        .iter()
        .map(|id| variable_to_source.get(id).cloned())
        .collect::<Option<Vec<_>>>()
        .ok_or(CbsemCompiledMomentErrorV2::MatrixVariableBindingMismatch)?;
    let dataset_sources = dataset
        .schema
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect::<Vec<_>>();
    if binding_sources != dataset_sources
        || binding_variables.iter().collect::<BTreeSet<_>>()
            != variable_to_source.keys().collect::<BTreeSet<_>>()
    {
        return Err(CbsemCompiledMomentErrorV2::MatrixVariableBindingMismatch);
    }
    let size = binding_sources.len();
    if dataset.batch.num_rows() != size
        || dataset.batch.num_columns() != size
        || dataset.schema.case_count != size
    {
        return Err(CbsemCompiledMomentErrorV2::MatrixShape {
            expected: size,
            rows: dataset.batch.num_rows(),
            columns: dataset.batch.num_columns(),
        });
    }
    for column in &dataset.schema.columns {
        if column.column_type != ColumnType::Numeric || column.scale_type != ScaleType::Continuous {
            return Err(CbsemCompiledMomentErrorV2::MatrixVariableBindingMismatch);
        }
    }
    let source_positions = binding_sources
        .iter()
        .enumerate()
        .map(|(index, source)| (source.as_str(), index))
        .collect::<HashMap<_, _>>();
    let canonical_positions = canonical_source_columns
        .iter()
        .map(|source| {
            source_positions
                .get(source.as_str())
                .copied()
                .ok_or(CbsemCompiledMomentErrorV2::MatrixVariableBindingMismatch)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut matrix = vec![vec![0.0; size]; size];
    for (canonical_row, source_row) in canonical_positions.iter().copied().enumerate() {
        for (canonical_column, source_column) in canonical_positions.iter().copied().enumerate() {
            let array = dataset.batch.column(source_column);
            if array.is_null(source_row) {
                return Err(CbsemCompiledMomentErrorV2::MatrixCellInvalid {
                    row: source_row,
                    column: source_column,
                });
            }
            let value = numeric_cell(array.as_ref(), source_row).ok_or(
                CbsemCompiledMomentErrorV2::MatrixCellInvalid {
                    row: source_row,
                    column: source_column,
                },
            )?;
            if !value.is_finite() {
                return Err(CbsemCompiledMomentErrorV2::MatrixCellNonFinite {
                    row: source_row,
                    column: source_column,
                });
            }
            matrix[canonical_row][canonical_column] = value;
        }
    }
    validate_symmetric_matrix(&matrix)?;
    if kind == CbsemMomentInputKindV2::Correlation {
        validate_correlation_matrix(&matrix)?;
    }
    let variable_ids = canonical_observed_ids(plan, canonical_source_columns)?;
    let canonical_scales = if let Some(standard_deviations) = standard_deviations {
        let mut scales = BTreeMap::new();
        for (id, source) in variable_ids.iter().zip(canonical_source_columns) {
            let value = standard_deviations
                .get(id)
                .copied()
                .ok_or(CbsemCompiledMomentErrorV2::MatrixVariableBindingMismatch)?;
            if !value.is_finite() || value <= 0.0 {
                return Err(CbsemCompiledMomentErrorV2::MatrixVariableBindingMismatch);
            }
            scales.insert(source.clone(), value);
        }
        for row in 0..size {
            for column in 0..size {
                matrix[row][column] *= scales[&canonical_source_columns[row]]
                    * scales[&canonical_source_columns[column]];
            }
        }
        Some(scales)
    } else {
        None
    };
    if denominator == SemCovarianceDenominatorV4::SampleNMinusOne {
        let scale = (declared_sample_size - 1) as f64 / declared_sample_size as f64;
        for row in &mut matrix {
            for value in row {
                *value *= scale;
            }
        }
    }
    ensure_strict_positive_definite(&matrix)?;
    let covariance_hash = covariance_sha256(
        kind,
        declared_sample_size,
        &variable_ids,
        denominator,
        &matrix,
    );
    Ok(PreparedMomentsV2 {
        provenance: CbsemMomentInputProvenanceV2 {
            kind,
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            declared_sample_size: Some(declared_sample_size),
            used_sample_size: declared_sample_size,
            omitted_observations: 0,
            covariance_denominator: denominator,
            variable_ids,
            source_columns: canonical_source_columns.to_vec(),
            standard_deviations: canonical_scales,
            canonical_ml_covariance_sha256: covariance_hash,
            canonical_observed_means_sha256: None,
            missing_data_treatment: None,
        },
        covariance_ml: matrix,
        observed_means: None,
    })
}

fn observed_source_map(plan: &CompiledCbsemPlanV2) -> BTreeMap<String, String> {
    plan.variables()
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } => Some((id.clone(), source_column.clone())),
            _ => None,
        })
        .collect()
}

fn canonical_observed_ids(
    plan: &CompiledCbsemPlanV2,
    canonical_source_columns: &[String],
) -> Result<Vec<String>, CbsemCompiledMomentErrorV2> {
    let by_source = observed_source_map(plan)
        .into_iter()
        .map(|(id, source)| (source, id))
        .collect::<HashMap<_, _>>();
    canonical_source_columns
        .iter()
        .map(|source| {
            by_source
                .get(source)
                .cloned()
                .ok_or(CbsemCompiledMomentErrorV2::MatrixVariableBindingMismatch)
        })
        .collect()
}

pub(crate) fn resolve_raw_columns(
    dataset: &Dataset,
    names: &[String],
) -> Result<Vec<usize>, CbsemCompiledMomentErrorV2> {
    names
        .iter()
        .map(|name| {
            let position = dataset
                .schema
                .columns
                .iter()
                .position(|column| column.name == *name)
                .ok_or(CbsemCompiledMomentErrorV2::MatrixVariableBindingMismatch)?;
            let column = &dataset.schema.columns[position];
            if column.column_type != ColumnType::Numeric
                || column.scale_type != ScaleType::Continuous
            {
                return Err(CbsemCompiledMomentErrorV2::MatrixVariableBindingMismatch);
            }
            Ok(position)
        })
        .collect()
}

pub(crate) fn numeric_cell(array: &dyn Array, row: usize) -> Option<f64> {
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        Some(values.value(row))
    } else {
        array
            .as_any()
            .downcast_ref::<Int64Array>()
            .map(|values| values.value(row) as f64)
    }
}

pub(crate) fn covariance_ml_from_rows(rows: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let columns = rows[0].len();
    let means = (0..columns)
        .map(|column| rows.iter().map(|row| row[column]).sum::<f64>() / rows.len() as f64)
        .collect::<Vec<_>>();
    let mut covariance = vec![vec![0.0; columns]; columns];
    for row in rows {
        for left in 0..columns {
            let left_centered = row[left] - means[left];
            for right in left..columns {
                covariance[left][right] += left_centered * (row[right] - means[right]);
            }
        }
    }
    for left in 0..columns {
        for right in left..columns {
            covariance[left][right] /= rows.len() as f64;
            covariance[right][left] = covariance[left][right];
        }
    }
    covariance
}

fn covariance_ml_from_sampling_positions_with_control(
    complete_rows: &[Vec<f64>],
    sampling_positions: &[usize],
    should_cancel: &impl Fn() -> bool,
) -> Result<Vec<Vec<f64>>, CbsemCompiledMomentErrorV2> {
    let complete_case_sample_size = complete_rows.len();
    let columns = complete_rows.first().map_or(0, Vec::len);
    let mut means = vec![0.0; columns];
    for (draw_position, sampling_position) in sampling_positions.iter().copied().enumerate() {
        if should_cancel() {
            return Err(CbsemCompiledMomentErrorV2::Cancelled);
        }
        let row = complete_rows.get(sampling_position).ok_or(
            CbsemCompiledMomentErrorV2::ExactCaseBootstrapSamplingPositionOutOfRange {
                draw_position,
                sampling_position,
                complete_case_sample_size,
            },
        )?;
        for (column, value) in means.iter_mut().zip(row) {
            *column += value;
        }
    }
    for mean in &mut means {
        *mean /= sampling_positions.len() as f64;
    }
    let mut covariance = vec![vec![0.0; columns]; columns];
    for (draw_position, sampling_position) in sampling_positions.iter().copied().enumerate() {
        if should_cancel() {
            return Err(CbsemCompiledMomentErrorV2::Cancelled);
        }
        let row = complete_rows.get(sampling_position).ok_or(
            CbsemCompiledMomentErrorV2::ExactCaseBootstrapSamplingPositionOutOfRange {
                draw_position,
                sampling_position,
                complete_case_sample_size,
            },
        )?;
        for left in 0..columns {
            let left_centered = row[left] - means[left];
            for right in left..columns {
                covariance[left][right] += left_centered * (row[right] - means[right]);
            }
        }
    }
    for left in 0..columns {
        for right in left..columns {
            covariance[left][right] /= sampling_positions.len() as f64;
            covariance[right][left] = covariance[left][right];
        }
    }
    Ok(covariance)
}

pub(crate) fn means_from_rows(rows: &[Vec<f64>]) -> Vec<f64> {
    (0..rows[0].len())
        .map(|column| rows.iter().map(|row| row[column]).sum::<f64>() / rows.len() as f64)
        .collect()
}

pub(crate) fn validate_symmetric_matrix(
    matrix: &[Vec<f64>],
) -> Result<(), CbsemCompiledMomentErrorV2> {
    for row in 0..matrix.len() {
        for column in row + 1..matrix.len() {
            let scale = matrix[row][column]
                .abs()
                .max(matrix[column][row].abs())
                .max(1.0);
            if (matrix[row][column] - matrix[column][row]).abs()
                > MATRIX_SYMMETRY_ABS_TOLERANCE * scale
            {
                return Err(CbsemCompiledMomentErrorV2::MatrixNotSymmetric { row, column });
            }
        }
    }
    Ok(())
}

fn validate_correlation_matrix(matrix: &[Vec<f64>]) -> Result<(), CbsemCompiledMomentErrorV2> {
    for row in 0..matrix.len() {
        if (matrix[row][row] - 1.0).abs() > CORRELATION_DIAGONAL_ABS_TOLERANCE {
            return Err(CbsemCompiledMomentErrorV2::CorrelationDiagonalInvalid { index: row });
        }
        for column in 0..matrix.len() {
            if !(-1.0..=1.0).contains(&matrix[row][column]) {
                return Err(CbsemCompiledMomentErrorV2::CorrelationOutOfRange { row, column });
            }
        }
    }
    Ok(())
}

pub(crate) fn ensure_strict_positive_definite(
    matrix: &[Vec<f64>],
) -> Result<(), CbsemCompiledMomentErrorV2> {
    if matrix.is_empty() || matrix.iter().any(|row| row.len() != matrix.len()) {
        return Err(CbsemCompiledMomentErrorV2::MatrixShape {
            expected: matrix.len(),
            rows: matrix.len(),
            columns: matrix.first().map_or(0, Vec::len),
        });
    }
    let scale = matrix
        .iter()
        .enumerate()
        .map(|(index, row)| row[index].abs())
        .fold(1.0_f64, f64::max);
    let tolerance = POSITIVE_DEFINITE_RELATIVE_TOLERANCE * scale;
    let mut lower = vec![vec![0.0; matrix.len()]; matrix.len()];
    for row in 0..matrix.len() {
        for column in 0..=row {
            let prior = (0..column)
                .map(|index| lower[row][index] * lower[column][index])
                .sum::<f64>();
            if row == column {
                let pivot = matrix[row][row] - prior;
                if !pivot.is_finite() || pivot <= tolerance {
                    return Err(CbsemCompiledMomentErrorV2::MatrixNotPositiveDefinite {
                        pivot: row,
                    });
                }
                lower[row][column] = pivot.sqrt();
            } else {
                lower[row][column] = (matrix[row][column] - prior) / lower[column][column];
            }
        }
    }
    Ok(())
}

pub(crate) fn covariance_sha256(
    kind: CbsemMomentInputKindV2,
    sample_size: usize,
    variable_ids: &[String],
    denominator: SemCovarianceDenominatorV4,
    covariance_ml: &[Vec<f64>],
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"quickpls-cbsem-canonical-ml-covariance-v2\0");
    digest.update(kind.as_str().as_bytes());
    digest.update((sample_size as u64).to_le_bytes());
    digest.update(match denominator {
        SemCovarianceDenominatorV4::SampleNMinusOne => b"sample_n_minus_one".as_slice(),
        SemCovarianceDenominatorV4::MaximumLikelihoodN => b"maximum_likelihood_n".as_slice(),
    });
    for variable in variable_ids {
        digest.update((variable.len() as u64).to_le_bytes());
        digest.update(variable.as_bytes());
    }
    for row in covariance_ml {
        for value in row {
            digest.update(value.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", digest.finalize())
}

fn covariance_sha256_with_control(
    kind: CbsemMomentInputKindV2,
    sample_size: usize,
    variable_ids: &[String],
    denominator: SemCovarianceDenominatorV4,
    covariance_ml: &[Vec<f64>],
    should_cancel: &impl Fn() -> bool,
) -> Result<String, CbsemCompiledMomentErrorV2> {
    if should_cancel() {
        return Err(CbsemCompiledMomentErrorV2::Cancelled);
    }
    let mut digest = Sha256::new();
    digest.update(b"quickpls-cbsem-canonical-ml-covariance-v2\0");
    digest.update(kind.as_str().as_bytes());
    digest.update((sample_size as u64).to_le_bytes());
    digest.update(match denominator {
        SemCovarianceDenominatorV4::SampleNMinusOne => b"sample_n_minus_one".as_slice(),
        SemCovarianceDenominatorV4::MaximumLikelihoodN => b"maximum_likelihood_n".as_slice(),
    });
    let mut work_units = 0usize;
    for variable in variable_ids {
        mean_replacement_hash_poll(should_cancel, &mut work_units)?;
        digest.update((variable.len() as u64).to_le_bytes());
        for chunk in variable
            .as_bytes()
            .chunks(MEAN_REPLACEMENT_CANCELLATION_POLL_INTERVAL_V1)
        {
            mean_replacement_hash_poll(should_cancel, &mut work_units)?;
            digest.update(chunk);
        }
    }
    for row in covariance_ml {
        for value in row {
            mean_replacement_hash_poll(should_cancel, &mut work_units)?;
            digest.update(value.to_bits().to_le_bytes());
        }
    }
    if should_cancel() {
        return Err(CbsemCompiledMomentErrorV2::Cancelled);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn mean_replacement_hash_poll(
    should_cancel: &impl Fn() -> bool,
    work_units: &mut usize,
) -> Result<(), CbsemCompiledMomentErrorV2> {
    if *work_units % MEAN_REPLACEMENT_CANCELLATION_POLL_INTERVAL_V1 == 0 && should_cancel() {
        return Err(CbsemCompiledMomentErrorV2::Cancelled);
    }
    *work_units += 1;
    Ok(())
}

pub(crate) fn observed_means_sha256(
    sample_size: usize,
    variable_ids: &[String],
    observed_means: &[f64],
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"quickpls-cbsem-canonical-observed-means-v1\0");
    digest.update((sample_size as u64).to_le_bytes());
    for variable in variable_ids {
        digest.update((variable.len() as u64).to_le_bytes());
        digest.update(variable.as_bytes());
    }
    for value in observed_means {
        digest.update(value.to_bits().to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn issue(
    issues: &mut Vec<CbsemMatrixModelIssueV2>,
    code: impl Into<String>,
    subject: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(CbsemMatrixModelIssueV2 {
        code: code.into(),
        subject: subject.into(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::{array::ArrayRef, record_batch::RecordBatch};
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_V4_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipeModelBindingV4,
        AnalysisSettings, CbsemEstimator, CbsemInput, CbsemModelType,
        LegacyBasicModelInterpretationV4, LegacyEstimandConfirmationV4, MethodConfig,
        MissingDataPolicy, MissingDataPolicyV4, RecipeV4CompilerTarget, SemGroupLevelV4,
        SemMatrixSampleMetadataV4, compile_analysis_recipe_v4, compile_cbsem_plan_v2,
        convert_legacy_basic_model_v4, validate_cbsem_ml_v1_estimator_capability_v2,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    fn legacy_model() -> ModelSpec {
        ModelSpec {
            id: uuid::Uuid::from_u128(0xCB5E_0002),
            name: "One-factor hand microcase".into(),
            constructs: vec![Construct {
                id: "f".into(),
                name: "Factor".into(),
                short_name: "F".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into(), "x3".into()],
            }],
            paths: Vec::new(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        }
    }

    fn raw_rows() -> Vec<Vec<f64>> {
        (0..40)
            .map(|index| {
                let t = index as f64 - 19.5;
                let a = ((index * 7) % 11) as f64 - 5.0;
                let b = ((index * 5) % 13) as f64 - 6.0;
                vec![
                    t + 0.30 * a,
                    0.80 * t + 0.50 * b,
                    0.50 * t - 0.40 * a + 0.20 * b,
                ]
            })
            .collect()
    }

    fn raw_dataset() -> Dataset {
        let mut csv = String::from("x1,x2,x3\n");
        for row in raw_rows() {
            csv.push_str(&format!("{:.17},{:.17},{:.17}\n", row[0], row[1], row[2]));
        }
        import_delimited_bytes(
            csv.as_bytes(),
            "raw-microcase.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap()
    }

    fn mean_replacement_dataset() -> Dataset {
        let rows = raw_rows();
        let means = (0..3)
            .map(|column| rows.iter().map(|row| row[column]).sum::<f64>() / rows.len() as f64)
            .collect::<Vec<_>>();
        let scales = (0..3)
            .map(|column| {
                (rows
                    .iter()
                    .map(|row| {
                        let deviation = row[column] - means[column];
                        deviation * deviation
                    })
                    .sum::<f64>()
                    / rows.len() as f64)
                    .sqrt()
            })
            .collect::<Vec<_>>();
        let mut csv = String::from("x1,x2,x3\n");
        for (index, row) in rows.into_iter().enumerate() {
            // Keep this integration fixture on the generic exact-estimator
            // start scale; the production mean-replacement kernel does not
            // standardize or otherwise transform source values.
            let standardized = [
                (row[0] - means[0]) / scales[0],
                (row[1] - means[1]) / scales[1],
                (row[2] - means[2]) / scales[2],
            ];
            let x1 = (index >= 2).then(|| format!("{:.17}", standardized[0]));
            let x2 = (index >= 7).then(|| format!("{:.17}", standardized[1]));
            let x3 = (index != 0).then(|| format!("{:.17}", standardized[2]));
            csv.push_str(&format!(
                "{},{},{}\n",
                x1.as_deref().unwrap_or("NA"),
                x2.as_deref().unwrap_or("NA"),
                x3.as_deref().unwrap_or("NA")
            ));
        }
        import_delimited_bytes(
            csv.as_bytes(),
            "raw-mean-replacement-microcase.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap()
    }

    fn shifted_raw_dataset() -> Dataset {
        let mut csv = String::from("x1,x2,x3\n");
        for row in raw_rows() {
            csv.push_str(&format!(
                "{:.17},{:.17},{:.17}\n",
                row[0] + 3.0,
                row[1] + 4.4,
                row[2] + 0.5
            ));
        }
        import_delimited_bytes(
            csv.as_bytes(),
            "raw-mean-microcase.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap()
    }

    fn sample_covariance(rows: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let ml = covariance_ml_from_rows(rows);
        let scale = rows.len() as f64 / (rows.len() - 1) as f64;
        ml.into_iter()
            .map(|row| row.into_iter().map(|value| value * scale).collect())
            .collect()
    }

    fn sample_correlation_and_sd(rows: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<f64>) {
        let covariance = sample_covariance(rows);
        let sd = (0..covariance.len())
            .map(|index| covariance[index][index].sqrt())
            .collect::<Vec<_>>();
        let correlation = (0..covariance.len())
            .map(|row| {
                (0..covariance.len())
                    .map(|column| {
                        if row == column {
                            1.0
                        } else {
                            (covariance[row][column] / (sd[row] * sd[column])).clamp(-1.0, 1.0)
                        }
                    })
                    .collect()
            })
            .collect();
        (correlation, sd)
    }

    fn matrix_dataset(
        kind: DataKind,
        sample_size: usize,
        canonical_matrix: &[Vec<f64>],
        order: &[usize],
    ) -> Dataset {
        let names = ["x1", "x2", "x3"];
        let mut csv = String::from(",");
        csv.push_str(
            &order
                .iter()
                .map(|index| names[*index])
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
        for source_row in order {
            csv.push_str(names[*source_row]);
            for source_column in order {
                csv.push_str(&format!(
                    ",{:.17}",
                    canonical_matrix[*source_row][*source_column]
                ));
            }
            csv.push('\n');
        }
        import_delimited_bytes(
            csv.as_bytes(),
            "matrix-microcase.csv",
            b',',
            &ImportOptions {
                data_kind: kind,
                sample_size: Some(sample_size),
                ..ImportOptions::default()
            },
        )
        .unwrap()
    }

    fn named_matrix_dataset(
        kind: DataKind,
        sample_size: usize,
        names: &[&str],
        canonical_matrix: &[Vec<f64>],
    ) -> Dataset {
        let mut csv = format!(",{}\n", names.join(","));
        for (row, name) in names.iter().enumerate() {
            csv.push_str(name);
            for value in &canonical_matrix[row] {
                csv.push_str(&format!(",{value:.17}"));
            }
            csv.push('\n');
        }
        import_delimited_bytes(
            csv.as_bytes(),
            "named-matrix-microcase.csv",
            b',',
            &ImportOptions {
                data_kind: kind,
                sample_size: Some(sample_size),
                ..ImportOptions::default()
            },
        )
        .unwrap()
    }

    fn named_covariance_binding(
        dataset: &Dataset,
        observed_ids: &[String],
        sample_size: usize,
    ) -> qpls_core::SemDataBindingV4 {
        qpls_core::SemDataBindingV4::Covariance {
            dataset_id: dataset.id.to_string(),
            variables: observed_ids.to_vec(),
            means: None,
            standard_deviations: None,
            sample: SemMatrixSampleMetadataV4 {
                sample_size,
                covariance_denominator: SemCovarianceDenominatorV4::MaximumLikelihoodN,
                effective_sample_size: None,
                degrees_of_freedom: None,
                group_sample_sizes: BTreeMap::new(),
            },
        }
    }

    fn sem_model(binding: qpls_core::SemDataBindingV4) -> qpls_core::SemModelV4 {
        let mut model = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.data_binding = binding;
        model.ensure_valid().unwrap();
        model
    }

    fn recipe(
        dataset: &Dataset,
        model: &qpls_core::SemModelV4,
        input: CbsemInput,
    ) -> AnalysisRecipeV4 {
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Cbsem;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.workers = 1;
        AnalysisRecipeV4 {
            schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: uuid::Uuid::from_u128(0xCB5E_2002),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                model: model.clone(),
                scientific_sha256: model.scientific_sha256().unwrap(),
            },
            estimand_confirmation: LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
            settings,
            method_config: Some(MethodConfig::Cbsem {
                model_type: CbsemModelType::Cfa,
                estimator: CbsemEstimator::Ml,
                input,
                mean_structure: false,
                bootstrap_samples: 0,
                bootstrap_v2: None,
                group_column: None,
                invariance_steps: Vec::new(),
            }),
            general_sem_config: None,
            metadata: BTreeMap::new(),
            legacy_source: None,
        }
    }

    fn compile(
        dataset: &Dataset,
        model: &qpls_core::SemModelV4,
        input: CbsemInput,
    ) -> (AnalysisRecipeV4, CompiledAnalysisRecipeV4) {
        let recipe = recipe(dataset, model, input);
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        (recipe, artifact)
    }

    fn compile_sem(
        dataset: &Dataset,
        model: &qpls_core::SemModelV4,
        input: CbsemInput,
    ) -> (AnalysisRecipeV4, CompiledAnalysisRecipeV4) {
        let mut recipe = recipe(dataset, model, input);
        let Some(MethodConfig::Cbsem { model_type, .. }) = recipe.method_config.as_mut() else {
            unreachable!()
        };
        *model_type = CbsemModelType::Sem;
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        (recipe, artifact)
    }

    fn raw_binding(dataset: &Dataset) -> qpls_core::SemDataBindingV4 {
        qpls_core::SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        }
    }

    fn mean_replacement_model(dataset: &Dataset) -> qpls_core::SemModelV4 {
        let mut model = sem_model(raw_binding(dataset));
        let qpls_core::SemDataBindingV4::Raw { missing_data, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *missing_data = MissingDataPolicyV4::MeanReplacement;
        for variable in &mut model.variables {
            let qpls_core::SemVariableV4::Observed {
                source_column,
                missing_markers,
                ..
            } = variable
            else {
                continue;
            };
            *missing_markers = dataset
                .schema
                .columns
                .iter()
                .find(|column| column.name == source_column.as_str())
                .unwrap()
                .missing_markers
                .iter()
                .map(|marker| marker.trim())
                .filter(|marker| !marker.is_empty())
                .map(str::to_owned)
                .collect();
            missing_markers.sort();
            missing_markers.dedup();
        }
        model.ensure_valid().unwrap();
        model
    }

    fn raw_mean_model(dataset: &Dataset, marker_intercept_fixed: bool) -> qpls_core::SemModelV4 {
        let mut model = sem_model(raw_binding(dataset));
        let factor_id = model
            .variables
            .iter_mut()
            .find_map(|variable| match variable {
                qpls_core::SemVariableV4::CommonFactor {
                    id, mean_policy, ..
                } => {
                    *mean_policy = qpls_core::FactorMeanPolicyV4::Estimated {
                        parameter: "parameter:factor_mean:f".into(),
                    };
                    Some(id.clone())
                }
                _ => None,
            })
            .unwrap();
        let marker_target = SemParameterTargetV4::Intercept {
            variable: "observed:x1".into(),
        };
        model.parameters.push(if marker_intercept_fixed {
            qpls_core::SemParameterV4::Fixed {
                id: "parameter:intercept:x1".into(),
                label: "x1 intercept anchor".into(),
                target: marker_target,
                value: 0.0,
                group_overrides: Vec::new(),
            }
        } else {
            qpls_core::SemParameterV4::Free {
                id: "parameter:intercept:x1".into(),
                label: "x1 intercept".into(),
                target: marker_target,
                start: Some(0.0),
                lower: Some(-20.0),
                upper: Some(20.0),
                equality_label: None,
                group_overrides: Vec::new(),
            }
        });
        for (source, start) in [("x2", 2.0), ("x3", -1.0)] {
            model.parameters.push(qpls_core::SemParameterV4::Free {
                id: format!("parameter:intercept:{source}"),
                label: format!("{source} intercept"),
                target: SemParameterTargetV4::Intercept {
                    variable: format!("observed:{source}"),
                },
                start: Some(start),
                lower: Some(-20.0),
                upper: Some(20.0),
                equality_label: None,
                group_overrides: Vec::new(),
            });
        }
        model.parameters.push(qpls_core::SemParameterV4::Free {
            id: "parameter:factor_mean:f".into(),
            label: "Factor mean".into(),
            target: SemParameterTargetV4::Mean {
                variable: factor_id,
            },
            start: Some(3.0),
            lower: Some(0.0),
            upper: Some(10.0),
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.ensure_valid().unwrap();
        model
    }

    fn mean_recipe(
        dataset: &Dataset,
        model: &qpls_core::SemModelV4,
        input: CbsemInput,
    ) -> AnalysisRecipeV4 {
        let mut recipe = recipe(dataset, model, input);
        let Some(MethodConfig::Cbsem { mean_structure, .. }) = recipe.method_config.as_mut() else {
            unreachable!()
        };
        *mean_structure = true;
        recipe
    }

    fn covariance_binding(
        dataset: &Dataset,
        variable_order: &[usize],
    ) -> qpls_core::SemDataBindingV4 {
        let ids = ["observed:x1", "observed:x2", "observed:x3"];
        qpls_core::SemDataBindingV4::Covariance {
            dataset_id: dataset.id.to_string(),
            variables: variable_order
                .iter()
                .map(|index| ids[*index].into())
                .collect(),
            means: None,
            standard_deviations: None,
            sample: SemMatrixSampleMetadataV4 {
                sample_size: 40,
                covariance_denominator: SemCovarianceDenominatorV4::SampleNMinusOne,
                effective_sample_size: None,
                degrees_of_freedom: None,
                group_sample_sizes: BTreeMap::new(),
            },
        }
    }

    fn correlation_binding(
        dataset: &Dataset,
        variable_order: &[usize],
        standard_deviations: Option<&[f64]>,
    ) -> qpls_core::SemDataBindingV4 {
        let ids = ["observed:x1", "observed:x2", "observed:x3"];
        qpls_core::SemDataBindingV4::Correlation {
            dataset_id: dataset.id.to_string(),
            variables: variable_order
                .iter()
                .map(|index| ids[*index].into())
                .collect(),
            means: None,
            standard_deviations: standard_deviations.map(|values| {
                ids.into_iter()
                    .enumerate()
                    .map(|(index, id)| (id.into(), values[index]))
                    .collect()
            }),
            sample: SemMatrixSampleMetadataV4 {
                sample_size: 40,
                covariance_denominator: SemCovarianceDenominatorV4::SampleNMinusOne,
                effective_sample_size: None,
                degrees_of_freedom: None,
                group_sample_sizes: BTreeMap::new(),
            },
        }
    }

    fn parameter_names(result: &CbsemCompiledMomentResultV2) -> Vec<String> {
        result
            .analysis
            .parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect()
    }

    fn standardized_estimates(
        result: &CbsemCompiledMomentResultV2,
    ) -> BTreeMap<String, (f64, f64)> {
        result
            .analysis
            .standardized
            .iter()
            .map(|parameter| {
                (
                    parameter.name.clone(),
                    (parameter.std_lv, parameter.std_all),
                )
            })
            .collect()
    }

    fn implied_covariance(result: &CbsemCompiledMomentResultV2) -> BTreeMap<(String, String), f64> {
        result
            .analysis
            .implied_covariance
            .iter()
            .map(|cell| ((cell.row.clone(), cell.column.clone()), cell.value))
            .collect()
    }

    fn assert_results_close(
        expected: &CbsemCompiledMomentResultV2,
        actual: &CbsemCompiledMomentResultV2,
    ) {
        assert_eq!(
            expected.input.used_sample_size,
            actual.input.used_sample_size
        );
        for (left_row, right_row) in expected.covariance_ml.iter().zip(&actual.covariance_ml) {
            for (left, right) in left_row.iter().zip(right_row) {
                assert!((left - right).abs() <= 1e-12, "{left} != {right}");
            }
        }
        assert!(expected.analysis.converged);
        assert!(actual.analysis.converged);
        assert!(
            expected.analysis.gradient_norm
                <= CBSEM_ML_OPTIMIZER_STAGNATION_GRADIENT_NORM_TOLERANCE,
            "expected gradient norm {} exceeds the optimizer's accepted convergence bound",
            expected.analysis.gradient_norm
        );
        assert!(
            actual.analysis.gradient_norm <= CBSEM_ML_OPTIMIZER_STAGNATION_GRADIENT_NORM_TOLERANCE,
            "actual gradient norm {} exceeds the optimizer's accepted convergence bound",
            actual.analysis.gradient_norm
        );
        assert!(
            cbsem_raw_matrix_objectives_equivalent_v2(
                expected.analysis.objective,
                actual.analysis.objective
            ),
            "objective: {} != {}",
            expected.analysis.objective,
            actual.analysis.objective
        );
        assert_eq!(parameter_names(expected), parameter_names(actual));
        assert_eq!(expected.parameter_ids, actual.parameter_ids);

        let expected_estimates = standardized_estimates(expected);
        let actual_estimates = standardized_estimates(actual);
        assert_eq!(
            expected_estimates.keys().collect::<Vec<_>>(),
            actual_estimates.keys().collect::<Vec<_>>()
        );
        for (name, (expected_std_lv, expected_std_all)) in expected_estimates {
            let (actual_std_lv, actual_std_all) = actual_estimates[&name];
            assert!(
                cbsem_raw_matrix_standardized_values_equivalent_v2(expected_std_lv, actual_std_lv),
                "{name} std.lv: {expected_std_lv} != {actual_std_lv}"
            );
            assert!(
                cbsem_raw_matrix_standardized_values_equivalent_v2(
                    expected_std_all,
                    actual_std_all
                ),
                "{name} std.all: {expected_std_all} != {actual_std_all}"
            );
        }

        let expected_implied = implied_covariance(expected);
        let actual_implied = implied_covariance(actual);
        assert_eq!(
            expected_implied.keys().collect::<Vec<_>>(),
            actual_implied.keys().collect::<Vec<_>>()
        );
        for (cell, expected_value) in expected_implied {
            let actual_value = actual_implied[&cell];
            assert!(
                cbsem_raw_matrix_standardized_values_equivalent_v2(expected_value, actual_value),
                "implied covariance {}~{}: {expected_value} != {actual_value}",
                cell.0,
                cell.1
            );
        }
    }

    #[test]
    fn equivalence_envelopes_are_optimizer_bound_and_reject_boundary_mutations() {
        assert_eq!(
            CBSEM_RAW_MATRIX_STANDARDIZED_ABS_TOLERANCE_V2,
            CBSEM_ML_OPTIMIZER_STAGNATION_GRADIENT_NORM_TOLERANCE
        );
        assert_eq!(
            CBSEM_RAW_MATRIX_STANDARDIZED_REL_TOLERANCE_V2,
            CBSEM_ML_OPTIMIZER_FINITE_DIFFERENCE_RELATIVE_STEP
        );
        assert_eq!(
            CBSEM_RAW_MATRIX_OBJECTIVE_ABS_TOLERANCE_V2,
            CBSEM_ML_OPTIMIZER_OBJECTIVE_STAGNATION_TOLERANCE
        );

        let standardized = 2.0_f64;
        let standardized_boundary = CBSEM_RAW_MATRIX_STANDARDIZED_ABS_TOLERANCE_V2
            + CBSEM_RAW_MATRIX_STANDARDIZED_REL_TOLERANCE_V2 * standardized.abs();
        assert!(cbsem_raw_matrix_standardized_values_equivalent_v2(
            standardized,
            standardized + standardized_boundary * 0.5
        ));
        assert!(!cbsem_raw_matrix_standardized_values_equivalent_v2(
            standardized,
            standardized + standardized_boundary * 4.0
        ));

        let objective = 0.25_f64;
        let objective_boundary = CBSEM_RAW_MATRIX_OBJECTIVE_ABS_TOLERANCE_V2
            + CBSEM_RAW_MATRIX_OBJECTIVE_REL_TOLERANCE_V2 * objective.abs();
        assert!(cbsem_raw_matrix_objectives_equivalent_v2(
            objective,
            objective + objective_boundary * 0.5
        ));
        assert!(!cbsem_raw_matrix_objectives_equivalent_v2(
            objective,
            objective + objective_boundary * 4.0
        ));
        assert!(!cbsem_raw_matrix_standardized_values_equivalent_v2(
            f64::NAN,
            standardized
        ));
        assert!(!cbsem_raw_matrix_objectives_equivalent_v2(
            objective,
            f64::INFINITY
        ));
    }

    #[test]
    fn controlled_execution_cancels_inside_optimization_without_a_result() {
        let raw = raw_dataset();
        let model = sem_model(raw_binding(&raw));
        let (recipe, artifact) = compile(&raw, &model, CbsemInput::Raw);
        let checks = AtomicUsize::new(0);
        let progress = Mutex::new(Vec::new());

        let error = estimate_cbsem_ml_compiled_moments_v2_with_control(
            &raw,
            &artifact,
            &recipe,
            &model,
            || checks.fetch_add(1, Ordering::SeqCst) >= 7,
            |update| progress.lock().unwrap().push(update),
        )
        .unwrap_err();

        assert!(matches!(error, CbsemCompiledMomentErrorV2::Cancelled));
        assert!(
            progress
                .lock()
                .unwrap()
                .iter()
                .any(|update| update.phase == "estimation")
        );
    }

    #[test]
    fn prepared_exact_bootstrap_source_exposes_equality_aware_optimizer_dimension_count() {
        let raw = raw_dataset();
        let mut model = sem_model(raw_binding(&raw));
        let mut constrained_loading_rows = 0;
        for parameter in &mut model.parameters {
            let qpls_core::SemParameterV4::Free {
                target: SemParameterTargetV4::Loading { .. },
                start,
                lower,
                upper,
                equality_label,
                ..
            } = parameter
            else {
                continue;
            };
            *start = None;
            *lower = None;
            *upper = None;
            *equality_label = Some("equal_nonmarker_loadings".into());
            constrained_loading_rows += 1;
        }
        assert_eq!(constrained_loading_rows, 2);
        model.ensure_valid().unwrap();
        let (recipe, artifact) = compile(&raw, &model, CbsemInput::Raw);

        let source =
            prepare_cbsem_ml_exact_case_bootstrap_source_v1(&raw, &artifact, &recipe, &model)
                .unwrap();
        let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
            unreachable!()
        };
        let free_parameter_rows = build_exact_projection_v3(plan, &recipe)
            .unwrap()
            .parameter_rows
            .into_iter()
            .filter(|row| {
                matches!(
                    row.specification,
                    CbsemExactParameterSpecificationV3::Free { .. }
                )
            })
            .count();

        assert_eq!(free_parameter_rows, 6);
        assert_eq!(source.optimizer_dimension_count(), 5);
        assert_eq!(source.optimizer_dimension_count() + 1, free_parameter_rows);
    }

    #[test]
    fn exact_case_bootstrap_refit_preserves_duplicate_draws_and_rejects_boundaries() {
        let raw = raw_dataset();
        let model = sem_model(raw_binding(&raw));
        let (recipe, artifact) = compile(&raw, &model, CbsemInput::Raw);
        let source =
            prepare_cbsem_ml_exact_case_bootstrap_source_v1(&raw, &artifact, &recipe, &model)
                .unwrap();
        let mut wrong_dataset_id = raw.clone();
        wrong_dataset_id.id = uuid::Uuid::from_u128(0xCB5E_B007);
        assert!(matches!(
            prepare_cbsem_ml_exact_case_bootstrap_source_v1(
                &wrong_dataset_id,
                &artifact,
                &recipe,
                &model,
            ),
            Err(CbsemCompiledMomentErrorV2::DatasetIdMismatch { .. })
        ));
        let duplicated = (0..20)
            .flat_map(|sampling_position| [sampling_position, sampling_position])
            .collect::<Vec<_>>();
        let prefit_receipt = source.sampling_receipt(&duplicated).unwrap();

        let refit = estimate_cbsem_ml_exact_case_resample_v1(&source, &duplicated).unwrap();

        assert_eq!(
            refit.method_version,
            CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1
        );
        assert_eq!(refit.source_dataset_id, raw.id.to_string());
        let recorded_fingerprint = raw
            .fingerprint
            .0
            .strip_prefix("v2:")
            .unwrap_or(&raw.fingerprint.0);
        assert_eq!(refit.source_dataset_fingerprint, recorded_fingerprint);
        assert_eq!(source.source_dataset_fingerprint(), recorded_fingerprint);
        assert_ne!(refit.source_dataset_fingerprint, raw.fingerprint.0);
        assert_eq!(
            source.complete_case_universe_sha256(),
            cbsem_exact_case_bootstrap_complete_case_universe_digest_v1(
                recorded_fingerprint,
                raw.batch.num_rows(),
                &(0..raw.batch.num_rows()).collect::<Vec<_>>(),
            )
        );
        assert_eq!(refit.source_row_count, raw.batch.num_rows());
        assert_eq!(refit.complete_case_sample_size, raw.batch.num_rows());
        assert_eq!(
            refit.complete_case_universe_sha256,
            source.complete_case_universe_sha256()
        );
        assert_eq!(refit.resampled_observations, duplicated.len());
        assert_eq!(
            refit.covariance_denominator,
            SemCovarianceDenominatorV4::MaximumLikelihoodN
        );
        assert!(refit.objective.is_finite());
        assert!(refit.gradient_norm.is_finite());
        assert_eq!(
            refit.estimator_method_version,
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        );
        assert!(!refit.free_parameters.is_empty());
        let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
            unreachable!()
        };
        let mut expected_free_parameter_ids = build_exact_projection_v3(plan, &recipe)
            .unwrap()
            .parameter_rows
            .into_iter()
            .filter(|row| {
                matches!(
                    &row.specification,
                    CbsemExactParameterSpecificationV3::Free { .. }
                )
            })
            .map(|row| row.stable_id)
            .collect::<Vec<_>>();
        expected_free_parameter_ids.sort();
        assert_eq!(
            source.optimizer_dimension_count(),
            expected_free_parameter_ids.len(),
            "unlabeled free rows must each retain an optimizer dimension"
        );
        assert_eq!(
            refit
                .free_parameters
                .iter()
                .map(|parameter| parameter.parameter_id.clone())
                .collect::<Vec<_>>(),
            expected_free_parameter_ids
        );
        assert!(
            refit
                .free_parameters
                .iter()
                .all(|parameter| parameter.estimate.is_finite())
        );
        assert_eq!(
            refit.sampling_positions_sha256,
            cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
                source.complete_case_sample_size(),
                &duplicated,
            )
        );
        assert_eq!(
            refit.sampling_positions_sha256,
            prefit_receipt.sampling_positions_sha256
        );
        assert_eq!(
            refit.sample_indices_sha256,
            cbsem_exact_case_bootstrap_index_digest_v1(
                recorded_fingerprint,
                raw.batch.num_rows(),
                &duplicated,
            )
        );
        assert_eq!(
            refit.sample_indices_sha256,
            prefit_receipt.sample_indices_sha256
        );
        assert_ne!(
            refit.sampling_positions_sha256, refit.sample_indices_sha256,
            "sampling-position and mapped-source-row receipts use separate digest domains"
        );
        let mut reversed = duplicated.clone();
        reversed.reverse();
        assert_ne!(
            refit.sampling_positions_sha256,
            cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
                source.complete_case_sample_size(),
                &reversed,
            )
        );
        assert_ne!(
            refit.sample_indices_sha256,
            cbsem_exact_case_bootstrap_index_digest_v1(
                recorded_fingerprint,
                raw.batch.num_rows(),
                &reversed,
            )
        );

        let mut out_of_range = duplicated.clone();
        out_of_range[7] = source.complete_case_sample_size();
        assert!(matches!(
            estimate_cbsem_ml_exact_case_resample_v1(&source, &out_of_range),
            Err(CbsemCompiledMomentErrorV2::ExactCaseBootstrapSamplingPositionOutOfRange {
                draw_position: 7,
                sampling_position,
                complete_case_sample_size,
            }) if sampling_position == source.complete_case_sample_size()
                && complete_case_sample_size == source.complete_case_sample_size()
        ));
        assert!(matches!(
            estimate_cbsem_ml_exact_case_resample_v1(&source, &duplicated[..10]),
            Err(CbsemCompiledMomentErrorV2::ExactCaseBootstrapDrawSizeMismatch {
                expected,
                actual: 10,
            }) if expected == source.complete_case_sample_size()
        ));
        let degenerate = vec![0; source.complete_case_sample_size()];
        assert!(source.sampling_receipt(&degenerate).is_ok());
        assert!(matches!(
            estimate_cbsem_ml_exact_case_resample_v1(&source, &degenerate),
            Err(CbsemCompiledMomentErrorV2::MatrixNotPositiveDefinite { .. })
        ));
        assert!(matches!(
            estimate_cbsem_ml_exact_case_resample_v1_with_control(
                &source,
                &duplicated,
                || true,
                |_| {},
            ),
            Err(CbsemCompiledMomentErrorV2::Cancelled)
        ));
    }

    #[test]
    fn opt_in_exact_refit_standard_errors_match_identity_point_and_preserve_legacy_bytes() {
        let raw = raw_dataset();
        let model = sem_model(raw_binding(&raw));
        let (recipe, artifact) = compile(&raw, &model, CbsemInput::Raw);
        let source =
            prepare_cbsem_ml_exact_case_bootstrap_source_v1(&raw, &artifact, &recipe, &model)
                .unwrap();
        let identity = (0..source.complete_case_sample_size()).collect::<Vec<_>>();
        let point =
            estimate_cbsem_ml_compiled_moments_v2(&raw, &artifact, &recipe, &model).unwrap();

        let first = estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1(
            &source, &identity,
        )
        .unwrap();
        let bytes_before_dimension_read = serde_json::to_vec(&first).unwrap();
        assert_eq!(
            source.optimizer_dimension_count(),
            first.refit.free_parameters.len()
        );
        let second = estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1(
            &source, &identity,
        )
        .unwrap();
        assert_eq!(first, second, "the opt-in refit must be deterministic");
        assert_eq!(
            serde_json::to_vec(&second).unwrap(),
            bytes_before_dimension_read,
            "reading D must not alter refit or studentized scientific bytes"
        );
        assert_eq!(
            first.standard_errors.method_version,
            crate::CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1
        );
        let crate::CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Available {
            information_method,
            parameters,
        } = &first.standard_errors.outcome
        else {
            panic!("identity refit expected-information SEs must be available")
        };
        assert_eq!(
            information_method,
            crate::CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1
        );
        let refit_ids = first
            .refit
            .free_parameters
            .iter()
            .map(|parameter| parameter.parameter_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            parameters
                .iter()
                .map(|parameter| parameter.parameter_id.as_str())
                .collect::<Vec<_>>(),
            refit_ids
        );
        assert!(refit_ids.windows(2).all(|pair| pair[0] < pair[1]));

        let expected = point
            .analysis
            .parameters
            .iter()
            .filter(|parameter| !parameter.fixed)
            .map(|parameter| {
                (
                    point.parameter_ids[&parameter.name].as_str(),
                    (
                        parameter.estimate,
                        parameter
                            .standard_error
                            .expect("canonical free parameter has an analytical SE"),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for (estimate, standard_error) in first.refit.free_parameters.iter().zip(parameters) {
            let (expected_estimate, expected_standard_error) =
                expected[estimate.parameter_id.as_str()];
            assert_eq!(estimate.estimate.to_bits(), expected_estimate.to_bits());
            assert_eq!(
                standard_error.standard_error.to_bits(),
                expected_standard_error.to_bits()
            );
            assert!(standard_error.standard_error.is_finite());
            assert!(standard_error.standard_error > 0.0);
        }

        let legacy = estimate_cbsem_ml_exact_case_resample_v1(&source, &identity).unwrap();
        assert_eq!(legacy, first.refit);
        let legacy_json = serde_json::to_string(&legacy).unwrap();
        assert!(!legacy_json.contains("standard_errors"));
        assert_eq!(
            serde_json::from_str::<CbsemExactCaseBootstrapRefitV1>(&legacy_json).unwrap(),
            legacy
        );

        let duplicated = (0..source.complete_case_sample_size() / 2)
            .flat_map(|position| [position, position])
            .collect::<Vec<_>>();
        let duplicated_refit =
            estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1(
                &source,
                &duplicated,
            )
            .unwrap();
        assert_eq!(
            duplicated_refit.refit.resampled_observations,
            source.complete_case_sample_size()
        );
        assert!(matches!(
            estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control(
                &source,
                &identity,
                || true,
                |_| {},
            ),
            Err(CbsemCompiledMomentErrorV2::Cancelled)
        ));
    }

    #[test]
    fn exact_case_bootstrap_source_rejects_mean_structure_before_any_refit() {
        let dataset = shifted_raw_dataset();
        let model = raw_mean_model(&dataset, true);
        let recipe = mean_recipe(&dataset, &model, CbsemInput::Raw);
        let target = RecipeV4CompilerTarget::CbsemPlanV2;
        let artifact =
            compile_analysis_recipe_v4(&recipe, Some(&model), target, target.capability_cell())
                .unwrap();

        assert!(matches!(
            prepare_cbsem_ml_exact_case_bootstrap_source_v1(&dataset, &artifact, &recipe, &model,),
            Err(CbsemCompiledMomentErrorV2::ExactCaseBootstrapUnsupported)
        ));
    }

    #[test]
    fn continuous_raw_mean_replacement_executes_cellwise_with_a_typed_receipt() {
        let dataset = mean_replacement_dataset();
        let source_arrow = write_arrow(&dataset.batch).unwrap();
        let source_fingerprint = dataset.fingerprint.clone();
        let model = mean_replacement_model(&dataset);
        let mut recipe = recipe(&dataset, &model, CbsemInput::Raw);
        recipe.settings.missing_data = MissingDataPolicy::MeanReplacement;
        let target = RecipeV4CompilerTarget::CbsemPlanV2;
        let artifact =
            compile_analysis_recipe_v4(&recipe, Some(&model), target, target.capability_cell())
                .unwrap();

        let result =
            estimate_cbsem_ml_compiled_moments_v2(&dataset, &artifact, &recipe, &model).unwrap();
        assert!(result.analysis.converged);
        assert!(result.analysis.score_lm.is_none());
        assert_eq!(
            result.method_version,
            CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1
        );
        assert_eq!(
            result.schema_version,
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4
        );
        assert_eq!(result.input.used_sample_size, 40);
        assert_eq!(result.input.omitted_observations, 0);
        let receipt = result.input.missing_data_treatment.as_ref().unwrap();
        assert_eq!(result.input.dataset_id, receipt.source_dataset_id);
        assert_eq!(
            result.input.dataset_fingerprint,
            receipt.source_dataset_fingerprint
        );
        assert_eq!(
            receipt.method_version,
            crate::MEAN_REPLACEMENT_METHOD_VERSION_V1
        );
        assert_eq!(receipt.source_dataset_id, dataset.id.to_string());
        assert_eq!(receipt.source_dataset_fingerprint, dataset.fingerprint.0);
        assert_eq!(receipt.source_row_count, 40);
        assert_eq!(receipt.retained_row_count, 40);
        assert_eq!(receipt.omitted_row_count, 0);
        assert_eq!(receipt.imputed_cell_count, 10);
        assert_eq!(receipt.affected_case_count, 7);
        assert_eq!(receipt.missingness_sha256.len(), 64);
        assert_eq!(receipt.completed_matrix_sha256.len(), 64);
        assert_eq!(receipt.receipt_sha256.len(), 64);
        assert_eq!(
            result.input.canonical_ml_covariance_sha256,
            covariance_sha256(
                CbsemMomentInputKindV2::Raw,
                result.input.used_sample_size,
                &result.input.variable_ids,
                SemCovarianceDenominatorV4::MaximumLikelihoodN,
                &result.covariance_ml,
            )
        );
        assert_eq!(receipt.cases[0].row_index_zero_based, 0);
        assert_eq!(
            receipt.cases[0].imputed_variable_ids,
            vec!["observed:x1", "observed:x2", "observed:x3"]
        );
        assert_eq!(receipt.cases[0].missing_fraction, 1.0);
        assert!(receipt.cases[0].high_missingness_warning);
        assert_eq!(
            receipt.variables[0].warning_level,
            crate::MeanReplacementWarningLevelV1::AtLeastFivePercent
        );
        assert_eq!(
            receipt.variables[1].warning_level,
            crate::MeanReplacementWarningLevelV1::AboveFifteenPercent
        );
        assert_eq!(
            receipt.variables[2].warning_level,
            crate::MeanReplacementWarningLevelV1::None
        );
        assert_eq!(write_arrow(&dataset.batch).unwrap(), source_arrow);
        assert_eq!(dataset.fingerprint, source_fingerprint);
    }

    #[test]
    fn mean_replacement_preparation_cancellation_and_outer_integrity_remain_typed() {
        let dataset = mean_replacement_dataset();
        let model = mean_replacement_model(&dataset);
        let mut recipe = recipe(&dataset, &model, CbsemInput::Raw);
        recipe.settings.missing_data = MissingDataPolicy::MeanReplacement;
        let target = RecipeV4CompilerTarget::CbsemPlanV2;
        let artifact =
            compile_analysis_recipe_v4(&recipe, Some(&model), target, target.capability_cell())
                .unwrap();

        let checks = AtomicUsize::new(0);
        assert!(matches!(
            estimate_cbsem_ml_compiled_moments_v2_with_control(
                &dataset,
                &artifact,
                &recipe,
                &model,
                || checks.fetch_add(1, Ordering::SeqCst) >= 8,
                |_| {}
            ),
            Err(CbsemCompiledMomentErrorV2::Cancelled)
        ));
        assert!(checks.load(Ordering::SeqCst) >= 9);

        let mut changed_bytes = dataset.clone();
        let mut columns = changed_bytes
            .batch
            .columns()
            .iter()
            .map(|column| {
                let values = column.as_any().downcast_ref::<Float64Array>().unwrap();
                (0..values.len())
                    .map(|row| (!values.is_null(row)).then(|| values.value(row)))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        columns[0][2] = columns[0][2].map(|value| value + 0.25);
        changed_bytes.batch = RecordBatch::try_from_iter(
            ["x1", "x2", "x3"]
                .into_iter()
                .zip(columns)
                .map(|(name, values)| (name, Arc::new(Float64Array::from(values)) as ArrayRef)),
        )
        .unwrap();
        assert!(matches!(
            estimate_cbsem_ml_compiled_moments_v2(&changed_bytes, &artifact, &recipe, &model),
            Err(CbsemCompiledMomentErrorV2::DatasetIntegrity(_))
        ));
    }

    #[test]
    fn hand_microcase_raw_covariance_and_scaled_correlation_are_equivalent() {
        let raw = raw_dataset();
        let raw_model = sem_model(raw_binding(&raw));
        let (raw_recipe, raw_artifact) = compile(&raw, &raw_model, CbsemInput::Raw);
        let raw_result =
            estimate_cbsem_ml_compiled_moments_v2(&raw, &raw_artifact, &raw_recipe, &raw_model)
                .unwrap();
        assert_eq!(
            raw_result.method_version,
            CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3
        );
        assert_eq!(
            raw_result.analysis.method_version,
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        );
        assert_eq!(
            raw_result.schema_version,
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2
        );
        assert!(!raw_result.analysis.mean_structure);
        assert!(raw_result.analysis.score_lm.is_some());
        assert!(
            serde_json::to_value(&raw_result.analysis)
                .unwrap()
                .get("exact_case_bootstrap")
                .is_none(),
            "legacy point-result bytes must omit the new exact-bootstrap field"
        );
        assert!(raw_result.observed_means.is_empty());
        assert!(raw_result.implied_means.is_empty());
        assert!(raw_result.residual_means.is_empty());
        assert!(raw_result.input.canonical_observed_means_sha256.is_none());
        assert!(
            serde_json::to_value(&raw_result.input)
                .unwrap()
                .get("missing_data_treatment")
                .is_none(),
            "listwise provenance bytes must omit the new optional receipt field"
        );

        let rows = raw_rows();
        let covariance = sample_covariance(&rows);
        let covariance_data = matrix_dataset(DataKind::Covariance, 40, &covariance, &[0, 1, 2]);
        let covariance_model = sem_model(covariance_binding(&covariance_data, &[0, 1, 2]));
        let (covariance_recipe, covariance_artifact) =
            compile(&covariance_data, &covariance_model, CbsemInput::Covariance);
        let covariance_result = estimate_cbsem_ml_compiled_moments_v2(
            &covariance_data,
            &covariance_artifact,
            &covariance_recipe,
            &covariance_model,
        )
        .unwrap();
        assert_results_close(&raw_result, &covariance_result);

        let (correlation, sd) = sample_correlation_and_sd(&rows);
        let correlation_data = matrix_dataset(DataKind::Correlation, 40, &correlation, &[0, 1, 2]);
        let correlation_model = sem_model(correlation_binding(
            &correlation_data,
            &[0, 1, 2],
            Some(&sd),
        ));
        let (correlation_recipe, correlation_artifact) = compile(
            &correlation_data,
            &correlation_model,
            CbsemInput::Correlation,
        );
        let correlation_result = estimate_cbsem_ml_compiled_moments_v2(
            &correlation_data,
            &correlation_artifact,
            &correlation_recipe,
            &correlation_model,
        )
        .unwrap();
        assert_results_close(&raw_result, &correlation_result);
        assert_eq!(
            correlation_result.input.covariance_denominator,
            SemCovarianceDenominatorV4::SampleNMinusOne
        );
        assert_eq!(correlation_result.analysis.input, "correlation");
        assert_eq!(
            correlation_result.parameter_ids,
            BTreeMap::from([
                ("construct:f=~x1".into(), "parameter_66_7831".into(),),
                ("construct:f=~x2".into(), "parameter_66_7832".into(),),
                ("construct:f=~x3".into(), "parameter_66_7833".into(),),
                ("construct:f~~construct:f".into(), "variance_66".into()),
                ("x1~~x1".into(), "residual_variance_7831".into()),
                ("x2~~x2".into(), "residual_variance_7832".into()),
                ("x3~~x3".into(), "residual_variance_7833".into()),
            ])
        );
        assert_eq!(
            correlation_result
                .analysis
                .parameters
                .iter()
                .map(|parameter| {
                    (
                        parameter.name.clone(),
                        (parameter.kind.clone(), parameter.fixed),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                ("construct:f=~x1".into(), ("loading".into(), true)),
                ("construct:f=~x2".into(), ("loading".into(), false)),
                ("construct:f=~x3".into(), ("loading".into(), false)),
                (
                    "construct:f~~construct:f".into(),
                    ("latent_variance".into(), false),
                ),
                ("x1~~x1".into(), ("residual_variance".into(), false),),
                ("x2~~x2".into(), ("residual_variance".into(), false),),
                ("x3~~x3".into(), ("residual_variance".into(), false),),
            ])
        );
    }

    #[test]
    fn raw_cfa_marker_identified_mean_structure_executes_joint_ml_with_stable_ids() {
        let dataset = shifted_raw_dataset();
        let model = raw_mean_model(&dataset, true);
        let recipe = mean_recipe(&dataset, &model, CbsemInput::Raw);
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        let result =
            estimate_cbsem_ml_compiled_moments_v2(&dataset, &artifact, &recipe, &model).unwrap();

        assert!(result.analysis.converged);
        assert!(result.analysis.mean_structure);
        assert!(result.analysis.score_lm.is_none());
        assert_eq!(result.analysis.input, "raw");
        assert_eq!(
            result.method_version,
            CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4
        );
        assert_eq!(
            result.analysis.method_version,
            crate::CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4
        );
        assert_eq!(
            result.schema_version,
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3
        );
        assert_eq!(result.observed_means.len(), 3);
        assert_eq!(result.implied_means.len(), 3);
        assert_eq!(result.residual_means.len(), 3);
        let covariance_row_sum = result
            .covariance_ml
            .iter()
            .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
            .fold(0.0_f64, f64::max);
        let reproduction_bound =
            covariance_row_sum * CBSEM_ML_OPTIMIZER_STAGNATION_GRADIENT_NORM_TOLERANCE;
        assert!(
            result
                .residual_means
                .iter()
                .all(|cell| cell.value.abs() <= reproduction_bound),
            "identified saturated mean structure must reproduce observed means within the estimator's declared gradient stopping envelope ({reproduction_bound})"
        );
        assert_eq!(
            result
                .input
                .canonical_observed_means_sha256
                .as_deref()
                .map(str::len),
            Some(64)
        );
        assert_eq!(
            result.parameter_ids.get("x1~1").map(String::as_str),
            Some("parameter:intercept:x1")
        );
        assert_eq!(
            result.parameter_ids.get("x2~1").map(String::as_str),
            Some("parameter:intercept:x2")
        );
        assert_eq!(
            result.parameter_ids.get("x3~1").map(String::as_str),
            Some("parameter:intercept:x3")
        );
        assert_eq!(
            result
                .parameter_ids
                .get("construct:f~1")
                .map(String::as_str),
            Some("parameter:factor_mean:f")
        );
        let latent_mean = result
            .analysis
            .parameters
            .iter()
            .find(|parameter| parameter.name == "construct:f~1")
            .unwrap();
        assert_eq!(latent_mean.kind, "latent_mean");
        assert!(!latent_mean.fixed);
        assert!(latent_mean.estimate > 0.0 && latent_mean.estimate < 10.0);
        let marker_intercept = result
            .analysis
            .parameters
            .iter()
            .find(|parameter| parameter.name == "x1~1")
            .unwrap();
        assert_eq!(marker_intercept.estimate.to_bits(), 0.0_f64.to_bits());
        assert!(marker_intercept.fixed);
    }

    #[test]
    fn raw_cfa_mean_structure_rejects_free_marker_anchor_and_matrix_input_with_typed_codes() {
        let raw = shifted_raw_dataset();
        let free_marker = raw_mean_model(&raw, false);
        let raw_recipe = mean_recipe(&raw, &free_marker, CbsemInput::Raw);
        let error = compile_analysis_recipe_v4(
            &raw_recipe,
            Some(&free_marker),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap_err();
        let RecipeV4CompilationError::CbsemEstimatorCapability(error) = error else {
            panic!("expected typed CB-SEM capability failure")
        };
        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.code == "latent_mean_marker_intercept_must_be_fixed")
        );

        let covariance = sample_covariance(&raw_rows());
        let matrix = matrix_dataset(DataKind::Covariance, 40, &covariance, &[0, 1, 2]);
        let matrix_model = sem_model(covariance_binding(&matrix, &[0, 1, 2]));
        let matrix_recipe = mean_recipe(&matrix, &matrix_model, CbsemInput::Covariance);
        let error = compile_analysis_recipe_v4(
            &matrix_recipe,
            Some(&matrix_model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap_err();
        let RecipeV4CompilationError::CbsemEstimatorCapability(error) = error else {
            panic!("expected typed CB-SEM matrix-mean capability failure")
        };
        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.code == "mean_structure_raw_input_required")
        );

        let special_model = raw_mean_model(&raw, true);
        let mut special_recipe = mean_recipe(&raw, &special_model, CbsemInput::Raw);
        special_recipe.metadata.insert(
            "cbsem_imply_all_exogenous_latent_correlations".into(),
            "true".into(),
        );
        let special_artifact = compile_analysis_recipe_v4(
            &special_recipe,
            Some(&special_model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        let error = estimate_cbsem_ml_compiled_moments_v2(
            &raw,
            &special_artifact,
            &special_recipe,
            &special_model,
        )
        .unwrap_err();
        let CbsemCompiledMomentErrorV2::UnsupportedPlan { issues } = error else {
            panic!("expected explicit Special Assumption blocker")
        };
        assert!(issues.iter().any(|issue| {
            issue.code == "special_assumption_requires_materialized_parameters"
                && issue.subject == "cbsem_imply_all_exogenous_latent_correlations"
        }));
    }

    #[test]
    fn matrix_variable_order_changes_identity_but_not_scientific_result() {
        let covariance = sample_covariance(&raw_rows());
        let canonical = matrix_dataset(DataKind::Covariance, 40, &covariance, &[0, 1, 2]);
        let canonical_model = sem_model(covariance_binding(&canonical, &[0, 1, 2]));
        let (canonical_recipe, canonical_artifact) =
            compile(&canonical, &canonical_model, CbsemInput::Covariance);
        let canonical_result = estimate_cbsem_ml_compiled_moments_v2(
            &canonical,
            &canonical_artifact,
            &canonical_recipe,
            &canonical_model,
        )
        .unwrap();

        let reordered = matrix_dataset(DataKind::Covariance, 40, &covariance, &[2, 0, 1]);
        let reordered_model = sem_model(covariance_binding(&reordered, &[2, 0, 1]));
        let (reordered_recipe, reordered_artifact) =
            compile(&reordered, &reordered_model, CbsemInput::Covariance);
        let reordered_result = estimate_cbsem_ml_compiled_moments_v2(
            &reordered,
            &reordered_artifact,
            &reordered_recipe,
            &reordered_model,
        )
        .unwrap();
        assert_ne!(
            canonical_artifact.receipt().analytical_identity_sha256(),
            reordered_artifact.receipt().analytical_identity_sha256()
        );
        assert_results_close(&canonical_result, &reordered_result);
    }

    #[test]
    fn correlation_scale_means_group_and_raw_missing_options_fail_with_typed_capability_codes() {
        let (correlation, _) = sample_correlation_and_sd(&raw_rows());
        let dataset = matrix_dataset(DataKind::Correlation, 40, &correlation, &[0, 1, 2]);
        let missing_scale = sem_model(correlation_binding(&dataset, &[0, 1, 2], None));
        let plan = compile_cbsem_plan_v2(&missing_scale).unwrap();
        let codes = validate_cbsem_ml_v1_estimator_capability_v2(&plan)
            .into_iter()
            .map(|issue| issue.code)
            .collect::<HashSet<_>>();
        assert!(codes.contains("correlation_scale_metadata_required"));

        let covariance = sample_covariance(&raw_rows());
        let covariance_data = matrix_dataset(DataKind::Covariance, 40, &covariance, &[0, 1, 2]);
        let mut moments = sem_model(covariance_binding(&covariance_data, &[0, 1, 2]));
        if let qpls_core::SemDataBindingV4::Covariance { means, sample, .. } =
            &mut moments.data_binding
        {
            *means = Some(BTreeMap::from([
                ("observed:x1".into(), 0.0),
                ("observed:x2".into(), 0.0),
                ("observed:x3".into(), 0.0),
            ]));
            sample.effective_sample_size = Some(39.5);
        }
        let codes =
            validate_cbsem_ml_v1_estimator_capability_v2(&compile_cbsem_plan_v2(&moments).unwrap())
                .into_iter()
                .map(|issue| issue.code)
                .collect::<HashSet<_>>();
        assert!(codes.contains("matrix_means_unsupported"));
        assert!(codes.contains("matrix_effective_sample_size_unsupported"));

        let raw = raw_dataset();
        let mut grouped = sem_model(raw_binding(&raw));
        grouped.group = SemGroupV4::ObservedGroups {
            grouping_variable: "observed:x1".into(),
            levels: vec![
                SemGroupLevelV4 {
                    id: "a".into(),
                    value: "A".into(),
                    label: "A".into(),
                },
                SemGroupLevelV4 {
                    id: "b".into(),
                    value: "B".into(),
                    label: "B".into(),
                },
            ],
        };
        if let qpls_core::SemDataBindingV4::Raw { missing_data, .. } = &mut grouped.data_binding {
            *missing_data = MissingDataPolicyV4::MeanReplacement;
        }
        let codes =
            validate_cbsem_ml_v1_estimator_capability_v2(&compile_cbsem_plan_v2(&grouped).unwrap())
                .into_iter()
                .map(|issue| issue.code)
                .collect::<HashSet<_>>();
        assert!(codes.contains("multigroup"));
        assert!(codes.contains("raw_data_options"));
    }

    #[test]
    fn singular_matrix_and_wrong_shape_fail_before_optimizer() {
        let singular = vec![vec![1.0; 3]; 3];
        let dataset = matrix_dataset(DataKind::Covariance, 40, &singular, &[0, 1, 2]);
        let model = sem_model(covariance_binding(&dataset, &[0, 1, 2]));
        let (recipe, artifact) = compile(&dataset, &model, CbsemInput::Covariance);
        assert!(matches!(
            estimate_cbsem_ml_compiled_moments_v2(&dataset, &artifact, &recipe, &model),
            Err(CbsemCompiledMomentErrorV2::MatrixNotPositiveDefinite { .. })
        ));

        let covariance = sample_covariance(&raw_rows());
        let valid = matrix_dataset(DataKind::Covariance, 40, &covariance, &[0, 1, 2]);
        let valid_model = sem_model(covariance_binding(&valid, &[0, 1, 2]));
        let plan = compile_cbsem_plan_v2(&valid_model).unwrap();
        let batch = RecordBatch::try_from_iter(vec![
            (
                "x1",
                Arc::new(Float64Array::from(vec![1.0, 0.2])) as ArrayRef,
            ),
            (
                "x2",
                Arc::new(Float64Array::from(vec![0.2, 1.0])) as ArrayRef,
            ),
            (
                "x3",
                Arc::new(Float64Array::from(vec![0.1, 0.3])) as ArrayRef,
            ),
        ])
        .unwrap();
        let mut malformed = valid.clone();
        malformed.batch = batch;
        malformed.schema.case_count = 2;
        assert!(matches!(
            prepare_moments(
                &plan,
                &malformed,
                &["x1".into(), "x2".into(), "x3".into()],
                false,
                &|| false,
            ),
            Err(CbsemCompiledMomentErrorV2::MatrixShape {
                expected: 3,
                rows: 2,
                columns: 3
            })
        ));
    }

    #[test]
    fn artifact_dataset_and_matrix_tampering_fail_closed() {
        let covariance = sample_covariance(&raw_rows());
        let dataset = matrix_dataset(DataKind::Covariance, 40, &covariance, &[0, 1, 2]);
        let model = sem_model(covariance_binding(&dataset, &[0, 1, 2]));
        let (recipe, artifact) = compile(&dataset, &model, CbsemInput::Covariance);

        let mut artifact_json = serde_json::to_value(&artifact).unwrap();
        artifact_json["plan"]["plan"]["model_name"] = serde_json::json!("tampered");
        let tampered_artifact: CompiledAnalysisRecipeV4 =
            serde_json::from_value(artifact_json).unwrap();
        assert!(matches!(
            estimate_cbsem_ml_compiled_moments_v2(&dataset, &tampered_artifact, &recipe, &model),
            Err(CbsemCompiledMomentErrorV2::CompiledArtifact(
                RecipeV4CompilationError::ArtifactMismatch
            ))
        ));

        let mut wrong_fingerprint = dataset.clone();
        wrong_fingerprint.fingerprint.0 = format!("v2:{}", "0".repeat(64));
        assert!(matches!(
            estimate_cbsem_ml_compiled_moments_v2(&wrong_fingerprint, &artifact, &recipe, &model),
            Err(CbsemCompiledMomentErrorV2::DatasetFingerprintMismatch)
        ));

        let mut changed_bytes = dataset.clone();
        let mut columns = changed_bytes
            .batch
            .columns()
            .iter()
            .map(|column| {
                column
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .unwrap()
                    .values()
                    .to_vec()
            })
            .collect::<Vec<_>>();
        columns[0][0] += 0.25;
        changed_bytes.batch = RecordBatch::try_from_iter(
            ["x1", "x2", "x3"]
                .into_iter()
                .zip(columns)
                .map(|(name, values)| (name, Arc::new(Float64Array::from(values)) as ArrayRef)),
        )
        .unwrap();
        assert!(matches!(
            estimate_cbsem_ml_compiled_moments_v2(&changed_bytes, &artifact, &recipe, &model),
            Err(CbsemCompiledMomentErrorV2::DatasetIntegrity(_))
        ));
    }

    #[test]
    fn matrix_sample_size_is_exact_and_ml_denominator_is_not_rescaled() {
        let covariance_sample = sample_covariance(&raw_rows());
        let dataset = matrix_dataset(DataKind::Covariance, 40, &covariance_sample, &[0, 1, 2]);
        let mut mismatch_model = sem_model(covariance_binding(&dataset, &[0, 1, 2]));
        if let qpls_core::SemDataBindingV4::Covariance { sample, .. } =
            &mut mismatch_model.data_binding
        {
            sample.sample_size = 41;
        }
        let (mismatch_recipe, mismatch_artifact) =
            compile(&dataset, &mismatch_model, CbsemInput::Covariance);
        assert!(matches!(
            estimate_cbsem_ml_compiled_moments_v2(
                &dataset,
                &mismatch_artifact,
                &mismatch_recipe,
                &mismatch_model
            ),
            Err(CbsemCompiledMomentErrorV2::SampleSizeMismatch {
                declared: 41,
                actual: Some(40)
            })
        ));

        let mut covariance_ml = covariance_sample.clone();
        for row in &mut covariance_ml {
            for value in row {
                *value *= 39.0 / 40.0;
            }
        }
        let ml_dataset = matrix_dataset(DataKind::Covariance, 40, &covariance_ml, &[0, 1, 2]);
        let mut ml_binding = covariance_binding(&ml_dataset, &[0, 1, 2]);
        if let qpls_core::SemDataBindingV4::Covariance { sample, .. } = &mut ml_binding {
            sample.covariance_denominator = SemCovarianceDenominatorV4::MaximumLikelihoodN;
        }
        let ml_model = sem_model(ml_binding);
        let (ml_recipe, ml_artifact) = compile(&ml_dataset, &ml_model, CbsemInput::Covariance);
        let ml_result =
            estimate_cbsem_ml_compiled_moments_v2(&ml_dataset, &ml_artifact, &ml_recipe, &ml_model)
                .unwrap();
        for (actual, expected) in ml_result
            .covariance_ml
            .iter()
            .flatten()
            .zip(covariance_ml.iter().flatten())
        {
            assert!((actual - expected).abs() <= 1e-12);
        }
    }

    fn two_factor_sem_model(
        dataset: &Dataset,
        covariance: Option<CompiledCbsemParameterStatusV2>,
    ) -> qpls_core::SemModelV4 {
        let legacy = ModelSpec {
            id: uuid::Uuid::from_u128(0xCB5E_4003),
            name: "Two-factor exact covariance microcase".into(),
            constructs: vec![
                Construct {
                    id: "f".into(),
                    name: "Factor F".into(),
                    short_name: "F".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into(), "x3".into()],
                },
                Construct {
                    id: "g".into(),
                    name: "Factor G".into(),
                    short_name: "G".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["y1".into(), "y2".into(), "y3".into()],
                },
            ],
            paths: Vec::new(),
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
        fix_all_free_parameters(&mut model);
        if let Some(specification) = covariance {
            let left = SemEndpointV4::Variable("construct:f".into());
            let right = SemEndpointV4::Variable("construct:g".into());
            let target = SemParameterTargetV4::Covariance {
                left: left.clone(),
                right: right.clone(),
            };
            model.relations.push(qpls_core::SemRelationV4::Covariance {
                id: "covariance:f:g".into(),
                left,
                right,
                parameter: "parameter:covariance:f:g".into(),
            });
            model.parameters.push(match specification {
                CompiledCbsemParameterStatusV2::Free {
                    start,
                    lower,
                    upper,
                    equality_label,
                } => qpls_core::SemParameterV4::Free {
                    id: "parameter:covariance:f:g".into(),
                    label: "Cov(F,G)".into(),
                    target,
                    start,
                    lower,
                    upper,
                    equality_label,
                    group_overrides: Vec::new(),
                },
                CompiledCbsemParameterStatusV2::Fixed { value } => {
                    qpls_core::SemParameterV4::Fixed {
                        id: "parameter:covariance:f:g".into(),
                        label: "Cov(F,G)".into(),
                        target,
                        value,
                        group_overrides: Vec::new(),
                    }
                }
                CompiledCbsemParameterStatusV2::Derived { expression } => {
                    qpls_core::SemParameterV4::Derived {
                        id: "parameter:covariance:f:g".into(),
                        label: "Cov(F,G)".into(),
                        target,
                        expression,
                        group_overrides: Vec::new(),
                    }
                }
            });
        }
        let observed_ids = ["x1", "x2", "x3", "y1", "y2", "y3"]
            .into_iter()
            .map(|name| format!("observed:{name}"))
            .collect::<Vec<_>>();
        model.data_binding = named_covariance_binding(dataset, &observed_ids, 500);
        model.ensure_valid().unwrap();
        model
    }

    fn reciprocal_two_factor_sem_model(
        dataset: &Dataset,
        f_to_g: f64,
        g_to_f: f64,
    ) -> qpls_core::SemModelV4 {
        let mut model = two_factor_sem_model(dataset, None);
        for factor in ["construct:f", "construct:g"] {
            let variance_parameter = match model
                .variables
                .iter_mut()
                .find(|variable| variable.id() == factor)
                .unwrap()
            {
                qpls_core::SemVariableV4::CommonFactor {
                    disturbance_policy, ..
                } => match disturbance_policy {
                    qpls_core::FactorDisturbancePolicyV4::ExogenousVariance { parameter } => {
                        let parameter = parameter.clone();
                        *disturbance_policy =
                            qpls_core::FactorDisturbancePolicyV4::EndogenousDisturbance {
                                parameter: parameter.clone(),
                            };
                        parameter
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            };
            let endpoint = SemEndpointV4::DisturbanceOf(factor.into());
            match model
                .parameters
                .iter_mut()
                .find(|parameter| parameter.id() == variance_parameter)
                .unwrap()
            {
                qpls_core::SemParameterV4::Free { target, .. }
                | qpls_core::SemParameterV4::Fixed { target, .. }
                | qpls_core::SemParameterV4::Derived { target, .. } => {
                    *target = SemParameterTargetV4::Variance { endpoint };
                }
            }
        }
        for (id, source, target, value) in [
            ("path:f:g", "construct:f", "construct:g", f_to_g),
            ("path:g:f", "construct:g", "construct:f", g_to_f),
        ] {
            let parameter_id = format!("parameter:{id}");
            model.relations.push(qpls_core::SemRelationV4::Structural {
                id: id.into(),
                source: source.into(),
                target: target.into(),
                parameter: parameter_id.clone(),
                role: qpls_core::StructuralRelationRoleV4::Structural,
                intercept_parameter: None,
            });
            model.parameters.push(qpls_core::SemParameterV4::Fixed {
                id: parameter_id,
                label: format!("{source} -> {target}"),
                target: SemParameterTargetV4::Regression {
                    source: source.into(),
                    target: target.into(),
                },
                value,
                group_overrides: Vec::new(),
            });
        }
        model.ensure_valid().unwrap();
        model
    }

    fn fix_all_free_parameters(model: &mut qpls_core::SemModelV4) {
        for parameter in &mut model.parameters {
            let replacement = match parameter.clone() {
                qpls_core::SemParameterV4::Free {
                    id,
                    label,
                    target,
                    start,
                    group_overrides,
                    ..
                } => Some(qpls_core::SemParameterV4::Fixed {
                    id,
                    label,
                    value: start.unwrap_or_else(|| match target {
                        SemParameterTargetV4::Loading { .. } => 0.7,
                        SemParameterTargetV4::Regression { .. }
                        | SemParameterTargetV4::Covariance { .. } => 0.0,
                        SemParameterTargetV4::Variance { .. } => 0.5,
                        _ => 0.0,
                    }),
                    target,
                    group_overrides,
                }),
                _ => None,
            };
            if let Some(replacement) = replacement {
                *parameter = replacement;
            }
        }
    }

    fn two_factor_covariance() -> Vec<Vec<f64>> {
        let loadings = [1.0, 0.7, 0.7, 1.0, 0.7, 0.7];
        (0..6)
            .map(|row| {
                (0..6)
                    .map(|column| {
                        let same_factor = row / 3 == column / 3;
                        let latent_covariance = if same_factor { 1.0 } else { 0.3 };
                        loadings[row] * loadings[column] * latent_covariance
                            + if row == column { 0.5 } else { 0.0 }
                    })
                    .collect()
            })
            .collect()
    }

    fn implied_cell(result: &CbsemCompiledMomentResultV2, row: &str, column: &str) -> f64 {
        result
            .analysis
            .implied_covariance
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .unwrap()
            .value
    }

    #[test]
    fn admissible_nonrecursive_system_executes_with_parameter_identity_and_stability_witness() {
        let names = ["x1", "x2", "x3", "y1", "y2", "y3"];
        let dataset =
            named_matrix_dataset(DataKind::Covariance, 500, &names, &two_factor_covariance());
        let model = reciprocal_two_factor_sem_model(&dataset, 0.2, 0.3);
        assert!(compile_cbsem_plan_v2(&model).unwrap().has_feedback());
        let (recipe, artifact) = compile_sem(&dataset, &model, CbsemInput::Covariance);

        let result =
            estimate_cbsem_ml_compiled_moments_v2(&dataset, &artifact, &recipe, &model).unwrap();

        assert!(result.analysis.converged);
        assert!(result.analysis.objective.is_finite());
        assert_eq!(
            result
                .parameter_ids
                .get("construct:g~construct:f")
                .map(String::as_str),
            Some("parameter:path:f:g")
        );
        assert_eq!(
            result
                .parameter_ids
                .get("construct:f~construct:g")
                .map(String::as_str),
            Some("parameter:path:g:f")
        );
        assert!(result.analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("Structural-system stability: admissible")
                && diagnostic.contains("0.244948")
        }));
    }

    #[test]
    fn unstable_nonrecursive_system_fails_with_typed_stability_diagnostic() {
        let names = ["x1", "x2", "x3", "y1", "y2", "y3"];
        let dataset =
            named_matrix_dataset(DataKind::Covariance, 500, &names, &two_factor_covariance());
        let model = reciprocal_two_factor_sem_model(&dataset, 1.2, 1.0);
        let (recipe, artifact) = compile_sem(&dataset, &model, CbsemInput::Covariance);

        assert!(matches!(
            estimate_cbsem_ml_compiled_moments_v2(&dataset, &artifact, &recipe, &model),
            Err(CbsemCompiledMomentErrorV2::ExactParameterTable(
                CbsemExactParameterTableErrorV3::StructuralSystemUnstable {
                    stage,
                    spectral_radius,
                    maximum,
                }
            )) if stage == "declared start" && spectral_radius > 1.0 && maximum < 1.0
        ));
    }

    #[test]
    fn undeclared_latent_covariance_is_zero_and_declared_fixed_covariance_changes_results() {
        let names = ["x1", "x2", "x3", "y1", "y2", "y3"];
        let dataset =
            named_matrix_dataset(DataKind::Covariance, 500, &names, &two_factor_covariance());
        let no_covariance = two_factor_sem_model(&dataset, None);
        let (no_recipe, no_artifact) = compile(&dataset, &no_covariance, CbsemInput::Covariance);
        let no_result = estimate_cbsem_ml_compiled_moments_v2(
            &dataset,
            &no_artifact,
            &no_recipe,
            &no_covariance,
        )
        .unwrap();
        assert_eq!(
            implied_cell(&no_result, "x1", "y1").to_bits(),
            0.0_f64.to_bits()
        );
        assert!(
            !no_result
                .parameter_ids
                .contains_key("construct:f~~construct:g")
        );

        let declared = two_factor_sem_model(
            &dataset,
            Some(CompiledCbsemParameterStatusV2::Fixed { value: 0.3 }),
        );
        let (declared_recipe, declared_artifact) =
            compile(&dataset, &declared, CbsemInput::Covariance);
        let declared_result = estimate_cbsem_ml_compiled_moments_v2(
            &dataset,
            &declared_artifact,
            &declared_recipe,
            &declared,
        )
        .unwrap();
        assert!((implied_cell(&declared_result, "x1", "y1") - 0.3).abs() <= 1e-12);
        assert_eq!(
            declared_result
                .parameter_ids
                .get("construct:f~~construct:g")
                .map(String::as_str),
            Some("parameter:covariance:f:g")
        );
        assert_ne!(no_result.plan_sha256, declared_result.plan_sha256);
        assert_eq!(
            declared_result.method_version,
            CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3
        );
    }

    #[test]
    fn declared_fixed_residual_covariance_changes_only_the_explicit_theta_cell() {
        let covariance = sample_covariance(&raw_rows());
        let dataset = matrix_dataset(DataKind::Covariance, 40, &covariance, &[0, 1, 2]);
        let mut base = sem_model(covariance_binding(&dataset, &[0, 1, 2]));
        fix_all_free_parameters(&mut base);
        base.ensure_valid().unwrap();
        let (base_recipe, base_artifact) = compile(&dataset, &base, CbsemInput::Covariance);
        let base_result =
            estimate_cbsem_ml_compiled_moments_v2(&dataset, &base_artifact, &base_recipe, &base)
                .unwrap();

        let mut correlated = base.clone();
        let left = SemEndpointV4::ResidualOf("observed:x1".into());
        let right = SemEndpointV4::ResidualOf("observed:x2".into());
        correlated
            .relations
            .push(qpls_core::SemRelationV4::Covariance {
                id: "residual:x1:x2".into(),
                left: left.clone(),
                right: right.clone(),
                parameter: "parameter:residual:x1:x2".into(),
            });
        correlated
            .parameters
            .push(qpls_core::SemParameterV4::Fixed {
                id: "parameter:residual:x1:x2".into(),
                label: "Residual covariance x1 x2".into(),
                target: SemParameterTargetV4::Covariance { left, right },
                value: 0.2,
                group_overrides: Vec::new(),
            });
        correlated.ensure_valid().unwrap();
        let (correlated_recipe, correlated_artifact) =
            compile(&dataset, &correlated, CbsemInput::Covariance);
        let correlated_result = estimate_cbsem_ml_compiled_moments_v2(
            &dataset,
            &correlated_artifact,
            &correlated_recipe,
            &correlated,
        )
        .unwrap();
        assert!(
            (implied_cell(&correlated_result, "x1", "x2")
                - implied_cell(&base_result, "x1", "x2")
                - 0.2)
                .abs()
                <= 1e-12
        );
        assert_eq!(
            correlated_result
                .analysis
                .parameters
                .iter()
                .find(|parameter| parameter.name == "x1~~x2")
                .map(|parameter| (parameter.kind.as_str(), parameter.fixed)),
            Some(("residual_covariance", true))
        );
    }

    #[test]
    fn equality_label_bounds_and_conflicting_starts_are_fail_closed() {
        let covariance = sample_covariance(&raw_rows());
        let dataset = matrix_dataset(DataKind::Covariance, 40, &covariance, &[0, 1, 2]);
        let mut equal = sem_model(covariance_binding(&dataset, &[0, 1, 2]));
        let mut matched = 0;
        for parameter in &mut equal.parameters {
            if let qpls_core::SemParameterV4::Free {
                target: SemParameterTargetV4::Loading { indicator, .. },
                start,
                lower,
                upper,
                equality_label,
                ..
            } = parameter
            {
                if indicator == "observed:x2" || indicator == "observed:x3" {
                    *start = Some(0.7);
                    *lower = Some(0.2);
                    *upper = Some(1.5);
                    *equality_label = Some("equal-secondary-loadings".into());
                    matched += 1;
                }
            }
        }
        assert_eq!(matched, 2);
        equal.ensure_valid().unwrap();
        let (equal_recipe, equal_artifact) = compile(&dataset, &equal, CbsemInput::Covariance);
        let equal_result =
            estimate_cbsem_ml_compiled_moments_v2(&dataset, &equal_artifact, &equal_recipe, &equal)
                .unwrap();
        let estimates = equal_result
            .analysis
            .parameters
            .iter()
            .filter(|parameter| {
                parameter.name == "construct:f=~x2" || parameter.name == "construct:f=~x3"
            })
            .map(|parameter| parameter.estimate)
            .collect::<Vec<_>>();
        assert_eq!(estimates.len(), 2);
        assert_eq!(estimates[0].to_bits(), estimates[1].to_bits());
        assert!(estimates[0] > 0.2 && estimates[0] < 1.5);

        let mut conflict = equal.clone();
        for parameter in &mut conflict.parameters {
            if let qpls_core::SemParameterV4::Free {
                target: SemParameterTargetV4::Loading { indicator, .. },
                start,
                ..
            } = parameter
            {
                if indicator == "observed:x3" {
                    *start = Some(0.8);
                }
            }
        }
        let (conflict_recipe, conflict_artifact) =
            compile(&dataset, &conflict, CbsemInput::Covariance);
        assert!(matches!(
            estimate_cbsem_ml_compiled_moments_v2(
                &dataset,
                &conflict_artifact,
                &conflict_recipe,
                &conflict
            ),
            Err(CbsemCompiledMomentErrorV2::ExactParameterTable(
                CbsemExactParameterTableErrorV3::EqualityStartConflict { .. }
            ))
        ));
    }

    #[test]
    fn non_positive_definite_declared_residual_covariance_is_typed() {
        let covariance = sample_covariance(&raw_rows());
        let dataset = matrix_dataset(DataKind::Covariance, 40, &covariance, &[0, 1, 2]);
        let mut model = sem_model(covariance_binding(&dataset, &[0, 1, 2]));
        fix_all_free_parameters(&mut model);
        let left = SemEndpointV4::ResidualOf("observed:x1".into());
        let right = SemEndpointV4::ResidualOf("observed:x2".into());
        model.relations.push(qpls_core::SemRelationV4::Covariance {
            id: "residual:non-pd".into(),
            left: left.clone(),
            right: right.clone(),
            parameter: "parameter:residual:non-pd".into(),
        });
        model.parameters.push(qpls_core::SemParameterV4::Fixed {
            id: "parameter:residual:non-pd".into(),
            label: "Impossible residual covariance".into(),
            target: SemParameterTargetV4::Covariance { left, right },
            value: 0.75,
            group_overrides: Vec::new(),
        });
        model.ensure_valid().unwrap();
        let (recipe, artifact) = compile(&dataset, &model, CbsemInput::Covariance);
        assert!(matches!(
            estimate_cbsem_ml_compiled_moments_v2(&dataset, &artifact, &recipe, &model),
            Err(CbsemCompiledMomentErrorV2::ExactParameterTable(
                CbsemExactParameterTableErrorV3::InitialResidualCovarianceNotPositiveDefinite
            ))
        ));
    }

    #[test]
    fn exact_delete_one_refit_is_distinct_deterministic_and_uses_n_minus_one_ml_rows() {
        let raw = raw_dataset();
        let model = sem_model(raw_binding(&raw));
        let (recipe, artifact) = compile(&raw, &model, CbsemInput::Raw);
        let source =
            prepare_cbsem_ml_exact_case_bootstrap_source_v1(&raw, &artifact, &recipe, &model)
                .unwrap();
        let first = estimate_cbsem_ml_exact_case_delete_one_v1(&source, 3).unwrap();
        let second = estimate_cbsem_ml_exact_case_delete_one_v1(&source, 3).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.method_version,
            CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1
        );
        assert_eq!(first.omitted_complete_case_position, 3);
        assert_eq!(first.omitted_source_row_index, 3);
        assert_eq!(
            first.retained_observations + 1,
            source.complete_case_sample_size()
        );
        assert_eq!(
            first.covariance_denominator,
            SemCovarianceDenominatorV4::MaximumLikelihoodN
        );
        assert!(
            first
                .free_parameters
                .windows(2)
                .all(|pair| pair[0].parameter_id < pair[1].parameter_id)
        );
        assert!(matches!(
            estimate_cbsem_ml_exact_case_delete_one_v1(&source, source.complete_case_sample_size()),
            Err(CbsemCompiledMomentErrorV2::ExactCaseBootstrapOmittedPositionOutOfRange { .. })
        ));
        assert!(matches!(
            estimate_cbsem_ml_exact_case_delete_one_v1_with_control(&source, 0, || true, |_| {}),
            Err(CbsemCompiledMomentErrorV2::Cancelled)
        ));
    }

    #[test]
    fn exact_delete_one_rejects_nine_retained_rows_for_ten_case_source() {
        let mut csv = String::from("x1,x2,x3\n");
        for row in raw_rows().into_iter().take(10) {
            csv.push_str(&format!("{:.17},{:.17},{:.17}\n", row[0], row[1], row[2]));
        }
        let raw = import_delimited_bytes(
            csv.as_bytes(),
            "raw-ten-case-microcase.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let model = sem_model(raw_binding(&raw));
        let (recipe, artifact) = compile(&raw, &model, CbsemInput::Raw);
        let source =
            prepare_cbsem_ml_exact_case_bootstrap_source_v1(&raw, &artifact, &recipe, &model)
                .unwrap();
        assert_eq!(source.complete_case_sample_size(), 10);
        assert!(matches!(
            estimate_cbsem_ml_exact_case_delete_one_v1(&source, 0),
            Err(CbsemCompiledMomentErrorV2::ExactCaseBootstrapInsufficientObservations(9))
        ));
    }
}
