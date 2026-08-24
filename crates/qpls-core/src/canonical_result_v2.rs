use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub const CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION: u32 = 2;
pub const CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION: u32 = 1;
pub const GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1: &str =
    "general_sem_pls_full_model_case_bootstrap_v1";
pub const GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1: &str =
    "general_sem_pls_case_bootstrap_v1";
pub const GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_METHOD_VERSION_V1: &str =
    "general_sem_pls_single_mediation_full_model_case_bootstrap_v1";
pub const GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1: &str =
    "general_sem_pls_single_mediation_case_bootstrap_v1";
pub const GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1: &str =
    "indexed_case_resampling_v1";
pub const GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1: &str = "type7_quantile_v1";
pub const GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1: &str =
    "sample_standard_error_b_minus_1_v1";
pub const GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1: &str = "neumaier_compensated_sum_v1";
pub const GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1: &str =
    "null_centered_plus_one_v1";
pub const GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1: &str =
    "minimum_usable_fraction_0_9_v1";
pub const GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1: &str =
    "qpls.general-sem-pls.multiple-two-way.point.v1";
pub const GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1: &str =
    "qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1";
pub const GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1: &str =
    "general_sem_pls_multiple_two_way_moderation_case_bootstrap_v1";
pub const GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1: &str =
    "qpls.general-sem-pls.three-way.point.v1";
pub const GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1: &str =
    "qpls.general-sem-pls.three-way.full-model-case-bootstrap.v1";
pub const GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1: &str =
    "general_sem_pls_three_way_moderation_case_bootstrap_v1";
pub const GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1: &str =
    "qpls.general-sem-pls.three-way.fixed-probes.v1";
pub const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1: &str =
    "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1";
pub const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1: &str =
    "general_sem_pls_two_way_moderated_mediation_case_bootstrap_v1";
pub const GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1: &str =
    "qpls.general-sem-pls.two-stage-product.sample-standardized.v1";
pub const GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1: &str =
    "qpls.general-sem-pls.simple-slope.other-moderators-zero.v1";
pub const GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1: &str =
    "qpls.general-sem-pls.interaction-hierarchy.strong.v1";
pub const CBSEM_RECURSIVE_SEM_BOOTSTRAP_METHOD_VERSION_V1: &str =
    "cbsem_exact_recursive_sem_case_bootstrap_v1";
pub const CBSEM_RECURSIVE_SEM_BOOTSTRAP_OPERATION_VERSION_V1: &str =
    "cbsem_recursive_sem_full_ml_case_bootstrap_v1";
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCellReferenceV2 {
    pub registry_schema_version: u32,
    pub capability_id: String,
    pub cell_id: String,
    pub capability_version: String,
}

pub fn capability_cell_reference_identity_v2(reference: &CapabilityCellReferenceV2) -> String {
    format!(
        "{}:{}:{}:{}",
        reference.registry_schema_version,
        reference.capability_id,
        reference.cell_id,
        reference.capability_version
    )
}

pub fn general_sem_pls_bootstrap_capability_cell_v1() -> CapabilityCellReferenceV2 {
    crate::pls_general_bootstrap_capability_cell_v1()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalColumnType {
    Number,
    Text,
    Boolean,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalMissingReason {
    NotApplicable,
    NotEstimated,
    Undefined,
    Withheld,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalResultCell {
    Number {
        value: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display: Option<String>,
    },
    Text {
        value: String,
    },
    Boolean {
        value: bool,
    },
    Missing {
        reason: CanonicalMissingReason,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display: Option<String>,
    },
}

impl CanonicalResultCell {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Number { .. } => "number",
            Self::Text { .. } => "text",
            Self::Boolean { .. } => "boolean",
            Self::Missing { .. } => "missing",
        }
    }

    fn matches_column(&self, data_type: CanonicalColumnType) -> bool {
        matches!(
            (self, data_type),
            (Self::Missing { .. }, _)
                | (Self::Number { .. }, CanonicalColumnType::Number)
                | (Self::Text { .. }, CanonicalColumnType::Text)
                | (Self::Boolean { .. }, CanonicalColumnType::Boolean)
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalColumnRole {
    Label,
    Estimate,
    Uncertainty,
    Decision,
    Diagnostic,
    Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultColumn {
    pub id: String,
    pub label: String,
    pub data_type: CanonicalColumnType,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<CanonicalColumnRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_precision: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultRow {
    pub id: String,
    pub cells: Vec<CanonicalResultCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultTable {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub columns: Vec<CanonicalResultColumn>,
    pub rows: Vec<CanonicalResultRow>,
    pub footnote_ids: Vec<String>,
    /// Explicit option cells that produced this table. Missing only for
    /// historical compatibility documents, which are not comparison or
    /// qualification-export eligible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cells: Option<Vec<CapabilityCellReferenceV2>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum CanonicalChartX {
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalChartPoint {
    pub x: CanonicalChartX,
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalChartSeries {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub points: Vec<CanonicalChartPoint>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalChartDisplayOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_legend: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_values: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_axis_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_axis_label: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalChartKind {
    Line,
    Bar,
    Scatter,
    Interval,
    Heatmap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultChart {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: CanonicalChartKind,
    pub series: Vec<CanonicalChartSeries>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_table_id: Option<String>,
    pub display: CanonicalChartDisplayOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultSection {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub table_ids: Vec<String>,
    pub chart_ids: Vec<String>,
    /// Explicit union of option cells represented by this section. Missing
    /// only for historical compatibility documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cells: Option<Vec<CapabilityCellReferenceV2>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalNoticeSeverity {
    Information,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultNotice {
    pub id: String,
    pub code: String,
    pub severity: CanonicalNoticeSeverity,
    pub message: String,
    pub section_ids: Vec<String>,
    pub table_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultExclusion {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cell: Option<CapabilityCellReferenceV2>,
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultFootnote {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultProvenanceV2 {
    pub run_id: String,
    pub project_id: String,
    pub model_id: String,
    pub model_digest: String,
    pub dataset_id: String,
    pub dataset_fingerprint: String,
    pub recipe_id: String,
    pub recipe_digest: String,
    pub capability_cell: CapabilityCellReferenceV2,
    pub method_version: String,
    pub engine_version: String,
    pub seed: Option<i64>,
    pub workers: i64,
    pub started_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultPresentationV2 {
    pub default_section_id: Option<String>,
    pub default_table_id: Option<String>,
    pub precision: i32,
    pub missing_value_label: String,
    pub chart_defaults: CanonicalChartDisplayOptions,
}

/// Exact model and qualified capability cell that produced a typed General SEM
/// result. Specialized result rows add relation, path, interaction, or
/// higher-order identities without replacing this common trace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalGeneralSemResultTraceV1 {
    pub model_id: String,
    pub capability_cell: CapabilityCellReferenceV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalGeneralSemEstimateV1 {
    pub estimate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_mean: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_bias: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard_error: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_usable_replicates: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_two_sided_exceedances: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalGeneralSemInferenceKindV1 {
    CaseBootstrap,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalGeneralSemBootstrapIntervalV1 {
    PercentileType7,
    Bca,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalGeneralSemInferenceTailV1 {
    TwoSided,
    OneSidedLower,
    OneSidedUpper,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalGeneralSemFailedReplicateV1 {
    pub replicate_index: u32,
    pub reason_code: CanonicalGeneralSemFailedReplicateReasonV1,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalGeneralSemFailedReplicateReasonV1 {
    InsufficientObservations,
    ConstantIndicator,
    StageOneRankDeficient,
    StageOneNonconvergence,
    IndeterminateScoreSign,
    ConstantConstructScore,
    ConstantInteractionProduct,
    RankDeficient,
    JointStageRankDeficient,
    IsolatedConstruct,
    EstimationNonconvergence,
    NumericalFailure,
    TargetInventoryMismatch,
}

/// Exact resampling and identity receipt for inferred General SEM effect rows.
/// The first executor slice uses indexed, full-model PLS case resampling; the
/// versioned fields prevent later algorithms from being read as that slice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalGeneralSemInferenceReceiptV1 {
    pub kind: CanonicalGeneralSemInferenceKindV1,
    pub capability_cell: CapabilityCellReferenceV2,
    /// Cross-capability scientific dependencies of a combined operation.
    /// Historical single-owner receipts omit this collection byte-for-byte.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_dependencies: Vec<CapabilityCellReferenceV2>,
    pub method_version: String,
    pub resampling_operation_version: String,
    pub resampling_stream_version: String,
    pub quantile_method_version: String,
    pub standard_error_method_version: String,
    pub summation_method_version: String,
    pub p_value_method_version: String,
    pub failure_policy_version: String,
    pub compilation_artifact_identity_sha256: String,
    pub compiled_plan_sha256: String,
    pub general_sem_config_sha256: String,
    pub recipe_analytical_sha256: String,
    pub model_scientific_sha256: String,
    pub source_dataset_fingerprint: String,
    pub complete_case_frame_sha256: String,
    pub usable_replicate_indices_sha256: String,
    pub effect_identity_set_sha256: String,
    pub effect_ids: Vec<String>,
    pub interval: CanonicalGeneralSemBootstrapIntervalV1,
    pub tail: CanonicalGeneralSemInferenceTailV1,
    pub confidence_level: f64,
    pub resamples_requested: u32,
    pub resamples_usable: u32,
    pub minimum_usable_resamples: u32,
    /// Decimal u64 wire form. This remains exact in JavaScript runtimes.
    pub seed: String,
    pub workers: u32,
    pub complete_model_reestimated_per_replicate: bool,
    pub failed_replicates: Vec<CanonicalGeneralSemFailedReplicateV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSpecificIndirectEffectResultV1 {
    pub effect_id: String,
    pub estimand_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub source_id: String,
    pub target_id: String,
    pub ordered_relation_ids: Vec<String>,
    pub value: CanonicalGeneralSemEstimateV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalAggregateEffectKindV1 {
    TotalIndirect,
    TotalEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalAggregateEffectResultV1 {
    pub effect_id: String,
    pub estimand_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub kind: CanonicalAggregateEffectKindV1,
    pub source_id: String,
    pub target_id: String,
    pub direct_relation_ids: Vec<String>,
    pub contributing_path_identities: Vec<String>,
    pub value: CanonicalGeneralSemEstimateV1,
}

/// Typed scientific identity hashed by an inference receipt. Estimates and
/// presentation are intentionally absent; authored/compiled estimand meaning
/// is fully bound.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalGeneralSemEffectIdentityV1 {
    SpecificIndirect {
        effect_id: String,
        estimand_id: String,
        source_id: String,
        target_id: String,
        ordered_relation_ids: Vec<String>,
    },
    TotalIndirect {
        effect_id: String,
        estimand_id: String,
        source_id: String,
        target_id: String,
        contributing_path_identities: Vec<String>,
    },
    TotalEffect {
        effect_id: String,
        estimand_id: String,
        source_id: String,
        target_id: String,
        direct_relation_ids: Vec<String>,
        contributing_path_identities: Vec<String>,
    },
    InteractionScientificRescaledGamma {
        effect_id: String,
        interaction_id: String,
        focal_relation_id: String,
        interaction_effect_relation_id: String,
        interaction_effect_parameter_id: String,
        generated_product_column_id: String,
        focal_predictor_id: String,
        moderator_id: String,
        outcome_id: String,
        stage_one_model_scientific_sha256: String,
        product_scale_version: String,
        method_version: String,
    },
    ConditionalIndirect {
        effect_id: String,
        target_id: String,
        estimand_id: String,
        moderated_stage: CanonicalModeratedMediationStageV1,
        interaction_id: String,
        x_id: String,
        mediator_id: String,
        y_id: String,
        moderator_id: String,
        ordered_relation_ids: Vec<String>,
        probe_value_index: u32,
        moderator_value_bits_hex: String,
    },
    ModeratedMediationIndex {
        effect_id: String,
        target_id: String,
        estimand_id: String,
        moderated_stage: CanonicalModeratedMediationStageV1,
        interaction_id: String,
        x_id: String,
        mediator_id: String,
        y_id: String,
        moderator_id: String,
        ordered_relation_ids: Vec<String>,
    },
}

impl CanonicalGeneralSemEffectIdentityV1 {
    pub fn effect_id(&self) -> &str {
        match self {
            Self::SpecificIndirect { effect_id, .. }
            | Self::TotalIndirect { effect_id, .. }
            | Self::TotalEffect { effect_id, .. }
            | Self::InteractionScientificRescaledGamma { effect_id, .. }
            | Self::ConditionalIndirect { effect_id, .. }
            | Self::ModeratedMediationIndex { effect_id, .. } => effect_id,
        }
    }
}

pub fn canonical_general_sem_effect_identities_v1(
    results: &CanonicalGeneralSemResultsV1,
) -> Vec<CanonicalGeneralSemEffectIdentityV1> {
    let mut identities = results
        .specific_indirect_effects
        .iter()
        .map(
            |effect| CanonicalGeneralSemEffectIdentityV1::SpecificIndirect {
                effect_id: effect.effect_id.clone(),
                estimand_id: effect.estimand_id.clone(),
                source_id: effect.source_id.clone(),
                target_id: effect.target_id.clone(),
                ordered_relation_ids: effect.ordered_relation_ids.clone(),
            },
        )
        .chain(
            results
                .aggregate_effects
                .iter()
                .map(|effect| match effect.kind {
                    CanonicalAggregateEffectKindV1::TotalIndirect => {
                        CanonicalGeneralSemEffectIdentityV1::TotalIndirect {
                            effect_id: effect.effect_id.clone(),
                            estimand_id: effect.estimand_id.clone(),
                            source_id: effect.source_id.clone(),
                            target_id: effect.target_id.clone(),
                            contributing_path_identities: effect
                                .contributing_path_identities
                                .clone(),
                        }
                    }
                    CanonicalAggregateEffectKindV1::TotalEffect => {
                        CanonicalGeneralSemEffectIdentityV1::TotalEffect {
                            effect_id: effect.effect_id.clone(),
                            estimand_id: effect.estimand_id.clone(),
                            source_id: effect.source_id.clone(),
                            target_id: effect.target_id.clone(),
                            direct_relation_ids: effect.direct_relation_ids.clone(),
                            contributing_path_identities: effect
                                .contributing_path_identities
                                .clone(),
                        }
                    }
                }),
        )
        .chain(results.interaction_effects.iter().map(|effect| {
            CanonicalGeneralSemEffectIdentityV1::InteractionScientificRescaledGamma {
                effect_id: effect.effect_id.clone(),
                interaction_id: effect.interaction_id.clone(),
                focal_relation_id: effect.focal_relation_id.clone(),
                interaction_effect_relation_id: effect.interaction_effect_relation_id.clone(),
                interaction_effect_parameter_id: effect.interaction_effect_parameter_id.clone(),
                generated_product_column_id: effect.generated_product_column_id.clone(),
                focal_predictor_id: effect.focal_predictor_id.clone(),
                moderator_id: effect.moderator_id.clone(),
                outcome_id: effect.outcome_id.clone(),
                stage_one_model_scientific_sha256: effect.stage_one_model_scientific_sha256.clone(),
                product_scale_version: effect.product_scale_version.clone(),
                method_version: effect.method_version.clone(),
            }
        }))
        .chain(results.conditional_indirect_effects.iter().map(|effect| {
            CanonicalGeneralSemEffectIdentityV1::ConditionalIndirect {
                effect_id: effect.effect_id.clone(),
                target_id: effect.target_id.clone(),
                estimand_id: effect.estimand_id.clone(),
                moderated_stage: effect.moderated_stage,
                interaction_id: effect.interaction_id.clone(),
                x_id: effect.x_id.clone(),
                mediator_id: effect.mediator_id.clone(),
                y_id: effect.y_id.clone(),
                moderator_id: effect.moderator_id.clone(),
                ordered_relation_ids: effect.ordered_relation_ids.clone(),
                probe_value_index: effect.probe_value_index,
                moderator_value_bits_hex: format!("{:016x}", effect.moderator_value.to_bits()),
            }
        }))
        .chain(results.moderated_mediation_indices.iter().map(|effect| {
            CanonicalGeneralSemEffectIdentityV1::ModeratedMediationIndex {
                effect_id: effect.effect_id.clone(),
                target_id: effect.target_id.clone(),
                estimand_id: effect.estimand_id.clone(),
                moderated_stage: effect.moderated_stage,
                interaction_id: effect.interaction_id.clone(),
                x_id: effect.x_id.clone(),
                mediator_id: effect.mediator_id.clone(),
                y_id: effect.y_id.clone(),
                moderator_id: effect.moderator_id.clone(),
                ordered_relation_ids: effect.ordered_relation_ids.clone(),
            }
        }))
        .collect::<Vec<_>>();
    identities.sort_by(|left, right| left.effect_id().cmp(right.effect_id()));
    identities
}

pub fn compiled_pls_effect_identities_v1(
    estimands: &[crate::CompiledPlsEffectEstimandV3],
) -> Vec<CanonicalGeneralSemEffectIdentityV1> {
    let mut identities = estimands
        .iter()
        .map(|estimand| match estimand {
            crate::CompiledPlsEffectEstimandV3::SpecificIndirect {
                estimand_id,
                path_identity,
                source_id,
                target_id,
                ordered_relation_ids,
            } => CanonicalGeneralSemEffectIdentityV1::SpecificIndirect {
                effect_id: path_identity.clone(),
                estimand_id: estimand_id.clone(),
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                ordered_relation_ids: ordered_relation_ids.clone(),
            },
            crate::CompiledPlsEffectEstimandV3::TotalIndirect {
                estimand_id,
                source_id,
                target_id,
                contributing_path_identities,
            } => CanonicalGeneralSemEffectIdentityV1::TotalIndirect {
                effect_id: estimand_id.clone(),
                estimand_id: estimand_id.clone(),
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                contributing_path_identities: contributing_path_identities.clone(),
            },
            crate::CompiledPlsEffectEstimandV3::TotalEffect {
                estimand_id,
                source_id,
                target_id,
                direct_relation_ids,
                contributing_indirect_path_identities,
            } => CanonicalGeneralSemEffectIdentityV1::TotalEffect {
                effect_id: estimand_id.clone(),
                estimand_id: estimand_id.clone(),
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                direct_relation_ids: direct_relation_ids.clone(),
                contributing_path_identities: contributing_indirect_path_identities.clone(),
            },
        })
        .collect::<Vec<_>>();
    identities.sort_by(|left, right| left.effect_id().cmp(right.effect_id()));
    identities
}

pub fn general_sem_effect_identity_set_sha256_v1(
    identities: &[CanonicalGeneralSemEffectIdentityV1],
) -> String {
    crate::sha256_serialized(&identities)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalConditionalProbeValuesResultV1 {
    DataDerivedMeanPlusMinusOneSd { mean: f64, standard_deviation: f64 },
    Explicit { values: Vec<f64> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalConditionalEffectProbeResultV1 {
    pub probe_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub moderator_id: String,
    pub values: CanonicalConditionalProbeValuesResultV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalInteractionHierarchyPolicyV1 {
    Strong,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalInteractionConstructionMethodV1 {
    TwoStage,
}

/// Point-estimate authority for one compiled two-stage interaction. The
/// coefficient fitted to the standardized product and the rescaled scientific
/// gamma are both retained with the exact product-scale receipt; consumers
/// never have to infer one scale from an unlabeled coefficient.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInteractionEffectResultV1 {
    pub effect_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub interaction_id: String,
    pub focal_relation_id: String,
    pub interaction_effect_relation_id: String,
    pub interaction_effect_parameter_id: String,
    pub focal_predictor_id: String,
    pub moderator_id: String,
    pub outcome_id: String,
    pub generated_product_column_id: String,
    pub stage_one_model_scientific_sha256: String,
    pub method_version: String,
    pub construction_method: CanonicalInteractionConstructionMethodV1,
    pub product_scale_version: String,
    pub hierarchy_policy: CanonicalInteractionHierarchyPolicyV1,
    pub hierarchy_policy_version: String,
    pub conditioning_policy_version: String,
    pub observation_count: u32,
    pub unstandardized_product_mean: f64,
    pub unstandardized_product_sample_standard_deviation: f64,
    pub standardized_product_coefficient: CanonicalGeneralSemEstimateV1,
    pub scientific_rescaled_gamma: CanonicalGeneralSemEstimateV1,
}

/// Point and optional bootstrap authority for the single bounded ordered
/// X-by-W-by-Z interaction. Lower-order interactions remain in the existing
/// two-way collection; this row owns only the scientific three-way delta.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalThreeWayInteractionEffectResultV1 {
    pub effect_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub interaction_id: String,
    pub focal_relation_id: String,
    pub interaction_effect_relation_id: String,
    pub interaction_effect_parameter_id: String,
    pub operand_ids: [String; 3],
    pub outcome_id: String,
    pub generated_product_column_id: String,
    pub stage_one_model_scientific_sha256: String,
    pub method_version: String,
    pub product_scale_version: String,
    pub hierarchy_policy: CanonicalInteractionHierarchyPolicyV1,
    pub hierarchy_policy_version: String,
    pub observation_count: u32,
    pub unstandardized_product_mean: f64,
    pub unstandardized_product_sample_standard_deviation: f64,
    pub standardized_product_coefficient: CanonicalGeneralSemEstimateV1,
    pub scientific_rescaled_delta: CanonicalGeneralSemEstimateV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalThreeWayModeratorProbeKindV1 {
    ContinuousStandardized,
    BinaryZeroOne,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalThreeWayConditionalInteractionEffectResultV1 {
    pub effect_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub interaction_id: String,
    pub focal_relation_id: String,
    pub first_moderator_id: String,
    pub second_moderator_id: String,
    pub second_moderator_probe_kind: CanonicalThreeWayModeratorProbeKindV1,
    pub second_moderator_probe_index: u32,
    pub second_moderator_value: f64,
    pub value: CanonicalGeneralSemEstimateV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalThreeWaySimpleSlopeResultV1 {
    pub effect_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub interaction_id: String,
    pub focal_relation_id: String,
    pub first_moderator_id: String,
    pub second_moderator_id: String,
    pub first_moderator_probe_kind: CanonicalThreeWayModeratorProbeKindV1,
    pub first_probe_index: u32,
    pub first_moderator_value: f64,
    pub second_moderator_probe_kind: CanonicalThreeWayModeratorProbeKindV1,
    pub second_probe_index: u32,
    pub second_moderator_value: f64,
    pub value: CanonicalGeneralSemEstimateV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalThreeWayModerationBootstrapReceiptV1 {
    pub capability_cell: CapabilityCellReferenceV2,
    pub capability_dependencies: Vec<CapabilityCellReferenceV2>,
    pub method_version: String,
    pub point_method_version: String,
    pub resampling_operation_version: String,
    pub resampling_stream_version: String,
    pub quantile_method_version: String,
    pub standard_error_method_version: String,
    pub summation_method_version: String,
    pub p_value_method_version: String,
    pub failure_policy_version: String,
    pub sign_alignment_method_version: String,
    pub product_scale_version: String,
    pub probe_policy_version: String,
    pub compiled_plan_sha256: String,
    pub general_sem_config_sha256: String,
    pub model_scientific_sha256: String,
    pub stage_one_model_scientific_sha256: String,
    pub source_dataset_fingerprint: String,
    pub complete_case_frame_sha256: String,
    pub usable_replicate_indices_sha256: String,
    pub target_identity_set_sha256: String,
    pub target_ids: Vec<String>,
    pub interval: CanonicalGeneralSemBootstrapIntervalV1,
    pub tail: CanonicalGeneralSemInferenceTailV1,
    pub confidence_level: f64,
    pub resamples_requested: u32,
    pub resamples_usable: u32,
    pub minimum_usable_resamples: u32,
    pub seed: String,
    pub workers: u32,
    pub complete_model_reestimated_per_replicate: bool,
    pub shared_stage_one_reestimated_per_replicate: bool,
    pub score_vectors_sign_aligned_before_products: bool,
    pub all_lower_order_and_three_way_products_recomputed_per_replicate: bool,
    pub joint_stage_two_reestimated_per_replicate: bool,
    pub complete_joint_point_contract_validated_per_replicate: bool,
    pub all_three_way_targets_share_one_replicate_ledger: bool,
    pub failed_replicates: Vec<CanonicalGeneralSemFailedReplicateV1>,
}

/// Scientific role of an ordinary coefficient in the final simultaneous
/// moderation equation. Interaction-product coefficients remain represented
/// by `CanonicalInteractionEffectResultV1`; this ledger covers the authored
/// structural and control relations that were re-estimated alongside them.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalStructuralRelationRoleV1 {
    Structural,
    Control,
}

/// The ledger is deliberately stage-qualified so a stage-one score-model
/// coefficient can never be presented as the final moderation estimate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalStructuralEstimateStageV1 {
    JointStageTwo,
}

/// One final ordinary coefficient from the simultaneous joint stage-two solve.
/// Stable relation and parameter identities bind the numeric value back to the
/// authored SemModelV4 and deterministically recompiled PLS plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalJointStageStructuralCoefficientResultV1 {
    pub relation_id: String,
    pub parameter_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub source_id: String,
    pub target_id: String,
    pub role: CanonicalStructuralRelationRoleV1,
    pub estimate: CanonicalGeneralSemEstimateV1,
    pub stage: CanonicalStructuralEstimateStageV1,
    pub method_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalConditionalEffectResultV1 {
    pub effect_id: String,
    pub estimand_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub interaction_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_effect_id: Option<String>,
    pub focal_relation_id: String,
    pub probe_id: String,
    pub moderator_id: String,
    pub probe_value_index: u32,
    pub moderator_value: f64,
    pub value: CanonicalGeneralSemEstimateV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalModeratedMediationStageV1 {
    FirstStage,
    SecondStage,
}

/// One of the three locked standardized-moderator conditional indirect
/// effects owned by the combined moderated-mediation cell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalConditionalIndirectEffectResultV1 {
    pub effect_id: String,
    pub target_id: String,
    pub estimand_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub moderated_stage: CanonicalModeratedMediationStageV1,
    pub interaction_id: String,
    pub x_id: String,
    pub mediator_id: String,
    pub y_id: String,
    pub moderator_id: String,
    pub ordered_relation_ids: Vec<String>,
    pub probe_value_index: u32,
    pub moderator_value: f64,
    pub value: CanonicalGeneralSemEstimateV1,
}

/// The Hayes-style index for the same exact target: scientific gamma times
/// the unmoderated-stage coefficient.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalModeratedMediationIndexResultV1 {
    pub effect_id: String,
    pub target_id: String,
    pub estimand_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub moderated_stage: CanonicalModeratedMediationStageV1,
    pub interaction_id: String,
    pub x_id: String,
    pub mediator_id: String,
    pub y_id: String,
    pub moderator_id: String,
    pub ordered_relation_ids: Vec<String>,
    pub value: CanonicalGeneralSemEstimateV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInteractionPlotPointV1 {
    pub focal_value: f64,
    pub predicted_value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInteractionPlotSeriesV1 {
    pub series_id: String,
    pub probe_id: String,
    pub probe_value_index: u32,
    pub moderator_value: f64,
    pub points: Vec<CanonicalInteractionPlotPointV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInteractionPlotResultV1 {
    pub plot_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub interaction_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_effect_id: Option<String>,
    pub focal_relation_id: String,
    pub focal_predictor_id: String,
    pub moderator_id: String,
    pub outcome_id: String,
    pub series: Vec<CanonicalInteractionPlotSeriesV1>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalHocStageKindV1 {
    LowerOrderScoreEstimation,
    HigherOrderEstimation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalHocRelationKindV1 {
    ComponentLoading,
    ComponentWeight,
    AuthoredStructural,
    AuthoredControl,
    TechnicalStructural,
    ExtendedIndirectEffect,
    ExtendedTotalEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHocGeneratedVariableMappingV1 {
    pub component_id: String,
    pub generated_score_variable_id: String,
    pub generated_component_relation_id: String,
    pub generated_component_parameter_id: String,
    pub component_relation_source_id: String,
    pub component_relation_target_id: String,
    pub relation_interpretation: crate::CompiledPlsHocComponentRelationInterpretationV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHocGeneratedScoreColumnReceiptV1 {
    pub component_id: String,
    pub generated_score_variable_id: String,
    pub observation_count: u32,
    pub values_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHocGeneratedScoreDatasetReceiptV1 {
    pub receipt_version: String,
    pub source_dataset_fingerprint: String,
    pub complete_case_row_count: u32,
    pub omitted_row_count: u32,
    pub complete_case_rows_sha256: String,
    pub generated_score_columns: Vec<CanonicalHocGeneratedScoreColumnReceiptV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHocPointStageReceiptV1 {
    pub receipt_version: String,
    pub stage_number: u32,
    pub role: crate::CompiledPlsHocStageRoleV1,
    pub projection_identity_sha256: String,
    pub model_scientific_sha256: String,
    pub compiled_plan_sha256: String,
    pub dataset_fingerprint: String,
    pub used_observations: u32,
    pub omitted_observations: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_score_dataset: Option<CanonicalHocGeneratedScoreDatasetReceiptV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHocRelationEstimateV1 {
    pub relation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter_id: Option<String>,
    pub source_id: String,
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<CanonicalHocRelationKindV1>,
    pub value: CanonicalGeneralSemEstimateV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collinearity_vif: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHocStageResultV1 {
    pub stage_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub higher_order_construct_id: String,
    pub stage_number: u32,
    pub kind: CanonicalHocStageKindV1,
    pub input_construct_ids: Vec<String>,
    pub output_variable_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approach: Option<crate::HigherOrderConstructionApproachV4>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurement_type: Option<crate::HigherOrderMeasurementTypeV4>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generated_variable_mappings: Vec<CanonicalHocGeneratedVariableMappingV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<CanonicalHocPointStageReceiptV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relation_estimates: Vec<CanonicalHocRelationEstimateV1>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalHocBootstrapTargetKindV1 {
    ComponentLoading,
    ComponentWeight,
    HocStructuralPath,
    ExtendedTotalEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHocBootstrapTargetIdentityV1 {
    pub kind: CanonicalHocBootstrapTargetKindV1,
    pub target_version: String,
    pub target_id: String,
    pub relation_id: String,
    pub parameter_id: String,
    pub source_id: String,
    pub target_variable_id: String,
    pub point_method_version: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalHocBootstrapFailureReasonV1 {
    InsufficientObservations,
    ConstantIndicator,
    StageOneRankDeficient,
    IsolatedConstruct,
    StageOneNonconvergence,
    IndeterminateScoreSign,
    ConstantComponentScore,
    StageTwoRankDeficient,
    StageTwoNonconvergence,
    ComponentCollinearity,
    NumericalFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHocBootstrapFailedReplicateV1 {
    pub replicate_index: u32,
    pub reason_code: CanonicalHocBootstrapFailureReasonV1,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHocBootstrapReceiptV1 {
    pub schema_version: u32,
    pub capability_cell: CapabilityCellReferenceV2,
    pub method_version: String,
    pub point_method_version: String,
    pub resampling_operation_version: String,
    pub resampling_stream_version: String,
    pub quantile_method_version: String,
    pub standard_error_method_version: String,
    pub summation_method_version: String,
    pub p_value_method_version: String,
    pub failure_policy_version: String,
    pub sign_alignment_method_version: String,
    pub target_version: String,
    pub general_sem_config_sha256: String,
    pub compiled_plan_sha256: String,
    pub hoc_stage_plan_sha256: String,
    pub model_scientific_sha256: String,
    pub stage_one_model_scientific_sha256: String,
    pub stage_two_model_scientific_sha256: String,
    pub source_dataset_fingerprint: String,
    pub complete_case_frame_sha256: String,
    pub usable_replicate_indices_sha256: String,
    pub target_identity_set_sha256: String,
    pub target_ids: Vec<String>,
    pub target_identities: Vec<CanonicalHocBootstrapTargetIdentityV1>,
    pub interval: CanonicalGeneralSemBootstrapIntervalV1,
    pub tail: CanonicalGeneralSemInferenceTailV1,
    pub confidence_level: f64,
    pub resamples_requested: u32,
    pub resamples_usable: u32,
    pub minimum_usable_resamples: u32,
    pub seed: String,
    pub workers: u32,
    pub complete_model_reestimated_per_replicate: bool,
    pub stage_one_reestimated_per_replicate: bool,
    pub generated_component_values_recalculated_per_replicate: bool,
    pub stage_one_scores_sign_aligned_per_replicate: bool,
    pub stage_two_reestimated_per_replicate: bool,
    pub stage_two_scores_sign_aligned_per_replicate: bool,
    pub complete_point_contract_validated_per_replicate: bool,
    pub failed_replicates: Vec<CanonicalHocBootstrapFailedReplicateV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalGeneralSemIntervalV1 {
    pub confidence_level: f64,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalCbsemParameterRoleV1 {
    Loading,
    Regression,
    Covariance,
    Variance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalCbsemEndpointV1 {
    Variable { variable_id: String },
    Residual { variable_id: String },
    Disturbance { variable_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalCbsemParameterTargetV1 {
    Loading {
        factor_id: String,
        indicator_id: String,
    },
    Regression {
        source_id: String,
        target_id: String,
    },
    Covariance {
        left: CanonicalCbsemEndpointV1,
        right: CanonicalCbsemEndpointV1,
    },
    Variance {
        endpoint: CanonicalCbsemEndpointV1,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalCbsemParameterStateV1 {
    Fixed {
        value: f64,
    },
    Free {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        equality_label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lower: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        upper: Option<f64>,
    },
}

/// Typed point-estimate row bound to one authoritative V3 parameter-table row.
/// Location/threshold/derived families are absent because the bounded v1
/// compiler rejects them before estimation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalCbsemParameterResultV1 {
    pub parameter_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub role: CanonicalCbsemParameterRoleV1,
    pub target: CanonicalCbsemParameterTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_id: Option<String>,
    pub state: CanonicalCbsemParameterStateV1,
    pub estimate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard_error: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standardized_estimate: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalCbsemBootstrapFailedReplicateReasonV1 {
    InsufficientObservations,
    NonpositiveDefiniteSampleCovariance,
    Nonconvergence,
    NonfiniteEstimate,
    ParameterInventoryMismatch,
    NumericalFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalCbsemBootstrapFailedReplicateV1 {
    pub replicate_index: u32,
    pub reason_code: CanonicalCbsemBootstrapFailedReplicateReasonV1,
    pub message: String,
}

/// Exact receipt for the bounded recursive-SEM percentile case bootstrap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalCbsemBootstrapReceiptV1 {
    pub capability_cell: CapabilityCellReferenceV2,
    pub method_version: String,
    pub resampling_operation_version: String,
    pub quantile_method_version: String,
    pub compiled_plan_sha256: String,
    pub base_plan_sha256: String,
    pub parameter_inventory_sha256: String,
    pub model_scientific_sha256: String,
    pub general_sem_config_sha256: String,
    pub recipe_analytical_sha256: String,
    pub source_dataset_fingerprint: String,
    pub complete_case_frame_sha256: String,
    pub usable_replicate_indices_sha256: String,
    pub confidence_level: f64,
    pub resamples_requested: u32,
    pub resamples_usable: u32,
    pub minimum_usable_resamples: u32,
    /// Canonical decimal u64 wire form; exact in JavaScript runtimes.
    pub seed: String,
    pub workers: u32,
    pub complete_model_reestimated_per_replicate: bool,
    pub failed_replicates: Vec<CanonicalCbsemBootstrapFailedReplicateV1>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalCbsemBootstrapUnavailableReasonV1 {
    InsufficientUsableReplicates,
    ParameterNotEligible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalCbsemBootstrapInferenceOutcomeV1 {
    Available {
        value: CanonicalGeneralSemEstimateV1,
    },
    Unavailable {
        reason: CanonicalCbsemBootstrapUnavailableReasonV1,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalCbsemBootstrapParameterInferenceV1 {
    pub parameter_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub point_estimate: f64,
    pub outcome: CanonicalCbsemBootstrapInferenceOutcomeV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalCbsemFitResultV1 {
    pub fit_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub chi_square: f64,
    pub degrees_of_freedom: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chi_square_p_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rmsea: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rmsea_interval: Option<CanonicalGeneralSemIntervalV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfi: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tli: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub srmr: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aic: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bic: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalIdentificationScopeV1 {
    Model,
    Variable,
    Relation,
    Interaction,
    HigherOrderConstruct,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalIdentificationStatusV1 {
    Identified,
    Provisional,
    Underidentified,
    LocallyUnderidentified,
    BoundaryCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalIdentificationDiagnosticV1 {
    pub diagnostic_id: String,
    pub trace: CanonicalGeneralSemResultTraceV1,
    pub scope: CanonicalIdentificationScopeV1,
    pub subject_id: String,
    pub status: CanonicalIdentificationStatusV1,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degrees_of_freedom: Option<i64>,
}

/// Optional typed analytical extension for the General SEM roadmap. Every
/// collection is canonical stable-ID order; the extension never implies that
/// an estimator executed a section that is absent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalGeneralSemResultsV1 {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_receipt: Option<CanonicalGeneralSemInferenceReceiptV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub specific_indirect_effects: Vec<CanonicalSpecificIndirectEffectResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aggregate_effects: Vec<CanonicalAggregateEffectResultV1>,
    /// Final ordinary structural/control coefficients for exact joint-stage
    /// moderation cells. Omitted for historical documents and estimator cells
    /// that do not execute a joint derived-term stage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub joint_stage_structural_coefficients: Vec<CanonicalJointStageStructuralCoefficientResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interaction_effects: Vec<CanonicalInteractionEffectResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub three_way_interaction_effects: Vec<CanonicalThreeWayInteractionEffectResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub three_way_conditional_interaction_effects:
        Vec<CanonicalThreeWayConditionalInteractionEffectResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub three_way_simple_slopes: Vec<CanonicalThreeWaySimpleSlopeResultV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub three_way_moderation_bootstrap_receipt:
        Option<CanonicalThreeWayModerationBootstrapReceiptV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditional_effect_probes: Vec<CanonicalConditionalEffectProbeResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditional_effects: Vec<CanonicalConditionalEffectResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditional_indirect_effects: Vec<CanonicalConditionalIndirectEffectResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moderated_mediation_indices: Vec<CanonicalModeratedMediationIndexResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interaction_plots: Vec<CanonicalInteractionPlotResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub higher_order_stages: Vec<CanonicalHocStageResultV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub higher_order_inference_receipt: Option<CanonicalHocBootstrapReceiptV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cbsem_parameters: Vec<CanonicalCbsemParameterResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cbsem_fit: Vec<CanonicalCbsemFitResultV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identification_diagnostics: Vec<CanonicalIdentificationDiagnosticV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cbsem_bootstrap_receipt: Option<CanonicalCbsemBootstrapReceiptV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cbsem_bootstrap_inference: Vec<CanonicalCbsemBootstrapParameterInferenceV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultDocumentV2 {
    pub schema_version: u32,
    pub document_id: String,
    pub title: String,
    pub provenance: CanonicalResultProvenanceV2,
    /// Sorted, distinct option-cell set. `provenance.capability_cell` remains
    /// the primary capability for wire compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cells: Option<Vec<CapabilityCellReferenceV2>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general_sem_results: Option<CanonicalGeneralSemResultsV1>,
    pub sections: Vec<CanonicalResultSection>,
    pub tables: Vec<CanonicalResultTable>,
    pub charts: Vec<CanonicalResultChart>,
    pub notices: Vec<CanonicalResultNotice>,
    pub exclusions: Vec<CanonicalResultExclusion>,
    pub footnotes: Vec<CanonicalResultFootnote>,
    pub presentation: CanonicalResultPresentationV2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalResultValidation {
    pub passed: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalResultQualificationIneligibilityV2 {
    LegacyCapabilityAttributionMissing,
    UnqualifiedLabsCapability,
    InvalidDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultUseEligibilityV2 {
    pub readable: bool,
    pub comparison_eligible: bool,
    pub qualification_export_eligible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ineligibility: Option<CanonicalResultQualificationIneligibilityV2>,
}

fn is_stable_id(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    characters.all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '_' | '.' | ':' | '-')
    })
}

fn require_stable_id(errors: &mut Vec<String>, value: &str, context: &str) {
    if !is_stable_id(value) {
        errors.push(format!("{context} must be a stable lowercase identifier"));
    }
}

fn require_unique_ids<'a>(
    errors: &mut Vec<String>,
    ids: impl IntoIterator<Item = &'a str>,
    context: &str,
) {
    let ids: Vec<&str> = ids.into_iter().collect();
    let mut seen = HashSet::new();
    let mut duplicates = BTreeSet::new();
    for id in &ids {
        if !seen.insert(*id) {
            duplicates.insert(*id);
        }
    }
    if !duplicates.is_empty() {
        errors.push(format!(
            "{context} contains duplicate IDs: {}",
            duplicates.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    for id in ids {
        require_stable_id(errors, id, &format!("{context} ID {id:?}"));
    }
}

fn validate_capability_reference(
    errors: &mut Vec<String>,
    reference: &CapabilityCellReferenceV2,
    context: &str,
) {
    if reference.registry_schema_version != 2 {
        errors.push(format!("{context}.registry_schema_version must equal 2"));
    }
    require_stable_id(
        errors,
        &reference.capability_id,
        &format!("{context}.capability_id"),
    );
    require_stable_id(errors, &reference.cell_id, &format!("{context}.cell_id"));
    if reference.capability_version.trim().is_empty() {
        errors.push(format!("{context}.capability_version must be nonempty"));
    }
}

fn validate_capability_set(
    errors: &mut Vec<String>,
    references: &[CapabilityCellReferenceV2],
    context: &str,
) -> Vec<String> {
    if references.is_empty() {
        errors.push(format!("{context} must not be empty"));
    }
    let identities = references
        .iter()
        .enumerate()
        .map(|(index, reference)| {
            validate_capability_reference(errors, reference, &format!("{context}[{index}]"));
            capability_cell_reference_identity_v2(reference)
        })
        .collect::<Vec<_>>();
    let mut seen = HashSet::new();
    let duplicates = identities
        .iter()
        .filter(|identity| !seen.insert(identity.as_str()))
        .cloned()
        .collect::<BTreeSet<_>>();
    if !duplicates.is_empty() {
        errors.push(format!(
            "{context} contains duplicate references: {}",
            duplicates.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    let mut sorted = identities.clone();
    sorted.sort();
    if identities != sorted {
        errors.push(format!(
            "{context} must be ordered by exact option-cell identity"
        ));
    }
    identities
}

fn require_canonical_stable_ids<'a>(
    errors: &mut Vec<String>,
    ids: impl IntoIterator<Item = &'a str>,
    context: &str,
) {
    let ids = ids.into_iter().collect::<Vec<_>>();
    require_unique_ids(errors, ids.iter().copied(), context);
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    if ids != sorted {
        errors.push(format!(
            "{context} must be ordered by exact stable identifier"
        ));
    }
}

fn validate_general_sem_trace(
    errors: &mut Vec<String>,
    trace: &CanonicalGeneralSemResultTraceV1,
    document_model_id: &str,
    document_capability_ids: Option<&HashSet<String>>,
    context: &str,
) {
    require_stable_id(errors, &trace.model_id, &format!("{context}.model_id"));
    if trace.model_id != document_model_id {
        errors.push(format!("{context}.model_id must equal provenance.model_id"));
    }
    validate_capability_reference(
        errors,
        &trace.capability_cell,
        &format!("{context}.capability_cell"),
    );
    let identity = capability_cell_reference_identity_v2(&trace.capability_cell);
    match document_capability_ids {
        Some(identities) if !identities.contains(&identity) => errors.push(format!(
            "{context}.capability_cell references undeclared option cell {identity}"
        )),
        None => errors.push(format!(
            "{context}.capability_cell requires document capability_cells"
        )),
        _ => {}
    }
}

fn validate_general_sem_bounds(
    errors: &mut Vec<String>,
    lower: Option<f64>,
    upper: Option<f64>,
    context: &str,
) {
    if lower.is_some_and(|value| !value.is_finite()) {
        errors.push(format!("{context}.lower must be finite"));
    }
    if upper.is_some_and(|value| !value.is_finite()) {
        errors.push(format!("{context}.upper must be finite"));
    }
    if matches!((lower, upper), (Some(lower), Some(upper)) if lower > upper) {
        errors.push(format!("{context}.lower must not exceed upper"));
    }
}

fn validate_general_sem_estimate(
    errors: &mut Vec<String>,
    value: &CanonicalGeneralSemEstimateV1,
    context: &str,
) {
    if !value.estimate.is_finite() {
        errors.push(format!("{context}.estimate must be finite"));
    }
    if value
        .bootstrap_mean
        .is_some_and(|bootstrap_mean| !bootstrap_mean.is_finite())
    {
        errors.push(format!("{context}.bootstrap_mean must be finite"));
    }
    if value
        .bootstrap_bias
        .is_some_and(|bootstrap_bias| !bootstrap_bias.is_finite())
    {
        errors.push(format!("{context}.bootstrap_bias must be finite"));
    }
    if value
        .standard_error
        .is_some_and(|standard_error| !standard_error.is_finite() || standard_error < 0.0)
    {
        errors.push(format!(
            "{context}.standard_error must be finite and nonnegative"
        ));
    }
    if value
        .p_value
        .is_some_and(|p_value| !p_value.is_finite() || !(0.0..=1.0).contains(&p_value))
    {
        errors.push(format!(
            "{context}.p_value must be finite and between 0 and 1"
        ));
    }
    validate_general_sem_bounds(errors, value.lower, value.upper, context);
    let inference_field_count = [
        value.bootstrap_mean.is_some(),
        value.bootstrap_bias.is_some(),
        value.standard_error.is_some(),
        value.lower.is_some(),
        value.upper.is_some(),
        value.p_value.is_some(),
        value.bootstrap_usable_replicates.is_some(),
        value.bootstrap_two_sided_exceedances.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if inference_field_count != 0 && inference_field_count != 8 {
        errors.push(format!(
            "{context} bootstrap inference fields must be either all absent or all present"
        ));
    }
    if let (Some(mean), Some(bias)) = (value.bootstrap_mean, value.bootstrap_bias)
        && !approximately_equal(mean - value.estimate, bias)
    {
        errors.push(format!(
            "{context}.bootstrap_bias must equal bootstrap_mean minus estimate"
        ));
    }
}

fn general_sem_estimate_has_inference(value: &CanonicalGeneralSemEstimateV1) -> bool {
    value.bootstrap_mean.is_some()
        || value.bootstrap_bias.is_some()
        || value.standard_error.is_some()
        || value.lower.is_some()
        || value.upper.is_some()
        || value.p_value.is_some()
        || value.bootstrap_usable_replicates.is_some()
        || value.bootstrap_two_sided_exceedances.is_some()
}

fn general_sem_estimate_has_complete_inference(value: &CanonicalGeneralSemEstimateV1) -> bool {
    value.bootstrap_mean.is_some()
        && value.bootstrap_bias.is_some()
        && value.standard_error.is_some()
        && value.lower.is_some()
        && value.upper.is_some()
        && value.p_value.is_some()
        && value.bootstrap_usable_replicates.is_some()
        && value.bootstrap_two_sided_exceedances.is_some()
}

fn approximately_equal(left: f64, right: f64) -> bool {
    left == right
        || (left - right).abs() <= f64::EPSILON * 8.0 * left.abs().max(right.abs()).max(1.0)
}

fn conditional_probe_value(
    probe: &CanonicalConditionalEffectProbeResultV1,
    index: u32,
) -> Option<f64> {
    let index = usize::try_from(index).ok()?;
    match &probe.values {
        CanonicalConditionalProbeValuesResultV1::DataDerivedMeanPlusMinusOneSd {
            mean,
            standard_deviation,
        } => [
            *mean - *standard_deviation,
            *mean,
            *mean + *standard_deviation,
        ]
        .get(index)
        .copied(),
        CanonicalConditionalProbeValuesResultV1::Explicit { values } => values.get(index).copied(),
    }
}

fn is_frozen_standardized_three_point_probe(
    probe: &CanonicalConditionalEffectProbeResultV1,
) -> bool {
    match &probe.values {
        CanonicalConditionalProbeValuesResultV1::Explicit { values } => {
            values.len() == 3
                && approximately_equal(values[0], -1.0)
                && approximately_equal(values[1], 0.0)
                && approximately_equal(values[2], 1.0)
        }
        CanonicalConditionalProbeValuesResultV1::DataDerivedMeanPlusMinusOneSd { .. } => false,
    }
}

fn validate_general_sem_inference_receipt_v1(
    errors: &mut Vec<String>,
    results: &CanonicalGeneralSemResultsV1,
    provenance: &CanonicalResultProvenanceV2,
    document_capability_ids: Option<&HashSet<String>>,
) {
    let context = "general_sem_results.inference_receipt";
    let mediation_effect_values = results
        .specific_indirect_effects
        .iter()
        .map(|effect| (effect.effect_id.as_str(), &effect.value, &effect.trace))
        .chain(
            results
                .aggregate_effects
                .iter()
                .map(|effect| (effect.effect_id.as_str(), &effect.value, &effect.trace)),
        )
        .collect::<Vec<_>>();
    let moderation_effect_values = results
        .interaction_effects
        .iter()
        .map(|effect| {
            (
                effect.effect_id.as_str(),
                &effect.scientific_rescaled_gamma,
                &effect.trace,
            )
        })
        .collect::<Vec<_>>();
    let moderated_mediation_derived_effect_values = results
        .conditional_indirect_effects
        .iter()
        .map(|effect| (effect.effect_id.as_str(), &effect.value, &effect.trace))
        .chain(
            results
                .moderated_mediation_indices
                .iter()
                .map(|effect| (effect.effect_id.as_str(), &effect.value, &effect.trace)),
        )
        .collect::<Vec<_>>();
    let moderated_mediation_derived_effect_ids = moderated_mediation_derived_effect_values
        .iter()
        .map(|(effect_id, _, _)| *effect_id)
        .collect::<BTreeSet<_>>();
    let moderated_mediation_effect_values = moderation_effect_values
        .iter()
        .copied()
        .chain(moderated_mediation_derived_effect_values.iter().copied())
        .collect::<Vec<_>>();
    let point_only_inference = results
        .joint_stage_structural_coefficients
        .iter()
        .any(|coefficient| general_sem_estimate_has_inference(&coefficient.estimate))
        || results.interaction_effects.iter().any(|effect| {
            general_sem_estimate_has_inference(&effect.standardized_product_coefficient)
        })
        || results
            .conditional_effects
            .iter()
            .any(|effect| general_sem_estimate_has_inference(&effect.value));
    let interaction_plot_interval_fields = results.interaction_plots.iter().any(|plot| {
        plot.series.iter().any(|series| {
            series
                .points
                .iter()
                .any(|point| point.lower.is_some() || point.upper.is_some())
        })
    });

    let Some(receipt) = &results.inference_receipt else {
        if mediation_effect_values
            .iter()
            .any(|(_, value, _)| general_sem_estimate_has_inference(value))
            || moderation_effect_values
                .iter()
                .any(|(_, value, _)| general_sem_estimate_has_inference(value))
            || moderated_mediation_derived_effect_values
                .iter()
                .any(|(_, value, _)| general_sem_estimate_has_inference(value))
            || point_only_inference
        {
            errors.push("general_sem_results inference fields require inference_receipt".into());
        }
        return;
    };

    validate_capability_reference(
        errors,
        &receipt.capability_cell,
        &format!("{context}.capability_cell"),
    );
    let single_mediation_bootstrap = receipt.capability_cell
        == crate::pls_general_single_mediation_bootstrap_capability_cell_v1();
    let mediation_bootstrap = receipt.capability_cell
        == general_sem_pls_bootstrap_capability_cell_v1()
        || single_mediation_bootstrap;
    let moderation_bootstrap = receipt.capability_cell
        == crate::pls_general_multiple_moderation_bootstrap_capability_cell_v1();
    let moderated_mediation_bootstrap = receipt.capability_cell
        == crate::pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1();
    if !mediation_bootstrap && !moderation_bootstrap && !moderated_mediation_bootstrap {
        errors.push(format!(
            "{context}.capability_cell must equal an exact General SEM mediation, moderation, or two-way moderated-mediation full-model bootstrap option cell"
        ));
    }
    let effect_values = if moderated_mediation_bootstrap {
        &moderated_mediation_effect_values
    } else if moderation_bootstrap {
        &moderation_effect_values
    } else {
        &mediation_effect_values
    };
    if moderation_bootstrap
        && (!results.specific_indirect_effects.is_empty() || !results.aggregate_effects.is_empty())
    {
        errors.push(format!(
            "{context} moderation bootstrap must not contain mediation effect rows"
        ));
    }
    if moderated_mediation_bootstrap
        && (!results.specific_indirect_effects.is_empty() || !results.aggregate_effects.is_empty())
    {
        errors.push(format!(
            "{context} moderated-mediation bootstrap must not contain ordinary mediation effect rows"
        ));
    }
    let capability_identity = capability_cell_reference_identity_v2(&receipt.capability_cell);
    match document_capability_ids {
        Some(identities) if !identities.contains(&capability_identity) => errors.push(format!(
            "{context}.capability_cell references undeclared option cell {capability_identity}"
        )),
        None => errors.push(format!(
            "{context}.capability_cell requires document capability_cells"
        )),
        _ => {}
    }
    if moderated_mediation_bootstrap {
        let dependency_identities = validate_capability_set(
            errors,
            &receipt.capability_dependencies,
            &format!("{context}.capability_dependencies"),
        );
        let mut expected_dependencies = vec![
            crate::RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            crate::pls_general_multiple_moderation_point_capability_cell_v1(),
        ];
        expected_dependencies.sort_by_key(capability_cell_reference_identity_v2);
        let expected_identities = expected_dependencies
            .iter()
            .map(capability_cell_reference_identity_v2)
            .collect::<Vec<_>>();
        if dependency_identities != expected_identities {
            errors.push(format!(
                "{context}.capability_dependencies must exactly declare the base PLS and moderation-point cells"
            ));
        }
        if let Some(document_identities) = document_capability_ids {
            for dependency_identity in dependency_identities {
                if !document_identities.contains(&dependency_identity) {
                    errors.push(format!(
                        "{context}.capability_dependencies references undeclared option cell {dependency_identity}"
                    ));
                }
            }
        }
    } else if !receipt.capability_dependencies.is_empty() {
        errors.push(format!(
            "{context}.capability_dependencies must be empty for single-owner v1 bootstrap receipts"
        ));
    }

    for (name, value) in [
        ("method_version", receipt.method_version.as_str()),
        (
            "resampling_operation_version",
            receipt.resampling_operation_version.as_str(),
        ),
        (
            "resampling_stream_version",
            receipt.resampling_stream_version.as_str(),
        ),
        (
            "quantile_method_version",
            receipt.quantile_method_version.as_str(),
        ),
        (
            "standard_error_method_version",
            receipt.standard_error_method_version.as_str(),
        ),
        (
            "summation_method_version",
            receipt.summation_method_version.as_str(),
        ),
        (
            "p_value_method_version",
            receipt.p_value_method_version.as_str(),
        ),
        (
            "failure_policy_version",
            receipt.failure_policy_version.as_str(),
        ),
    ] {
        require_stable_id(errors, value, &format!("{context}.{name}"));
    }
    let expected_method_version = if moderated_mediation_bootstrap {
        GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1
    } else if moderation_bootstrap {
        GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
    } else if single_mediation_bootstrap {
        GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_METHOD_VERSION_V1
    } else {
        GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1
    };
    let expected_operation_version = if moderated_mediation_bootstrap {
        GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
    } else if moderation_bootstrap {
        GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
    } else if single_mediation_bootstrap {
        GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
    } else {
        GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1
    };
    if receipt.method_version != expected_method_version {
        errors.push(format!(
            "{context}.method_version must equal {expected_method_version}"
        ));
    }
    if receipt.resampling_operation_version != expected_operation_version {
        errors.push(format!(
            "{context}.resampling_operation_version must equal {expected_operation_version}"
        ));
    }
    if receipt.resampling_stream_version != GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1 {
        errors.push(format!(
            "{context}.resampling_stream_version must equal {GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1}"
        ));
    }
    if receipt.quantile_method_version != GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1 {
        errors.push(format!(
            "{context}.quantile_method_version must equal {GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1}"
        ));
    }
    if receipt.standard_error_method_version != GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1
    {
        errors.push(format!(
            "{context}.standard_error_method_version must equal {GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1}"
        ));
    }
    if receipt.summation_method_version != GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1 {
        errors.push(format!(
            "{context}.summation_method_version must equal {GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1}"
        ));
    }
    if receipt.p_value_method_version
        != GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1
    {
        errors.push(format!(
            "{context}.p_value_method_version must equal {GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1}"
        ));
    }
    if receipt.failure_policy_version != GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1 {
        errors.push(format!(
            "{context}.failure_policy_version must equal {GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1}"
        ));
    }
    for (name, value) in [
        (
            "compilation_artifact_identity_sha256",
            receipt.compilation_artifact_identity_sha256.as_str(),
        ),
        (
            "compiled_plan_sha256",
            receipt.compiled_plan_sha256.as_str(),
        ),
        (
            "general_sem_config_sha256",
            receipt.general_sem_config_sha256.as_str(),
        ),
        (
            "recipe_analytical_sha256",
            receipt.recipe_analytical_sha256.as_str(),
        ),
        (
            "model_scientific_sha256",
            receipt.model_scientific_sha256.as_str(),
        ),
        (
            "complete_case_frame_sha256",
            receipt.complete_case_frame_sha256.as_str(),
        ),
        (
            "usable_replicate_indices_sha256",
            receipt.usable_replicate_indices_sha256.as_str(),
        ),
        (
            "effect_identity_set_sha256",
            receipt.effect_identity_set_sha256.as_str(),
        ),
    ] {
        if !is_lowercase_sha256(value) {
            errors.push(format!("{context}.{name} must be a lowercase SHA-256"));
        }
    }
    if !is_dataset_fingerprint_v1(&receipt.source_dataset_fingerprint) {
        errors.push(format!(
            "{context}.source_dataset_fingerprint must be a bare lowercase SHA-256 or v2:<lowercase SHA-256>"
        ));
    }
    require_canonical_stable_ids(
        errors,
        receipt.effect_ids.iter().map(String::as_str),
        &format!("{context}.effect_ids"),
    );
    if receipt.effect_ids.is_empty() {
        errors.push(format!("{context}.effect_ids must not be empty"));
    }
    let mut expected_effect_ids = effect_values
        .iter()
        .map(|(effect_id, _, _)| (*effect_id).to_string())
        .collect::<Vec<_>>();
    expected_effect_ids.sort();
    if receipt.effect_ids != expected_effect_ids {
        errors.push(if moderated_mediation_bootstrap {
            format!(
                "{context}.effect_ids must exactly cover one scientific gamma, three conditional indirect effects, and one moderated-mediation index"
            )
        } else if moderation_bootstrap {
            format!(
                "{context}.effect_ids must exactly cover scientific rescaled gamma interaction rows"
            )
        } else {
            format!("{context}.effect_ids must exactly cover specific and aggregate effect rows")
        });
    }
    let effect_identities = canonical_general_sem_effect_identities_v1(results)
        .into_iter()
        .filter(|identity| match identity {
            CanonicalGeneralSemEffectIdentityV1::InteractionScientificRescaledGamma { .. } => {
                moderation_bootstrap || moderated_mediation_bootstrap
            }
            CanonicalGeneralSemEffectIdentityV1::ConditionalIndirect { .. }
            | CanonicalGeneralSemEffectIdentityV1::ModeratedMediationIndex { .. } => {
                moderated_mediation_bootstrap
            }
            CanonicalGeneralSemEffectIdentityV1::SpecificIndirect { .. }
            | CanonicalGeneralSemEffectIdentityV1::TotalIndirect { .. }
            | CanonicalGeneralSemEffectIdentityV1::TotalEffect { .. } => {
                !moderation_bootstrap && !moderated_mediation_bootstrap
            }
        })
        .collect::<Vec<_>>();
    if receipt.effect_identity_set_sha256
        != general_sem_effect_identity_set_sha256_v1(&effect_identities)
    {
        errors.push(format!(
            "{context}.effect_identity_set_sha256 does not match the typed effect identity set"
        ));
    }
    if receipt.model_scientific_sha256 != provenance.model_digest {
        errors.push(format!(
            "{context}.model_scientific_sha256 must equal provenance.model_digest"
        ));
    }
    if receipt.source_dataset_fingerprint != provenance.dataset_fingerprint {
        errors.push(format!(
            "{context}.source_dataset_fingerprint must equal provenance.dataset_fingerprint"
        ));
    }
    if receipt.recipe_analytical_sha256 != provenance.recipe_digest {
        errors.push(format!(
            "{context}.recipe_analytical_sha256 must equal provenance.recipe_digest"
        ));
    }
    if !receipt.confidence_level.is_finite()
        || receipt.confidence_level <= 0.0
        || receipt.confidence_level >= 1.0
    {
        errors.push(format!(
            "{context}.confidence_level must be finite and strictly between 0 and 1"
        ));
    }
    if receipt.interval != CanonicalGeneralSemBootstrapIntervalV1::PercentileType7 {
        errors.push(format!(
            "{context}.interval must equal percentile_type7 for the v1 executor"
        ));
    }
    if receipt.tail != CanonicalGeneralSemInferenceTailV1::TwoSided {
        errors.push(format!(
            "{context}.tail must equal two_sided for the v1 executor"
        ));
    }
    if !(2..=10_000).contains(&receipt.resamples_requested) {
        errors.push(format!(
            "{context}.resamples_requested must be between 2 and 10000"
        ));
    }
    let expected_minimum = ((f64::from(receipt.resamples_requested) * 0.9).ceil() as u32).max(2);
    if receipt.minimum_usable_resamples != expected_minimum {
        errors.push(format!(
            "{context}.minimum_usable_resamples must equal the 90 percent usable gate"
        ));
    }
    if receipt.resamples_usable < receipt.minimum_usable_resamples
        || receipt.resamples_usable > receipt.resamples_requested
    {
        errors.push(format!(
            "{context}.resamples_usable must satisfy the declared usable gate"
        ));
    }
    if receipt.resamples_usable as usize + receipt.failed_replicates.len()
        != receipt.resamples_requested as usize
    {
        errors.push(format!(
            "{context} requested count must equal usable plus failed replicates"
        ));
    }
    if !(1..=64).contains(&receipt.workers) {
        errors.push(format!("{context}.workers must be between 1 and 64"));
    }
    match receipt.seed.parse::<u64>() {
        Ok(seed) if seed.to_string() == receipt.seed => {
            if provenance.seed.and_then(|value| u64::try_from(value).ok()) != Some(seed) {
                errors.push(format!("{context}.seed must equal provenance.seed"));
            }
        }
        _ => errors.push(format!("{context}.seed must be a canonical decimal u64")),
    }
    if i64::from(receipt.workers) != provenance.workers {
        errors.push(format!("{context}.workers must equal provenance.workers"));
    }
    if !receipt.complete_model_reestimated_per_replicate {
        errors.push(format!(
            "{context}.complete_model_reestimated_per_replicate must be true"
        ));
    }
    let mut previous_failure_index = None;
    let mut failed_indices = BTreeSet::new();
    for (index, failure) in receipt.failed_replicates.iter().enumerate() {
        let failure_context = format!("{context}.failed_replicates[{index}]");
        if failure.replicate_index >= receipt.resamples_requested {
            errors.push(format!(
                "{failure_context}.replicate_index is outside the requested plan"
            ));
        }
        failed_indices.insert(failure.replicate_index);
        if previous_failure_index.is_some_and(|previous| previous >= failure.replicate_index) {
            errors.push(format!(
                "{context}.failed_replicates must be strictly ordered by replicate_index"
            ));
        }
        previous_failure_index = Some(failure.replicate_index);
        if failure.message.trim().is_empty() {
            errors.push(format!("{failure_context}.message must be nonempty"));
        }
    }
    let usable_replicate_indices = (0..receipt.resamples_requested)
        .filter(|replicate_index| !failed_indices.contains(replicate_index))
        .collect::<Vec<_>>();
    if usable_replicate_indices.len() != receipt.resamples_usable as usize {
        errors.push(format!(
            "{context}.resamples_usable contradicts the failure ledger"
        ));
    }
    if receipt.usable_replicate_indices_sha256
        != crate::sha256_serialized(&usable_replicate_indices)
    {
        errors.push(format!(
            "{context}.usable_replicate_indices_sha256 does not match the failure ledger"
        ));
    }
    if effect_values
        .iter()
        .any(|(_, value, _)| !general_sem_estimate_has_complete_inference(value))
    {
        errors.push(if moderation_bootstrap {
            format!(
                "{context} requires complete inference fields for every scientific rescaled gamma interaction effect"
            )
        } else {
            format!("{context} requires complete inference fields for every covered effect")
        });
    }
    let uncovered_inference = point_only_inference
        || ((moderation_bootstrap || moderated_mediation_bootstrap)
            && interaction_plot_interval_fields)
        || (!moderated_mediation_bootstrap
            && moderated_mediation_derived_effect_values
                .iter()
                .any(|(_, value, _)| general_sem_estimate_has_inference(value)))
        || if moderation_bootstrap || moderated_mediation_bootstrap {
            mediation_effect_values
                .iter()
                .any(|(_, value, _)| general_sem_estimate_has_inference(value))
        } else {
            moderation_effect_values
                .iter()
                .any(|(_, value, _)| general_sem_estimate_has_inference(value))
        };
    if uncovered_inference {
        errors.push(if moderated_mediation_bootstrap {
            format!(
                "{context} moderated-mediation v1 permits inference only for scientific gamma, the three locked conditional indirect effects, and the index"
            )
        } else if moderation_bootstrap {
            format!(
                "{context} moderation v1 permits inference only for scientific_rescaled_gamma; standardized-product, joint-stage, conditional, plot, mediation, and higher-order estimates must remain point-only"
            )
        } else {
            format!(
                "{context} v1 does not cover interaction, conditional, or higher-order estimate inference"
            )
        });
    }
    let expected_effect_capability = if moderation_bootstrap || moderated_mediation_bootstrap {
        crate::pls_general_multiple_moderation_point_capability_cell_v1()
    } else {
        crate::pls_general_recursive_effects_capability_cell_v1()
    };
    for (effect_id, value, trace) in effect_values {
        let expected_effect_capability = if moderated_mediation_bootstrap
            && moderated_mediation_derived_effect_ids.contains(effect_id)
        {
            crate::pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1()
        } else {
            expected_effect_capability.clone()
        };
        if trace.capability_cell != expected_effect_capability {
            errors.push(if moderated_mediation_bootstrap {
                format!(
                    "{context} effect {effect_id} trace.capability_cell does not match its moderation-point or supplemental moderated-mediation owner"
                )
            } else if moderation_bootstrap {
                format!(
                    "{context} effect {effect_id} trace.capability_cell must equal the General SEM multiple two-way moderation point option cell"
                )
            } else {
                format!(
                    "{context} effect {effect_id} trace.capability_cell must equal the PLS recursive-effects option cell"
                )
            });
        }
        let Some(usable) = value.bootstrap_usable_replicates else {
            continue;
        };
        let Some(exceedances) = value.bootstrap_two_sided_exceedances else {
            continue;
        };
        if usable != receipt.resamples_usable {
            errors.push(format!(
                "{context} effect {effect_id} usable replicate count contradicts the receipt"
            ));
        }
        if exceedances > usable {
            errors.push(format!(
                "{context} effect {effect_id} exceedance count exceeds usable replicates"
            ));
        }
        if let Some(p_value) = value.p_value {
            let expected = f64::from(exceedances + 1) / f64::from(usable + 1);
            if !approximately_equal(p_value, expected) {
                errors.push(format!(
                    "{context} effect {effect_id} p_value contradicts the plus-one exceedance ledger"
                ));
            }
        }
    }
}

fn validate_cbsem_endpoint_v1(
    errors: &mut Vec<String>,
    endpoint: &CanonicalCbsemEndpointV1,
    context: &str,
) {
    let variable_id = match endpoint {
        CanonicalCbsemEndpointV1::Variable { variable_id }
        | CanonicalCbsemEndpointV1::Residual { variable_id }
        | CanonicalCbsemEndpointV1::Disturbance { variable_id } => variable_id,
    };
    require_stable_id(errors, variable_id, &format!("{context}.variable_id"));
}

fn validate_cbsem_parameter_result_v1(
    errors: &mut Vec<String>,
    row: &CanonicalCbsemParameterResultV1,
    document_model_id: &str,
    document_capability_ids: Option<&HashSet<String>>,
    context: &str,
) {
    validate_general_sem_trace(
        errors,
        &row.trace,
        document_model_id,
        document_capability_ids,
        &format!("{context}.trace"),
    );
    if row.trace.capability_cell != crate::cbsem_general_sem_ml_capability_cell_v1() {
        errors.push(format!(
            "{context}.trace.capability_cell must equal the exact CB-SEM General SEM ML point cell"
        ));
    }
    if let Some(relation_id) = &row.relation_id {
        require_stable_id(errors, relation_id, &format!("{context}.relation_id"));
    }
    let target_matches_role = matches!(
        (&row.role, &row.target),
        (
            CanonicalCbsemParameterRoleV1::Loading,
            CanonicalCbsemParameterTargetV1::Loading { .. }
        ) | (
            CanonicalCbsemParameterRoleV1::Regression,
            CanonicalCbsemParameterTargetV1::Regression { .. }
        ) | (
            CanonicalCbsemParameterRoleV1::Covariance,
            CanonicalCbsemParameterTargetV1::Covariance { .. }
        ) | (
            CanonicalCbsemParameterRoleV1::Variance,
            CanonicalCbsemParameterTargetV1::Variance { .. }
        )
    );
    if !target_matches_role {
        errors.push(format!("{context}.role and target kind must agree"));
    }
    match &row.target {
        CanonicalCbsemParameterTargetV1::Loading {
            factor_id,
            indicator_id,
        } => {
            require_stable_id(errors, factor_id, &format!("{context}.target.factor_id"));
            require_stable_id(
                errors,
                indicator_id,
                &format!("{context}.target.indicator_id"),
            );
            if factor_id == indicator_id {
                errors.push(format!(
                    "{context}.target loading factor and indicator must differ"
                ));
            }
        }
        CanonicalCbsemParameterTargetV1::Regression {
            source_id,
            target_id,
        } => {
            require_stable_id(errors, source_id, &format!("{context}.target.source_id"));
            require_stable_id(errors, target_id, &format!("{context}.target.target_id"));
            if source_id == target_id {
                errors.push(format!(
                    "{context}.target regression source and target must differ"
                ));
            }
        }
        CanonicalCbsemParameterTargetV1::Covariance { left, right } => {
            validate_cbsem_endpoint_v1(errors, left, &format!("{context}.target.left"));
            validate_cbsem_endpoint_v1(errors, right, &format!("{context}.target.right"));
            if left == right {
                errors.push(format!("{context}.target covariance endpoints must differ"));
            }
        }
        CanonicalCbsemParameterTargetV1::Variance { endpoint } => {
            validate_cbsem_endpoint_v1(errors, endpoint, &format!("{context}.target.endpoint"));
        }
    }
    if !row.estimate.is_finite() {
        errors.push(format!("{context}.estimate must be finite"));
    }
    let uncertainty_count = [
        row.standard_error.is_some(),
        row.z_value.is_some(),
        row.p_value.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if uncertainty_count != 0 && uncertainty_count != 3 {
        errors.push(format!(
            "{context} standard_error, z_value, and p_value must be all absent or all present"
        ));
    }
    if row
        .standard_error
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        errors.push(format!(
            "{context}.standard_error must be finite and nonnegative"
        ));
    }
    if row.z_value.is_some_and(|value| !value.is_finite()) {
        errors.push(format!("{context}.z_value must be finite"));
    }
    if row
        .p_value
        .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        errors.push(format!(
            "{context}.p_value must be finite and between 0 and 1"
        ));
    }
    if row
        .standardized_estimate
        .is_some_and(|value| !value.is_finite())
    {
        errors.push(format!("{context}.standardized_estimate must be finite"));
    }
    match &row.state {
        CanonicalCbsemParameterStateV1::Fixed { value } => {
            if !value.is_finite() || !approximately_equal(*value, row.estimate) {
                errors.push(format!(
                    "{context}.state fixed value must be finite and equal estimate"
                ));
            }
            if uncertainty_count != 0 {
                errors.push(format!(
                    "{context} fixed parameters must not publish sampling uncertainty"
                ));
            }
        }
        CanonicalCbsemParameterStateV1::Free {
            equality_label,
            lower,
            upper,
        } => {
            if equality_label
                .as_deref()
                .is_some_and(|label| label.trim().is_empty())
            {
                errors.push(format!(
                    "{context}.state.equality_label must be nonempty when present"
                ));
            }
            validate_general_sem_bounds(errors, *lower, *upper, &format!("{context}.state"));
            if lower.is_some_and(|value| !value.is_finite())
                || upper.is_some_and(|value| !value.is_finite())
            {
                errors.push(format!("{context}.state bounds must be finite"));
            }
            if lower.is_some_and(|value| row.estimate < value)
                || upper.is_some_and(|value| row.estimate > value)
            {
                errors.push(format!(
                    "{context}.estimate must satisfy its declared bounds"
                ));
            }
        }
    }
}

fn validate_cbsem_bootstrap_v1(
    errors: &mut Vec<String>,
    results: &CanonicalGeneralSemResultsV1,
    provenance: &CanonicalResultProvenanceV2,
    document_capability_ids: Option<&HashSet<String>>,
) {
    let context = "general_sem_results.cbsem_bootstrap_receipt";
    let Some(receipt) = &results.cbsem_bootstrap_receipt else {
        if !results.cbsem_bootstrap_inference.is_empty() {
            errors.push(format!(
                "{context} is required when cbsem_bootstrap_inference is present"
            ));
        }
        return;
    };
    if results.cbsem_bootstrap_inference.is_empty() {
        errors.push(format!(
            "{context} requires at least one cbsem_bootstrap_inference row"
        ));
    }
    validate_capability_reference(
        errors,
        &receipt.capability_cell,
        &format!("{context}.capability_cell"),
    );
    let exact_cell = crate::cbsem_recursive_sem_bootstrap_capability_cell_v1();
    if receipt.capability_cell != exact_cell {
        errors.push(format!(
            "{context}.capability_cell must equal the exact recursive-SEM bootstrap cell"
        ));
    }
    let identity = capability_cell_reference_identity_v2(&receipt.capability_cell);
    if document_capability_ids.is_some_and(|ids| !ids.contains(&identity)) {
        errors.push(format!(
            "{context}.capability_cell references undeclared option cell {identity}"
        ));
    }
    if receipt.method_version != CBSEM_RECURSIVE_SEM_BOOTSTRAP_METHOD_VERSION_V1 {
        errors.push(format!(
            "{context}.method_version is not the frozen v1 method"
        ));
    }
    if receipt.resampling_operation_version != CBSEM_RECURSIVE_SEM_BOOTSTRAP_OPERATION_VERSION_V1 {
        errors.push(format!(
            "{context}.resampling_operation_version is not the frozen v1 operation"
        ));
    }
    if receipt.quantile_method_version != GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1 {
        errors.push(format!(
            "{context}.quantile_method_version must equal type7_quantile_v1"
        ));
    }
    for (name, value) in [
        (
            "compiled_plan_sha256",
            receipt.compiled_plan_sha256.as_str(),
        ),
        ("base_plan_sha256", receipt.base_plan_sha256.as_str()),
        (
            "parameter_inventory_sha256",
            receipt.parameter_inventory_sha256.as_str(),
        ),
        (
            "model_scientific_sha256",
            receipt.model_scientific_sha256.as_str(),
        ),
        (
            "general_sem_config_sha256",
            receipt.general_sem_config_sha256.as_str(),
        ),
        (
            "recipe_analytical_sha256",
            receipt.recipe_analytical_sha256.as_str(),
        ),
        (
            "complete_case_frame_sha256",
            receipt.complete_case_frame_sha256.as_str(),
        ),
        (
            "usable_replicate_indices_sha256",
            receipt.usable_replicate_indices_sha256.as_str(),
        ),
    ] {
        if !is_lowercase_sha256(value) {
            errors.push(format!("{context}.{name} must be a lowercase SHA-256"));
        }
    }
    if receipt.model_scientific_sha256 != provenance.model_digest {
        errors.push(format!(
            "{context}.model_scientific_sha256 must equal provenance.model_digest"
        ));
    }
    if receipt.recipe_analytical_sha256 != provenance.recipe_digest {
        errors.push(format!(
            "{context}.recipe_analytical_sha256 must equal provenance.recipe_digest"
        ));
    }
    if receipt.source_dataset_fingerprint != provenance.dataset_fingerprint
        || !is_dataset_fingerprint_v1(&receipt.source_dataset_fingerprint)
    {
        errors.push(format!(
            "{context}.source_dataset_fingerprint must equal provenance.dataset_fingerprint"
        ));
    }
    if receipt.confidence_level.to_bits() != 0.95_f64.to_bits() {
        errors.push(format!("{context}.confidence_level must equal 0.95"));
    }
    if !(500..=10_000).contains(&receipt.resamples_requested) {
        errors.push(format!(
            "{context}.resamples_requested must be between 500 and 10000"
        ));
    }
    let expected_minimum = (f64::from(receipt.resamples_requested) * 0.9).ceil() as u32;
    if receipt.minimum_usable_resamples != expected_minimum {
        errors.push(format!(
            "{context}.minimum_usable_resamples must equal the 90 percent usable gate"
        ));
    }
    if receipt.resamples_usable > receipt.resamples_requested
        || receipt.resamples_usable as usize + receipt.failed_replicates.len()
            != receipt.resamples_requested as usize
    {
        errors.push(format!(
            "{context} requested count must equal usable plus failed replicates"
        ));
    }
    if !(1..=64).contains(&receipt.workers) || i64::from(receipt.workers) != provenance.workers {
        errors.push(format!(
            "{context}.workers must be between 1 and 64 and equal provenance.workers"
        ));
    }
    match receipt.seed.parse::<u64>() {
        Ok(seed) if seed.to_string() == receipt.seed => {
            if provenance.seed.and_then(|value| u64::try_from(value).ok()) != Some(seed) {
                errors.push(format!("{context}.seed must equal provenance.seed"));
            }
        }
        _ => errors.push(format!("{context}.seed must be a canonical decimal u64")),
    }
    if !receipt.complete_model_reestimated_per_replicate {
        errors.push(format!(
            "{context}.complete_model_reestimated_per_replicate must be true"
        ));
    }
    let mut previous = None;
    for (index, failure) in receipt.failed_replicates.iter().enumerate() {
        let failure_context = format!("{context}.failed_replicates[{index}]");
        if failure.replicate_index >= receipt.resamples_requested
            || previous.is_some_and(|value| value >= failure.replicate_index)
        {
            errors.push(format!(
                "{failure_context}.replicate_index must be unique, ordered, and in range"
            ));
        }
        previous = Some(failure.replicate_index);
        if failure.message.trim().is_empty() {
            errors.push(format!("{failure_context}.message must be nonempty"));
        }
    }

    let point_rows = results
        .cbsem_parameters
        .iter()
        .map(|row| (row.parameter_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let parameter_ids = results
        .cbsem_bootstrap_inference
        .iter()
        .map(|row| row.parameter_id.as_str())
        .collect::<Vec<_>>();
    if receipt.parameter_inventory_sha256 != crate::sha256_serialized(&parameter_ids) {
        errors.push(format!(
            "{context}.parameter_inventory_sha256 must bind the ordered inference parameter IDs"
        ));
    }
    for (index, inference) in results.cbsem_bootstrap_inference.iter().enumerate() {
        let inference_context = format!("general_sem_results.cbsem_bootstrap_inference[{index}]");
        validate_general_sem_trace(
            errors,
            &inference.trace,
            provenance.model_id.as_str(),
            document_capability_ids,
            &format!("{inference_context}.trace"),
        );
        if inference.trace.capability_cell != exact_cell {
            errors.push(format!(
                "{inference_context}.trace.capability_cell must equal the recursive-SEM bootstrap cell"
            ));
        }
        let Some(point) = point_rows.get(inference.parameter_id.as_str()) else {
            errors.push(format!(
                "{inference_context}.parameter_id must reference cbsem_parameters"
            ));
            continue;
        };
        if !inference.point_estimate.is_finite()
            || !approximately_equal(inference.point_estimate, point.estimate)
        {
            errors.push(format!(
                "{inference_context}.point_estimate must equal the point parameter estimate"
            ));
        }
        match &inference.outcome {
            CanonicalCbsemBootstrapInferenceOutcomeV1::Available { value } => {
                if !matches!(point.state, CanonicalCbsemParameterStateV1::Free { .. }) {
                    errors.push(format!(
                        "{inference_context}.outcome cannot publish inference for a fixed parameter"
                    ));
                }
                validate_general_sem_estimate(
                    errors,
                    value,
                    &format!("{inference_context}.outcome.value"),
                );
                if !approximately_equal(value.estimate, inference.point_estimate)
                    || !general_sem_estimate_has_inference(value)
                {
                    errors.push(format!(
                        "{inference_context}.outcome.value must contain inference for the same point estimate"
                    ));
                }
                if value.bootstrap_usable_replicates != Some(receipt.resamples_usable) {
                    errors.push(format!(
                        "{inference_context}.outcome.value bootstrap usable count must equal the receipt"
                    ));
                }
                if let (Some(exceedances), Some(p_value)) =
                    (value.bootstrap_two_sided_exceedances, value.p_value)
                {
                    if exceedances > receipt.resamples_usable
                        || !approximately_equal(
                            p_value,
                            f64::from(exceedances + 1)
                                / f64::from(receipt.resamples_usable + 1),
                        )
                    {
                        errors.push(format!(
                            "{inference_context}.outcome.value p_value contradicts the plus-one exceedance ledger"
                        ));
                    }
                }
                if receipt.resamples_usable < receipt.minimum_usable_resamples {
                    errors.push(format!(
                        "{inference_context} cannot be available below the usable-replicate gate"
                    ));
                }
            }
            CanonicalCbsemBootstrapInferenceOutcomeV1::Unavailable {
                reason: CanonicalCbsemBootstrapUnavailableReasonV1::InsufficientUsableReplicates,
            } if receipt.resamples_usable >= receipt.minimum_usable_resamples => errors.push(
                format!(
                    "{inference_context} cannot report insufficient usable replicates after the gate passed"
                ),
            ),
            CanonicalCbsemBootstrapInferenceOutcomeV1::Unavailable { .. } => {}
        }
    }
}

fn validate_canonical_moderated_mediation_results_v1<'a>(
    errors: &mut Vec<String>,
    results: &'a CanonicalGeneralSemResultsV1,
    document_model_id: &str,
    document_capability_ids: Option<&HashSet<String>>,
    effect_ids: &mut BTreeSet<&'a str>,
) {
    let context = "general_sem_results";
    let supplemental_cell =
        crate::pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1();
    let combined_receipt = results
        .inference_receipt
        .as_ref()
        .map(|receipt| &receipt.capability_cell)
        == Some(&supplemental_cell);
    if results.conditional_indirect_effects.is_empty()
        && results.moderated_mediation_indices.is_empty()
        && !combined_receipt
    {
        return;
    }
    if results.conditional_indirect_effects.len() != 3 {
        errors.push(format!(
            "{context}.conditional_indirect_effects must contain exactly the locked -1/0/+1 targets"
        ));
    }
    if results.moderated_mediation_indices.len() != 1 {
        errors.push(format!(
            "{context}.moderated_mediation_indices must contain exactly one index"
        ));
    }
    if results.interaction_effects.len() != 1 {
        errors.push(format!(
            "{context} two-way moderated mediation requires exactly one interaction effect"
        ));
    }
    if results
        .inference_receipt
        .as_ref()
        .map(|receipt| &receipt.capability_cell)
        != Some(&supplemental_cell)
    {
        errors.push(format!(
            "{context} moderated-mediation rows require the exact combined bootstrap receipt"
        ));
    }

    let mut probe_indices = BTreeSet::new();
    for (index, effect) in results.conditional_indirect_effects.iter().enumerate() {
        let item_context = format!("{context}.conditional_indirect_effects[{index}]");
        if !effect_ids.insert(effect.effect_id.as_str()) {
            errors.push(format!(
                "{item_context}.effect_id is duplicated across effect sections"
            ));
        }
        for (name, id) in [
            ("target_id", effect.target_id.as_str()),
            ("estimand_id", effect.estimand_id.as_str()),
            ("interaction_id", effect.interaction_id.as_str()),
            ("x_id", effect.x_id.as_str()),
            ("mediator_id", effect.mediator_id.as_str()),
            ("y_id", effect.y_id.as_str()),
            ("moderator_id", effect.moderator_id.as_str()),
        ] {
            require_stable_id(errors, id, &format!("{item_context}.{name}"));
        }
        validate_general_sem_trace(
            errors,
            &effect.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        if effect.trace.capability_cell != supplemental_cell {
            errors.push(format!(
                "{item_context}.trace.capability_cell must equal the supplemental two-way moderated-mediation cell"
            ));
        }
        if effect.ordered_relation_ids.len() != 2 {
            errors.push(format!(
                "{item_context}.ordered_relation_ids must contain exactly two relations"
            ));
        }
        require_unique_ids(
            errors,
            effect.ordered_relation_ids.iter().map(String::as_str),
            &format!("{item_context}.ordered_relation_ids"),
        );
        if effect.probe_value_index > 2 {
            errors.push(format!(
                "{item_context}.probe_value_index must be 0, 1, or 2"
            ));
        } else {
            probe_indices.insert(effect.probe_value_index);
            let expected_probe = crate::GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_PROBES_V1
                [effect.probe_value_index as usize];
            if !approximately_equal(effect.moderator_value, expected_probe) {
                errors.push(format!(
                    "{item_context}.moderator_value must equal the locked standardized probe"
                ));
            }
        }
        if effect.effect_id
            != crate::conditional_indirect_effect_identity_v1(
                &effect.target_id,
                effect.probe_value_index,
            )
        {
            errors.push(format!(
                "{item_context}.effect_id must equal the canonical target/probe identity"
            ));
        }
        validate_general_sem_estimate(errors, &effect.value, &format!("{item_context}.value"));
    }
    if probe_indices != BTreeSet::from([0, 1, 2]) {
        errors.push(format!(
            "{context}.conditional_indirect_effects must cover probe indices 0, 1, and 2 exactly"
        ));
    }

    for (index, effect) in results.moderated_mediation_indices.iter().enumerate() {
        let item_context = format!("{context}.moderated_mediation_indices[{index}]");
        if !effect_ids.insert(effect.effect_id.as_str()) {
            errors.push(format!(
                "{item_context}.effect_id is duplicated across effect sections"
            ));
        }
        for (name, id) in [
            ("target_id", effect.target_id.as_str()),
            ("estimand_id", effect.estimand_id.as_str()),
            ("interaction_id", effect.interaction_id.as_str()),
            ("x_id", effect.x_id.as_str()),
            ("mediator_id", effect.mediator_id.as_str()),
            ("y_id", effect.y_id.as_str()),
            ("moderator_id", effect.moderator_id.as_str()),
        ] {
            require_stable_id(errors, id, &format!("{item_context}.{name}"));
        }
        validate_general_sem_trace(
            errors,
            &effect.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        if effect.trace.capability_cell != supplemental_cell {
            errors.push(format!(
                "{item_context}.trace.capability_cell must equal the supplemental two-way moderated-mediation cell"
            ));
        }
        if effect.ordered_relation_ids.len() != 2 {
            errors.push(format!(
                "{item_context}.ordered_relation_ids must contain exactly two relations"
            ));
        }
        require_unique_ids(
            errors,
            effect.ordered_relation_ids.iter().map(String::as_str),
            &format!("{item_context}.ordered_relation_ids"),
        );
        if effect.effect_id != crate::moderated_mediation_index_identity_v1(&effect.target_id) {
            errors.push(format!(
                "{item_context}.effect_id must equal the canonical target index identity"
            ));
        }
        validate_general_sem_estimate(errors, &effect.value, &format!("{item_context}.value"));
    }

    let (Some(first), Some(index_effect), Some(interaction)) = (
        results.conditional_indirect_effects.first(),
        results.moderated_mediation_indices.first(),
        results.interaction_effects.first(),
    ) else {
        return;
    };
    for effect in &results.conditional_indirect_effects {
        if effect.target_id != first.target_id
            || effect.estimand_id != first.estimand_id
            || effect.moderated_stage != first.moderated_stage
            || effect.interaction_id != first.interaction_id
            || effect.x_id != first.x_id
            || effect.mediator_id != first.mediator_id
            || effect.y_id != first.y_id
            || effect.moderator_id != first.moderator_id
            || effect.ordered_relation_ids != first.ordered_relation_ids
        {
            errors.push(format!(
                "{context}.conditional_indirect_effects must share one exact compiled target"
            ));
            break;
        }
    }
    if index_effect.target_id != first.target_id
        || index_effect.estimand_id != first.estimand_id
        || index_effect.moderated_stage != first.moderated_stage
        || index_effect.interaction_id != first.interaction_id
        || index_effect.x_id != first.x_id
        || index_effect.mediator_id != first.mediator_id
        || index_effect.y_id != first.y_id
        || index_effect.moderator_id != first.moderator_id
        || index_effect.ordered_relation_ids != first.ordered_relation_ids
    {
        errors.push(format!(
            "{context}.moderated_mediation_indices must identify the same target as the conditional indirect effects"
        ));
    }
    let distinct_nodes = [
        first.x_id.as_str(),
        first.mediator_id.as_str(),
        first.y_id.as_str(),
        first.moderator_id.as_str(),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if distinct_nodes.len() != 4 {
        errors.push(format!(
            "{context} moderated mediation requires distinct X, M, Y, and W identities"
        ));
    }
    if interaction.interaction_id != first.interaction_id
        || interaction.moderator_id != first.moderator_id
    {
        errors.push(format!(
            "{context} moderated-mediation target must bind the one published interaction effect"
        ));
    }
    if first.ordered_relation_ids.len() != 2 {
        return;
    }
    let (moderated_relation_id, other_relation_id, expected_focal, expected_outcome) =
        match first.moderated_stage {
            CanonicalModeratedMediationStageV1::FirstStage => (
                first.ordered_relation_ids[0].as_str(),
                first.ordered_relation_ids[1].as_str(),
                first.x_id.as_str(),
                first.mediator_id.as_str(),
            ),
            CanonicalModeratedMediationStageV1::SecondStage => (
                first.ordered_relation_ids[1].as_str(),
                first.ordered_relation_ids[0].as_str(),
                first.mediator_id.as_str(),
                first.y_id.as_str(),
            ),
        };
    if interaction.focal_relation_id != moderated_relation_id
        || interaction.focal_predictor_id != expected_focal
        || interaction.outcome_id != expected_outcome
    {
        errors.push(format!(
            "{context} interaction effect does not match the declared moderated path stage"
        ));
    }
    let moderated_beta = results
        .joint_stage_structural_coefficients
        .iter()
        .find(|coefficient| coefficient.relation_id == moderated_relation_id)
        .map(|coefficient| coefficient.estimate.estimate);
    let other_beta = results
        .joint_stage_structural_coefficients
        .iter()
        .find(|coefficient| coefficient.relation_id == other_relation_id)
        .map(|coefficient| coefficient.estimate.estimate);
    let (Some(moderated_beta), Some(other_beta)) = (moderated_beta, other_beta) else {
        errors.push(format!(
            "{context} moderated-mediation formulas require both selected path coefficients in the joint-stage ledger"
        ));
        return;
    };
    let gamma = interaction.scientific_rescaled_gamma.estimate;
    for effect in &results.conditional_indirect_effects {
        let expected = (moderated_beta + gamma * effect.moderator_value) * other_beta;
        if !approximately_equal(effect.value.estimate, expected) {
            errors.push(format!(
                "{context} conditional indirect effect {} contradicts the bounded formula",
                effect.effect_id
            ));
        }
    }
    let expected_index = gamma * other_beta;
    if !approximately_equal(index_effect.value.estimate, expected_index) {
        errors.push(format!(
            "{context} moderated-mediation index contradicts scientific gamma times the other-stage coefficient"
        ));
    }
}

fn validate_hoc_inference_receipt_v1(
    errors: &mut Vec<String>,
    results: &CanonicalGeneralSemResultsV1,
    provenance: &CanonicalResultProvenanceV2,
    document_capability_ids: Option<&HashSet<String>>,
) {
    let context = "general_sem_results.higher_order_inference_receipt";
    let inferred_relations = results
        .higher_order_stages
        .iter()
        .flat_map(|stage| stage.relation_estimates.iter())
        .filter(|relation| general_sem_estimate_has_inference(&relation.value))
        .collect::<Vec<_>>();
    let Some(receipt) = &results.higher_order_inference_receipt else {
        if !inferred_relations.is_empty() {
            errors.push(format!(
                "{context} is required when higher-order relations contain inference"
            ));
        }
        return;
    };
    if results.inference_receipt.is_some() {
        errors.push(format!(
            "{context} is mutually exclusive with the mediation/moderation inference receipt"
        ));
    }
    validate_capability_reference(
        errors,
        &receipt.capability_cell,
        &format!("{context}.capability_cell"),
    );
    let receipt_identity = capability_cell_reference_identity_v2(&receipt.capability_cell);
    if document_capability_ids.is_none_or(|ids| !ids.contains(&receipt_identity)) {
        errors.push(format!(
            "{context}.capability_cell must be declared by document capability_cells"
        ));
    }
    if receipt.capability_cell != crate::pls_general_higher_order_bootstrap_capability_cell_v1() {
        errors.push(format!(
            "{context}.capability_cell must equal the exact HOC bootstrap cell"
        ));
    }
    if receipt.schema_version != 1
        || receipt.method_version != crate::PLS_GENERAL_HIGHER_ORDER_BOOTSTRAP_CAPABILITY_VERSION_V1
        || receipt.point_method_version
            != crate::PLS_GENERAL_HIGHER_ORDER_POINT_CAPABILITY_VERSION_V1
        || receipt.resampling_operation_version
            != "general_sem_pls_higher_order_full_model_case_bootstrap_operation_v1"
        || receipt.resampling_stream_version != "indexed_case_resampling_v1"
        || receipt.quantile_method_version != "type7_quantile_v1"
        || receipt.standard_error_method_version != "sample_standard_error_b_minus_1_v1"
        || receipt.summation_method_version != "neumaier_compensated_sum_v1"
        || receipt.p_value_method_version != "null_centered_plus_one_v1"
        || receipt.failure_policy_version != "minimum_usable_fraction_0_9_v1"
        || receipt.sign_alignment_method_version != "sampled_original_construct_score_covariance_v1"
        || receipt.target_version != "compiled_hoc_component_and_structural_relation_target_v1"
    {
        errors.push(format!(
            "{context} schema, point method, or bootstrap method identity is invalid"
        ));
    }
    for (name, digest) in [
        (
            "general_sem_config_sha256",
            receipt.general_sem_config_sha256.as_str(),
        ),
        (
            "compiled_plan_sha256",
            receipt.compiled_plan_sha256.as_str(),
        ),
        (
            "hoc_stage_plan_sha256",
            receipt.hoc_stage_plan_sha256.as_str(),
        ),
        (
            "model_scientific_sha256",
            receipt.model_scientific_sha256.as_str(),
        ),
        (
            "stage_one_model_scientific_sha256",
            receipt.stage_one_model_scientific_sha256.as_str(),
        ),
        (
            "stage_two_model_scientific_sha256",
            receipt.stage_two_model_scientific_sha256.as_str(),
        ),
        (
            "complete_case_frame_sha256",
            receipt.complete_case_frame_sha256.as_str(),
        ),
        (
            "usable_replicate_indices_sha256",
            receipt.usable_replicate_indices_sha256.as_str(),
        ),
        (
            "target_identity_set_sha256",
            receipt.target_identity_set_sha256.as_str(),
        ),
    ] {
        if !is_lowercase_sha256(digest) {
            errors.push(format!("{context}.{name} must be a lowercase SHA-256"));
        }
    }
    if receipt.model_scientific_sha256 != provenance.model_digest
        || receipt.source_dataset_fingerprint != provenance.dataset_fingerprint
        || receipt.seed.parse::<i64>().ok() != provenance.seed
        || i64::from(receipt.workers) != provenance.workers
    {
        errors.push(format!(
            "{context} model, dataset, seed, or worker authority differs from provenance"
        ));
    }
    if receipt.interval != CanonicalGeneralSemBootstrapIntervalV1::PercentileType7
        || receipt.tail != CanonicalGeneralSemInferenceTailV1::TwoSided
        || !receipt.confidence_level.is_finite()
        || !(0.0..1.0).contains(&receipt.confidence_level)
        || !(2..=10_000).contains(&receipt.resamples_requested)
    {
        errors.push(format!(
            "{context} inference configuration is outside the exact percentile two-sided contract"
        ));
    }
    let expected_minimum = ((f64::from(receipt.resamples_requested) * 0.9).ceil() as u32).max(2);
    if receipt.minimum_usable_resamples != expected_minimum
        || receipt.resamples_usable < expected_minimum
        || receipt.resamples_usable > receipt.resamples_requested
        || receipt.resamples_usable as usize + receipt.failed_replicates.len()
            != receipt.resamples_requested as usize
        || !(1..=64).contains(&receipt.workers)
        || !receipt.complete_model_reestimated_per_replicate
        || !receipt.stage_one_reestimated_per_replicate
        || !receipt.generated_component_values_recalculated_per_replicate
        || !receipt.stage_one_scores_sign_aligned_per_replicate
        || !receipt.stage_two_reestimated_per_replicate
        || !receipt.stage_two_scores_sign_aligned_per_replicate
        || !receipt.complete_point_contract_validated_per_replicate
    {
        errors.push(format!(
            "{context} usable gate or full two-stage refit flags are invalid"
        ));
    }
    require_canonical_stable_ids(
        errors,
        receipt.target_ids.iter().map(String::as_str),
        &format!("{context}.target_ids"),
    );
    let identity_ids = receipt
        .target_identities
        .iter()
        .map(|identity| identity.target_id.as_str())
        .collect::<Vec<_>>();
    if identity_ids
        != receipt
            .target_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
        || crate::sha256_serialized(&receipt.target_identities)
            != receipt.target_identity_set_sha256
    {
        errors.push(format!(
            "{context} target identities or identity digest contradict target_ids"
        ));
    }
    let relations = results
        .higher_order_stages
        .iter()
        .flat_map(|stage| stage.relation_estimates.iter())
        .map(|relation| (relation.relation_id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    for identity in &receipt.target_identities {
        let Some(relation) = relations.get(identity.relation_id.as_str()) else {
            errors.push(format!(
                "{context} target {} references a missing HOC relation",
                identity.target_id
            ));
            continue;
        };
        let expected_kind = match identity.kind {
            CanonicalHocBootstrapTargetKindV1::ComponentLoading => {
                CanonicalHocRelationKindV1::ComponentLoading
            }
            CanonicalHocBootstrapTargetKindV1::ComponentWeight => {
                CanonicalHocRelationKindV1::ComponentWeight
            }
            CanonicalHocBootstrapTargetKindV1::HocStructuralPath => {
                CanonicalHocRelationKindV1::AuthoredStructural
            }
            CanonicalHocBootstrapTargetKindV1::ExtendedTotalEffect => {
                CanonicalHocRelationKindV1::ExtendedTotalEffect
            }
        };
        if identity.target_id != identity.relation_id
            || identity.target_version != receipt.target_version
            || identity.point_method_version != receipt.point_method_version
            || relation.parameter_id.as_deref() != Some(identity.parameter_id.as_str())
            || relation.source_id != identity.source_id
            || relation.target_id != identity.target_variable_id
            || relation.kind != Some(expected_kind)
            || !general_sem_estimate_has_complete_inference(&relation.value)
            || relation.value.bootstrap_usable_replicates != Some(receipt.resamples_usable)
        {
            errors.push(format!(
                "{context} target {} differs from its typed HOC relation or inference ledger",
                identity.target_id
            ));
        }
        if let (Some(exceedances), Some(usable), Some(p_value)) = (
            relation.value.bootstrap_two_sided_exceedances,
            relation.value.bootstrap_usable_replicates,
            relation.value.p_value,
        ) {
            let expected = f64::from(exceedances + 1) / f64::from(usable + 1);
            if exceedances > usable || !approximately_equal(p_value, expected) {
                errors.push(format!(
                    "{context} target {} contradicts the plus-one probability ledger",
                    identity.target_id
                ));
            }
        }
    }
    let target_ids = receipt
        .target_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if inferred_relations
        .iter()
        .any(|relation| !target_ids.contains(relation.relation_id.as_str()))
    {
        errors.push(format!(
            "{context} leaves inferred HOC relations outside its exact target inventory"
        ));
    }
    let mut previous_failure = None;
    let mut failed_indices = BTreeSet::new();
    for failure in &receipt.failed_replicates {
        if failure.replicate_index >= receipt.resamples_requested
            || previous_failure.is_some_and(|previous| previous >= failure.replicate_index)
            || failure.message.trim().is_empty()
        {
            errors.push(format!("{context}.failed_replicates is not canonical"));
        }
        previous_failure = Some(failure.replicate_index);
        failed_indices.insert(failure.replicate_index);
    }
    let usable_indices = (0..receipt.resamples_requested)
        .filter(|index| !failed_indices.contains(index))
        .collect::<Vec<_>>();
    if crate::sha256_serialized(&usable_indices) != receipt.usable_replicate_indices_sha256 {
        errors.push(format!(
            "{context}.usable_replicate_indices_sha256 contradicts the failure ledger"
        ));
    }
}

fn validate_three_way_moderation_results_v1(
    errors: &mut Vec<String>,
    results: &CanonicalGeneralSemResultsV1,
    provenance: &CanonicalResultProvenanceV2,
    document_capability_ids: Option<&HashSet<String>>,
) {
    let context = "general_sem_results.three_way_moderation";
    let has_rows = !results.three_way_interaction_effects.is_empty()
        || !results.three_way_conditional_interaction_effects.is_empty()
        || !results.three_way_simple_slopes.is_empty();
    if !has_rows {
        if results.three_way_moderation_bootstrap_receipt.is_some() {
            errors.push(format!(
                "{context} receipt requires typed three-way result rows"
            ));
        }
        return;
    }
    if results.three_way_interaction_effects.len() != 1 {
        errors.push(format!(
            "{context} requires exactly one bounded three-way interaction row"
        ));
    }
    let point_cell = crate::pls_general_three_way_moderation_point_capability_cell_v1();
    let authority = results.three_way_interaction_effects.first();
    let mut target_ids = Vec::new();
    let mut interaction_id = None;
    for (index, effect) in results.three_way_interaction_effects.iter().enumerate() {
        let item = format!("{context}.interaction_effects[{index}]");
        target_ids.push(effect.effect_id.clone());
        for (name, id) in [
            ("effect_id", effect.effect_id.as_str()),
            ("interaction_id", effect.interaction_id.as_str()),
            ("focal_relation_id", effect.focal_relation_id.as_str()),
            (
                "interaction_effect_relation_id",
                effect.interaction_effect_relation_id.as_str(),
            ),
            (
                "interaction_effect_parameter_id",
                effect.interaction_effect_parameter_id.as_str(),
            ),
            ("outcome_id", effect.outcome_id.as_str()),
            (
                "generated_product_column_id",
                effect.generated_product_column_id.as_str(),
            ),
        ] {
            require_stable_id(errors, id, &format!("{item}.{name}"));
        }
        for (operand_index, operand_id) in effect.operand_ids.iter().enumerate() {
            require_stable_id(
                errors,
                operand_id,
                &format!("{item}.operand_ids[{operand_index}]"),
            );
        }
        validate_general_sem_trace(
            errors,
            &effect.trace,
            &provenance.model_id,
            document_capability_ids,
            &format!("{item}.trace"),
        );
        if effect.trace.capability_cell != point_cell {
            errors.push(format!(
                "{item}.trace must use the exact three-way point cell"
            ));
        }
        if effect.operand_ids.iter().collect::<BTreeSet<_>>().len() != 3 {
            errors.push(format!(
                "{item}.operand_ids must contain ordered distinct X, W, and Z"
            ));
        }
        if effect
            .operand_ids
            .iter()
            .any(|operand| operand == &effect.outcome_id)
        {
            errors.push(format!(
                "{item}.outcome_id must be distinct from ordered X, W, and Z"
            ));
        }
        if effect.effect_id != format!("three_way_delta:{}", effect.interaction_id) {
            errors.push(format!(
                "{item}.effect_id must be the canonical three-way delta target"
            ));
        }
        interaction_id = Some(effect.interaction_id.as_str());
        if effect.method_version != GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1
            || effect.product_scale_version != GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
            || effect.hierarchy_policy_version != GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1
            || effect.hierarchy_policy != CanonicalInteractionHierarchyPolicyV1::Strong
        {
            errors.push(format!(
                "{item} method, product scale, or hierarchy version is not the exact v1 contract"
            ));
        }
        if !is_lowercase_sha256(&effect.stage_one_model_scientific_sha256)
            || effect.stage_one_model_scientific_sha256 == provenance.model_digest
        {
            errors.push(format!("{item}.stage_one_model_scientific_sha256 must identify the projected stage-one model"));
        }
        if effect.observation_count < 3
            || !effect.unstandardized_product_mean.is_finite()
            || !effect
                .unstandardized_product_sample_standard_deviation
                .is_finite()
            || effect.unstandardized_product_sample_standard_deviation <= f64::EPSILON
        {
            errors.push(format!("{item} product-scale receipt is invalid"));
        }
        validate_general_sem_estimate(
            errors,
            &effect.standardized_product_coefficient,
            &format!("{item}.standardized_product_coefficient"),
        );
        validate_general_sem_estimate(
            errors,
            &effect.scientific_rescaled_delta,
            &format!("{item}.scientific_rescaled_delta"),
        );
        if general_sem_estimate_has_inference(&effect.standardized_product_coefficient) {
            errors.push(format!(
                "{item}.standardized_product_coefficient must remain point-only"
            ));
        }
        if !approximately_equal(
            effect.standardized_product_coefficient.estimate
                / effect.unstandardized_product_sample_standard_deviation,
            effect.scientific_rescaled_delta.estimate,
        ) {
            errors.push(format!(
                "{item}.scientific_rescaled_delta has inconsistent scaling"
            ));
        }
    }
    let mut second_probe_rows = BTreeMap::new();
    for (index, effect) in results
        .three_way_conditional_interaction_effects
        .iter()
        .enumerate()
    {
        let item = format!("{context}.conditional_interaction_effects[{index}]");
        target_ids.push(effect.effect_id.clone());
        for (name, id) in [
            ("effect_id", effect.effect_id.as_str()),
            ("interaction_id", effect.interaction_id.as_str()),
            ("focal_relation_id", effect.focal_relation_id.as_str()),
            ("first_moderator_id", effect.first_moderator_id.as_str()),
            ("second_moderator_id", effect.second_moderator_id.as_str()),
        ] {
            require_stable_id(errors, id, &format!("{item}.{name}"));
        }
        validate_general_sem_trace(
            errors,
            &effect.trace,
            &provenance.model_id,
            document_capability_ids,
            &format!("{item}.trace"),
        );
        if effect.trace.capability_cell != point_cell
            || interaction_id.is_some_and(|id| effect.interaction_id != id)
            || authority.is_some_and(|row| {
                effect.focal_relation_id != row.focal_relation_id
                    || effect.first_moderator_id != row.operand_ids[1]
                    || effect.second_moderator_id != row.operand_ids[2]
            })
            || effect.effect_id
                != format!(
                    "three_way_conditional_xw:{}:z{}",
                    effect.interaction_id, effect.second_moderator_probe_index
                )
            || !probe_value_matches_kind(
                effect.second_moderator_probe_kind,
                effect.second_moderator_probe_index,
                effect.second_moderator_value,
            )
            || second_probe_rows
                .insert(
                    effect.second_moderator_probe_index,
                    (
                        effect.second_moderator_probe_kind,
                        effect.second_moderator_value,
                    ),
                )
                .is_some()
        {
            errors.push(format!(
                "{item} authority, identity, or fixed probe is invalid"
            ));
        }
        validate_general_sem_estimate(errors, &effect.value, &format!("{item}.value"));
    }
    validate_complete_probe_axis(
        errors,
        &second_probe_rows,
        &format!("{context}.conditional_interaction_effects"),
    );
    let mut slope_grid = BTreeMap::new();
    for (index, effect) in results.three_way_simple_slopes.iter().enumerate() {
        let item = format!("{context}.simple_slopes[{index}]");
        target_ids.push(effect.effect_id.clone());
        for (name, id) in [
            ("effect_id", effect.effect_id.as_str()),
            ("interaction_id", effect.interaction_id.as_str()),
            ("focal_relation_id", effect.focal_relation_id.as_str()),
            ("first_moderator_id", effect.first_moderator_id.as_str()),
            ("second_moderator_id", effect.second_moderator_id.as_str()),
        ] {
            require_stable_id(errors, id, &format!("{item}.{name}"));
        }
        validate_general_sem_trace(
            errors,
            &effect.trace,
            &provenance.model_id,
            document_capability_ids,
            &format!("{item}.trace"),
        );
        if effect.trace.capability_cell != point_cell
            || interaction_id.is_some_and(|id| effect.interaction_id != id)
            || authority.is_some_and(|row| {
                effect.focal_relation_id != row.focal_relation_id
                    || effect.first_moderator_id != row.operand_ids[1]
                    || effect.second_moderator_id != row.operand_ids[2]
            })
            || effect.effect_id
                != format!(
                    "three_way_simple_x:{}:w{}:z{}",
                    effect.interaction_id, effect.first_probe_index, effect.second_probe_index
                )
            || !probe_value_matches_kind(
                effect.first_moderator_probe_kind,
                effect.first_probe_index,
                effect.first_moderator_value,
            )
            || !probe_value_matches_kind(
                effect.second_moderator_probe_kind,
                effect.second_probe_index,
                effect.second_moderator_value,
            )
            || slope_grid
                .insert(
                    (effect.first_probe_index, effect.second_probe_index),
                    (
                        effect.first_moderator_probe_kind,
                        effect.first_moderator_value,
                        effect.second_moderator_probe_kind,
                        effect.second_moderator_value,
                    ),
                )
                .is_some()
        {
            errors.push(format!(
                "{item} authority, identity, or fixed probe is invalid"
            ));
        }
        validate_general_sem_estimate(errors, &effect.value, &format!("{item}.value"));
    }
    let first_probe_rows = slope_grid
        .iter()
        .map(|((first, _), (_, value, _, _))| (*first, *value))
        .collect::<BTreeMap<_, _>>();
    let first_probe_kind = slope_grid.values().next().map(|(kind, _, _, _)| *kind);
    let second_slope_probe_rows = slope_grid
        .iter()
        .map(|((_, second), (_, _, _, value))| (*second, *value))
        .collect::<BTreeMap<_, _>>();
    let second_slope_probe_kind = slope_grid.values().next().map(|(_, _, kind, _)| *kind);
    if !probe_axis_is_complete(&first_probe_rows, first_probe_kind)
        || !probe_axis_is_complete(&second_slope_probe_rows, second_slope_probe_kind)
        || slope_grid.len() != first_probe_rows.len() * second_slope_probe_rows.len()
        || slope_grid.values().any(|(first_kind, _, second_kind, _)| {
            Some(*first_kind) != first_probe_kind || Some(*second_kind) != second_slope_probe_kind
        })
    {
        errors.push(format!(
            "{context}.simple_slopes must contain one complete fixed W-by-Z probe grid"
        ));
    }
    target_ids.sort();
    let inferred = results
        .three_way_interaction_effects
        .iter()
        .any(|row| general_sem_estimate_has_inference(&row.scientific_rescaled_delta))
        || results
            .three_way_conditional_interaction_effects
            .iter()
            .any(|row| general_sem_estimate_has_inference(&row.value))
        || results
            .three_way_simple_slopes
            .iter()
            .any(|row| general_sem_estimate_has_inference(&row.value));
    let Some(receipt) = &results.three_way_moderation_bootstrap_receipt else {
        if inferred {
            errors.push(format!(
                "{context} inference fields require the shared bootstrap receipt"
            ));
        }
        return;
    };
    if receipt.capability_cell
        != crate::pls_general_three_way_moderation_bootstrap_capability_cell_v1()
        || receipt.method_version
            != GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
        || receipt.point_method_version
            != GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1
        || receipt.resampling_operation_version
            != GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
        || receipt.resampling_stream_version
            != GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1
        || receipt.quantile_method_version != GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1
        || receipt.standard_error_method_version
            != GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1
        || receipt.summation_method_version != GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1
        || receipt.p_value_method_version
            != GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1
        || receipt.failure_policy_version != GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1
        || receipt.sign_alignment_method_version != "sampled_original_construct_score_covariance_v1"
        || receipt.product_scale_version != GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
        || receipt.probe_policy_version != GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1
        || receipt.target_ids != target_ids
        || receipt.target_identity_set_sha256 != crate::sha256_serialized(&target_ids)
        || !receipt.complete_model_reestimated_per_replicate
        || !receipt.shared_stage_one_reestimated_per_replicate
        || !receipt.score_vectors_sign_aligned_before_products
        || !receipt.all_lower_order_and_three_way_products_recomputed_per_replicate
        || !receipt.joint_stage_two_reestimated_per_replicate
        || !receipt.complete_joint_point_contract_validated_per_replicate
        || !receipt.all_three_way_targets_share_one_replicate_ledger
    {
        errors.push(format!(
            "{context} shared bootstrap receipt differs from the exact v1 contract"
        ));
    }
    validate_capability_reference(
        errors,
        &receipt.capability_cell,
        &format!("{context}.receipt.capability_cell"),
    );
    let receipt_identity = capability_cell_reference_identity_v2(&receipt.capability_cell);
    if document_capability_ids.is_none_or(|ids| !ids.contains(&receipt_identity)) {
        errors.push(format!(
            "{context}.receipt.capability_cell must be declared by document capability_cells"
        ));
    }
    let dependency_ids = validate_capability_set(
        errors,
        &receipt.capability_dependencies,
        &format!("{context}.receipt.capability_dependencies"),
    );
    let mut expected_dependencies = vec![
        crate::RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        point_cell,
    ];
    expected_dependencies.sort_by_key(capability_cell_reference_identity_v2);
    if dependency_ids
        != expected_dependencies
            .iter()
            .map(capability_cell_reference_identity_v2)
            .collect::<Vec<_>>()
    {
        errors.push(format!(
            "{context}.receipt capability dependencies must be base PLS plus three-way point"
        ));
    }
    if let Some(document_ids) = document_capability_ids {
        for dependency_id in &dependency_ids {
            if !document_ids.contains(dependency_id) {
                errors.push(format!(
                    "{context}.receipt dependency {dependency_id} is undeclared"
                ));
            }
        }
    }
    if receipt.resamples_usable as usize + receipt.failed_replicates.len()
        != receipt.resamples_requested as usize
        || receipt.resamples_usable < receipt.minimum_usable_resamples
        || receipt.minimum_usable_resamples
            != ((f64::from(receipt.resamples_requested) * 0.9).ceil() as u32).max(2)
        || !(2..=10_000).contains(&receipt.resamples_requested)
        || !(1..=64).contains(&receipt.workers)
        || receipt.interval != CanonicalGeneralSemBootstrapIntervalV1::PercentileType7
        || receipt.tail != CanonicalGeneralSemInferenceTailV1::TwoSided
        || !receipt.confidence_level.is_finite()
        || !(0.0..1.0).contains(&receipt.confidence_level)
    {
        errors.push(format!(
            "{context}.receipt failure ledger contradicts its counts"
        ));
    }
    for (name, digest) in [
        (
            "compiled_plan_sha256",
            receipt.compiled_plan_sha256.as_str(),
        ),
        (
            "general_sem_config_sha256",
            receipt.general_sem_config_sha256.as_str(),
        ),
        (
            "model_scientific_sha256",
            receipt.model_scientific_sha256.as_str(),
        ),
        (
            "stage_one_model_scientific_sha256",
            receipt.stage_one_model_scientific_sha256.as_str(),
        ),
        (
            "complete_case_frame_sha256",
            receipt.complete_case_frame_sha256.as_str(),
        ),
        (
            "usable_replicate_indices_sha256",
            receipt.usable_replicate_indices_sha256.as_str(),
        ),
        (
            "target_identity_set_sha256",
            receipt.target_identity_set_sha256.as_str(),
        ),
    ] {
        if !is_lowercase_sha256(digest) {
            errors.push(format!(
                "{context}.receipt.{name} must be a lowercase SHA-256"
            ));
        }
    }
    if receipt.model_scientific_sha256 != provenance.model_digest
        || receipt.source_dataset_fingerprint != provenance.dataset_fingerprint
        || receipt.seed.parse::<i64>().ok() != provenance.seed
        || i64::from(receipt.workers) != provenance.workers
    {
        errors.push(format!(
            "{context}.receipt model, dataset, seed, or workers differ from provenance"
        ));
    }
    let mut previous_failure = None;
    let mut failed_indices = BTreeSet::new();
    for failure in &receipt.failed_replicates {
        if failure.replicate_index >= receipt.resamples_requested
            || previous_failure.is_some_and(|previous| previous >= failure.replicate_index)
            || failure.message.trim().is_empty()
        {
            errors.push(format!(
                "{context}.receipt.failed_replicates is not canonical"
            ));
        }
        previous_failure = Some(failure.replicate_index);
        failed_indices.insert(failure.replicate_index);
    }
    let usable_indices = (0..receipt.resamples_requested)
        .filter(|index| !failed_indices.contains(index))
        .collect::<Vec<_>>();
    if crate::sha256_serialized(&usable_indices) != receipt.usable_replicate_indices_sha256 {
        errors.push(format!(
            "{context}.receipt usable-replicate digest contradicts its failure ledger"
        ));
    }
    let all_complete = results
        .three_way_interaction_effects
        .iter()
        .all(|row| general_sem_estimate_has_complete_inference(&row.scientific_rescaled_delta))
        && results
            .three_way_conditional_interaction_effects
            .iter()
            .all(|row| general_sem_estimate_has_complete_inference(&row.value))
        && results
            .three_way_simple_slopes
            .iter()
            .all(|row| general_sem_estimate_has_complete_inference(&row.value));
    if !inferred || !all_complete {
        errors.push(format!(
            "{context} bootstrap receipt requires complete inferred target rows"
        ));
    }
    for (target_id, value) in results
        .three_way_interaction_effects
        .iter()
        .map(|row| (row.effect_id.as_str(), &row.scientific_rescaled_delta))
        .chain(
            results
                .three_way_conditional_interaction_effects
                .iter()
                .map(|row| (row.effect_id.as_str(), &row.value)),
        )
        .chain(
            results
                .three_way_simple_slopes
                .iter()
                .map(|row| (row.effect_id.as_str(), &row.value)),
        )
    {
        if value.bootstrap_usable_replicates != Some(receipt.resamples_usable) {
            errors.push(format!(
                "{context} target {target_id} usable count differs from the shared receipt"
            ));
        }
        if let (Some(exceedances), Some(p_value)) =
            (value.bootstrap_two_sided_exceedances, value.p_value)
        {
            let expected = f64::from(exceedances + 1) / f64::from(receipt.resamples_usable + 1);
            if exceedances > receipt.resamples_usable || !approximately_equal(p_value, expected) {
                errors.push(format!("{context} target {target_id} contradicts the shared plus-one probability ledger"));
            }
        }
    }
}

fn probe_value_matches_kind(
    kind: CanonicalThreeWayModeratorProbeKindV1,
    index: u32,
    value: f64,
) -> bool {
    let expected: Option<f64> = match kind {
        CanonicalThreeWayModeratorProbeKindV1::ContinuousStandardized => {
            [-1.0, 0.0, 1.0].get(index as usize).copied()
        }
        CanonicalThreeWayModeratorProbeKindV1::BinaryZeroOne => {
            [0.0, 1.0].get(index as usize).copied()
        }
    };
    expected.is_some_and(|expected| value.to_bits() == expected.to_bits())
}

fn validate_complete_probe_axis(
    errors: &mut Vec<String>,
    rows: &BTreeMap<u32, (CanonicalThreeWayModeratorProbeKindV1, f64)>,
    context: &str,
) {
    let kind = rows.values().next().map(|(kind, _)| *kind);
    let values = rows
        .iter()
        .map(|(index, (_, value))| (*index, *value))
        .collect::<BTreeMap<_, _>>();
    if rows.values().any(|(candidate, _)| Some(*candidate) != kind)
        || !probe_axis_is_complete(&values, kind)
    {
        errors.push(format!(
            "{context} must contain one complete fixed probe axis"
        ));
    }
}

fn probe_axis_is_complete(
    rows: &BTreeMap<u32, f64>,
    kind: Option<CanonicalThreeWayModeratorProbeKindV1>,
) -> bool {
    let expected: &[f64] = match kind {
        Some(CanonicalThreeWayModeratorProbeKindV1::ContinuousStandardized) => &[-1.0, 0.0, 1.0],
        Some(CanonicalThreeWayModeratorProbeKindV1::BinaryZeroOne) => &[0.0, 1.0],
        None => return false,
    };
    rows.len() == expected.len()
        && expected.iter().enumerate().all(|(index, expected)| {
            rows.get(&(index as u32))
                .is_some_and(|value| value.to_bits() == expected.to_bits())
        })
}

fn validate_general_sem_results_v1(
    errors: &mut Vec<String>,
    results: &CanonicalGeneralSemResultsV1,
    provenance: &CanonicalResultProvenanceV2,
    document_capability_ids: Option<&HashSet<String>>,
) {
    let context = "general_sem_results";
    let document_model_id = provenance.model_id.as_str();
    if results.schema_version != CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION {
        errors.push(format!(
            "{context}.schema_version must equal {CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION}"
        ));
    }
    validate_general_sem_inference_receipt_v1(errors, results, provenance, document_capability_ids);
    validate_three_way_moderation_results_v1(errors, results, provenance, document_capability_ids);
    validate_hoc_inference_receipt_v1(errors, results, provenance, document_capability_ids);
    if results.specific_indirect_effects.is_empty()
        && results.aggregate_effects.is_empty()
        && results.joint_stage_structural_coefficients.is_empty()
        && results.interaction_effects.is_empty()
        && results.three_way_interaction_effects.is_empty()
        && results.three_way_conditional_interaction_effects.is_empty()
        && results.three_way_simple_slopes.is_empty()
        && results.three_way_moderation_bootstrap_receipt.is_none()
        && results.conditional_effect_probes.is_empty()
        && results.conditional_effects.is_empty()
        && results.conditional_indirect_effects.is_empty()
        && results.moderated_mediation_indices.is_empty()
        && results.interaction_plots.is_empty()
        && results.higher_order_stages.is_empty()
        && results.cbsem_parameters.is_empty()
        && results.cbsem_fit.is_empty()
        && results.identification_diagnostics.is_empty()
        && results.cbsem_bootstrap_receipt.is_none()
        && results.cbsem_bootstrap_inference.is_empty()
    {
        errors.push(format!(
            "{context} must contain at least one typed result section"
        ));
    }

    require_canonical_stable_ids(
        errors,
        results
            .joint_stage_structural_coefficients
            .iter()
            .map(|item| item.relation_id.as_str()),
        &format!("{context}.joint_stage_structural_coefficients"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .interaction_effects
            .iter()
            .map(|item| item.effect_id.as_str()),
        &format!("{context}.interaction_effects"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .three_way_interaction_effects
            .iter()
            .map(|item| item.effect_id.as_str()),
        &format!("{context}.three_way_interaction_effects"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .three_way_conditional_interaction_effects
            .iter()
            .map(|item| item.effect_id.as_str()),
        &format!("{context}.three_way_conditional_interaction_effects"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .three_way_simple_slopes
            .iter()
            .map(|item| item.effect_id.as_str()),
        &format!("{context}.three_way_simple_slopes"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .specific_indirect_effects
            .iter()
            .map(|item| item.effect_id.as_str()),
        &format!("{context}.specific_indirect_effects"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .aggregate_effects
            .iter()
            .map(|item| item.effect_id.as_str()),
        &format!("{context}.aggregate_effects"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .conditional_effect_probes
            .iter()
            .map(|item| item.probe_id.as_str()),
        &format!("{context}.conditional_effect_probes"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .conditional_effects
            .iter()
            .map(|item| item.effect_id.as_str()),
        &format!("{context}.conditional_effects"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .conditional_indirect_effects
            .iter()
            .map(|item| item.effect_id.as_str()),
        &format!("{context}.conditional_indirect_effects"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .moderated_mediation_indices
            .iter()
            .map(|item| item.effect_id.as_str()),
        &format!("{context}.moderated_mediation_indices"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .interaction_plots
            .iter()
            .map(|item| item.plot_id.as_str()),
        &format!("{context}.interaction_plots"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .higher_order_stages
            .iter()
            .map(|item| item.stage_id.as_str()),
        &format!("{context}.higher_order_stages"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .cbsem_parameters
            .iter()
            .map(|item| item.parameter_id.as_str()),
        &format!("{context}.cbsem_parameters"),
    );
    require_canonical_stable_ids(
        errors,
        results.cbsem_fit.iter().map(|item| item.fit_id.as_str()),
        &format!("{context}.cbsem_fit"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .identification_diagnostics
            .iter()
            .map(|item| item.diagnostic_id.as_str()),
        &format!("{context}.identification_diagnostics"),
    );
    require_canonical_stable_ids(
        errors,
        results
            .cbsem_bootstrap_inference
            .iter()
            .map(|item| item.parameter_id.as_str()),
        &format!("{context}.cbsem_bootstrap_inference"),
    );

    let mut effect_ids = BTreeSet::new();
    let mut estimand_ids = BTreeSet::new();
    let mut specific_signatures = BTreeSet::new();
    for (index, effect) in results.specific_indirect_effects.iter().enumerate() {
        let item_context = format!("{context}.specific_indirect_effects[{index}]");
        if !effect_ids.insert(effect.effect_id.as_str()) {
            errors.push(format!(
                "{item_context}.effect_id is duplicated across effect sections"
            ));
        }
        require_stable_id(
            errors,
            &effect.estimand_id,
            &format!("{item_context}.estimand_id"),
        );
        if !estimand_ids.insert(effect.estimand_id.as_str()) {
            errors.push(format!(
                "{item_context}.estimand_id is duplicated across effect sections"
            ));
        }
        require_stable_id(
            errors,
            &effect.source_id,
            &format!("{item_context}.source_id"),
        );
        require_stable_id(
            errors,
            &effect.target_id,
            &format!("{item_context}.target_id"),
        );
        if effect.source_id == effect.target_id {
            errors.push(format!(
                "{item_context} requires distinct source_id and target_id"
            ));
        }
        validate_general_sem_trace(
            errors,
            &effect.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        if effect.ordered_relation_ids.len() < 2 {
            errors.push(format!(
                "{item_context}.ordered_relation_ids requires at least two relations"
            ));
        }
        require_unique_ids(
            errors,
            effect.ordered_relation_ids.iter().map(String::as_str),
            &format!("{item_context}.ordered_relation_ids"),
        );
        if crate::specific_directed_path_identity_v1(&effect.ordered_relation_ids)
            != effect.effect_id
        {
            errors.push(format!(
                "{item_context}.effect_id must equal the canonical ordered relation-path identity"
            ));
        }
        if !specific_signatures.insert(effect.ordered_relation_ids.join("\0")) {
            errors.push(format!(
                "{item_context} duplicates another specific indirect path"
            ));
        }
        validate_general_sem_estimate(errors, &effect.value, &format!("{item_context}.value"));
    }

    let mut aggregate_signatures = BTreeSet::new();
    for (index, effect) in results.aggregate_effects.iter().enumerate() {
        let item_context = format!("{context}.aggregate_effects[{index}]");
        if !effect_ids.insert(effect.effect_id.as_str()) {
            errors.push(format!(
                "{item_context}.effect_id is duplicated across effect sections"
            ));
        }
        require_stable_id(
            errors,
            &effect.estimand_id,
            &format!("{item_context}.estimand_id"),
        );
        if !estimand_ids.insert(effect.estimand_id.as_str()) {
            errors.push(format!(
                "{item_context}.estimand_id is duplicated across effect sections"
            ));
        }
        validate_general_sem_trace(
            errors,
            &effect.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        require_stable_id(
            errors,
            &effect.source_id,
            &format!("{item_context}.source_id"),
        );
        require_stable_id(
            errors,
            &effect.target_id,
            &format!("{item_context}.target_id"),
        );
        if effect.source_id == effect.target_id {
            errors.push(format!(
                "{item_context} requires distinct source_id and target_id"
            ));
        }
        if effect.effect_id != effect.estimand_id {
            errors.push(format!(
                "{item_context}.effect_id must equal estimand_id for aggregate effects"
            ));
        }
        require_canonical_stable_ids(
            errors,
            effect.direct_relation_ids.iter().map(String::as_str),
            &format!("{item_context}.direct_relation_ids"),
        );
        require_canonical_stable_ids(
            errors,
            effect
                .contributing_path_identities
                .iter()
                .map(String::as_str),
            &format!("{item_context}.contributing_path_identities"),
        );
        let kind = match effect.kind {
            CanonicalAggregateEffectKindV1::TotalIndirect => {
                if !effect.direct_relation_ids.is_empty() {
                    errors.push(format!(
                        "{item_context}.direct_relation_ids must be empty for total indirect effects"
                    ));
                }
                if effect.contributing_path_identities.is_empty() {
                    errors.push(format!(
                        "{item_context}.contributing_path_identities must not be empty"
                    ));
                }
                "total_indirect"
            }
            CanonicalAggregateEffectKindV1::TotalEffect => {
                if effect.direct_relation_ids.is_empty()
                    && effect.contributing_path_identities.is_empty()
                {
                    errors.push(format!(
                        "{item_context} must identify at least one direct relation or indirect path"
                    ));
                }
                "total_effect"
            }
        };
        if !aggregate_signatures.insert(format!(
            "{kind}\0{}\0{}",
            effect.source_id, effect.target_id
        )) {
            errors.push(format!(
                "{item_context} duplicates another aggregate scientific effect"
            ));
        }
        validate_general_sem_estimate(errors, &effect.value, &format!("{item_context}.value"));
    }

    if results.interaction_effects.is_empty()
        != results.joint_stage_structural_coefficients.is_empty()
    {
        errors.push(format!(
            "{context}.joint_stage_structural_coefficients and interaction_effects must both be present for the exact joint-stage moderation cell"
        ));
    }

    let three_way_joint_stage = !results.three_way_interaction_effects.is_empty();
    let moderation_cell = if three_way_joint_stage {
        crate::pls_general_three_way_moderation_point_capability_cell_v1()
    } else {
        crate::pls_general_multiple_moderation_point_capability_cell_v1()
    };
    let joint_stage_method_version = if three_way_joint_stage {
        GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1
    } else {
        GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
    };
    let pairwise_conditioning_policy_version = if three_way_joint_stage {
        GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1
    } else {
        GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1
    };
    let mut joint_stage_parameter_ids = BTreeSet::new();
    let mut joint_stage_relation_ids = BTreeSet::new();
    for (index, coefficient) in results
        .joint_stage_structural_coefficients
        .iter()
        .enumerate()
    {
        let item_context = format!("{context}.joint_stage_structural_coefficients[{index}]");
        for (name, id) in [
            ("relation_id", coefficient.relation_id.as_str()),
            ("parameter_id", coefficient.parameter_id.as_str()),
            ("source_id", coefficient.source_id.as_str()),
            ("target_id", coefficient.target_id.as_str()),
            ("method_version", coefficient.method_version.as_str()),
        ] {
            require_stable_id(errors, id, &format!("{item_context}.{name}"));
        }
        validate_general_sem_trace(
            errors,
            &coefficient.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        if coefficient.trace.capability_cell != moderation_cell {
            errors.push(format!(
                "{item_context}.trace.capability_cell must equal the exact joint-stage moderation point option cell"
            ));
        }
        if coefficient.source_id == coefficient.target_id {
            errors.push(format!(
                "{item_context} requires distinct source_id and target_id"
            ));
        }
        if !joint_stage_relation_ids.insert(coefficient.relation_id.as_str()) {
            errors.push(format!("{item_context}.relation_id is duplicated"));
        }
        if !joint_stage_parameter_ids.insert(coefficient.parameter_id.as_str()) {
            errors.push(format!("{item_context}.parameter_id is duplicated"));
        }
        if coefficient.method_version != joint_stage_method_version {
            errors.push(format!(
                "{item_context}.method_version must equal {joint_stage_method_version}"
            ));
        }
        validate_general_sem_estimate(
            errors,
            &coefficient.estimate,
            &format!("{item_context}.estimate"),
        );
        if general_sem_estimate_has_inference(&coefficient.estimate) {
            errors.push(format!(
                "{item_context}.estimate must contain point estimation only"
            ));
        }
    }

    let mut interaction_effects_by_id = BTreeMap::new();
    let mut interaction_ids = BTreeSet::new();
    let mut interaction_relation_ids = BTreeSet::new();
    let mut interaction_parameter_ids = BTreeSet::new();
    let mut generated_product_column_ids = BTreeSet::new();
    let mut stage_one_model_digests = BTreeSet::new();
    for (index, effect) in results.interaction_effects.iter().enumerate() {
        let item_context = format!("{context}.interaction_effects[{index}]");
        if !effect_ids.insert(effect.effect_id.as_str()) {
            errors.push(format!(
                "{item_context}.effect_id is duplicated across effect sections"
            ));
        }
        for (name, id) in [
            ("interaction_id", effect.interaction_id.as_str()),
            ("focal_relation_id", effect.focal_relation_id.as_str()),
            (
                "interaction_effect_relation_id",
                effect.interaction_effect_relation_id.as_str(),
            ),
            (
                "interaction_effect_parameter_id",
                effect.interaction_effect_parameter_id.as_str(),
            ),
            ("focal_predictor_id", effect.focal_predictor_id.as_str()),
            ("moderator_id", effect.moderator_id.as_str()),
            ("outcome_id", effect.outcome_id.as_str()),
            (
                "generated_product_column_id",
                effect.generated_product_column_id.as_str(),
            ),
            ("method_version", effect.method_version.as_str()),
            (
                "product_scale_version",
                effect.product_scale_version.as_str(),
            ),
            (
                "hierarchy_policy_version",
                effect.hierarchy_policy_version.as_str(),
            ),
            (
                "conditioning_policy_version",
                effect.conditioning_policy_version.as_str(),
            ),
        ] {
            require_stable_id(errors, id, &format!("{item_context}.{name}"));
        }
        if !is_lowercase_sha256(&effect.stage_one_model_scientific_sha256) {
            errors.push(format!(
                "{item_context}.stage_one_model_scientific_sha256 must be a lowercase SHA-256"
            ));
        }
        if effect.stage_one_model_scientific_sha256 == provenance.model_digest {
            errors.push(format!(
                "{item_context}.stage_one_model_scientific_sha256 must identify the projected interaction-free scoring model"
            ));
        }
        stage_one_model_digests.insert(effect.stage_one_model_scientific_sha256.as_str());
        validate_general_sem_trace(
            errors,
            &effect.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        if effect.trace.capability_cell != moderation_cell {
            errors.push(format!(
                "{item_context}.trace.capability_cell must equal the exact joint-stage moderation point option cell"
            ));
        }
        if effect.effect_id != effect.interaction_effect_relation_id {
            errors.push(format!(
                "{item_context}.effect_id must equal interaction_effect_relation_id"
            ));
        }
        if !interaction_ids.insert(effect.interaction_id.as_str()) {
            errors.push(format!("{item_context}.interaction_id is duplicated"));
        }
        if !interaction_relation_ids.insert(effect.interaction_effect_relation_id.as_str()) {
            errors.push(format!(
                "{item_context}.interaction_effect_relation_id is duplicated"
            ));
        }
        if joint_stage_relation_ids.contains(effect.interaction_effect_relation_id.as_str()) {
            errors.push(format!(
                "{item_context}.interaction_effect_relation_id must not appear in the ordinary joint-stage structural ledger"
            ));
        }
        if !interaction_parameter_ids.insert(effect.interaction_effect_parameter_id.as_str()) {
            errors.push(format!(
                "{item_context}.interaction_effect_parameter_id is duplicated"
            ));
        }
        if !generated_product_column_ids.insert(effect.generated_product_column_id.as_str()) {
            errors.push(format!(
                "{item_context}.generated_product_column_id is duplicated"
            ));
        }
        if effect.focal_predictor_id == effect.moderator_id
            || effect.focal_predictor_id == effect.outcome_id
            || effect.moderator_id == effect.outcome_id
        {
            errors.push(format!(
                "{item_context} requires distinct focal, moderator, and outcome identities"
            ));
        }
        if effect.observation_count < 3 {
            errors.push(format!(
                "{item_context}.observation_count must be at least three"
            ));
        }
        if effect.method_version != joint_stage_method_version {
            errors.push(format!(
                "{item_context}.method_version must equal {joint_stage_method_version}"
            ));
        }
        if effect.product_scale_version != GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1 {
            errors.push(format!(
                "{item_context}.product_scale_version must equal {GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1}"
            ));
        }
        if effect.hierarchy_policy_version != GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1 {
            errors.push(format!(
                "{item_context}.hierarchy_policy_version must equal {GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1}"
            ));
        }
        if effect.conditioning_policy_version != pairwise_conditioning_policy_version {
            errors.push(format!(
                "{item_context}.conditioning_policy_version must equal {pairwise_conditioning_policy_version}"
            ));
        }
        if !effect.unstandardized_product_mean.is_finite() {
            errors.push(format!(
                "{item_context}.unstandardized_product_mean must be finite"
            ));
        }
        if !effect
            .unstandardized_product_sample_standard_deviation
            .is_finite()
            || effect.unstandardized_product_sample_standard_deviation <= f64::EPSILON
        {
            errors.push(format!(
                "{item_context}.unstandardized_product_sample_standard_deviation must be finite and positive"
            ));
        }
        validate_general_sem_estimate(
            errors,
            &effect.standardized_product_coefficient,
            &format!("{item_context}.standardized_product_coefficient"),
        );
        validate_general_sem_estimate(
            errors,
            &effect.scientific_rescaled_gamma,
            &format!("{item_context}.scientific_rescaled_gamma"),
        );
        let expected_gamma = effect.standardized_product_coefficient.estimate
            / effect.unstandardized_product_sample_standard_deviation;
        if !approximately_equal(expected_gamma, effect.scientific_rescaled_gamma.estimate) {
            errors.push(format!(
                "{item_context}.scientific_rescaled_gamma must equal standardized_product_coefficient divided by the product sample standard deviation"
            ));
        }
        interaction_effects_by_id.insert(effect.effect_id.as_str(), effect);
    }
    if stage_one_model_digests.len() > 1 {
        errors.push(format!(
            "{context}.interaction_effects must share one stage-one model scientific digest"
        ));
    }

    validate_canonical_moderated_mediation_results_v1(
        errors,
        results,
        document_model_id,
        document_capability_ids,
        &mut effect_ids,
    );

    for (index, probe) in results.conditional_effect_probes.iter().enumerate() {
        let item_context = format!("{context}.conditional_effect_probes[{index}]");
        validate_general_sem_trace(
            errors,
            &probe.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        require_stable_id(
            errors,
            &probe.moderator_id,
            &format!("{item_context}.moderator_id"),
        );
        match &probe.values {
            CanonicalConditionalProbeValuesResultV1::DataDerivedMeanPlusMinusOneSd {
                mean,
                standard_deviation,
            } => {
                if !mean.is_finite() {
                    errors.push(format!("{item_context}.values.mean must be finite"));
                }
                if !standard_deviation.is_finite() || *standard_deviation < 0.0 {
                    errors.push(format!(
                        "{item_context}.values.standard_deviation must be finite and nonnegative"
                    ));
                }
            }
            CanonicalConditionalProbeValuesResultV1::Explicit { values } => {
                if values.is_empty() {
                    errors.push(format!("{item_context}.values.values must not be empty"));
                }
                for (value_index, value) in values.iter().enumerate() {
                    if !value.is_finite() {
                        errors.push(format!(
                            "{item_context}.values.values[{value_index}] must be finite"
                        ));
                    }
                }
                for (value_index, pair) in values.windows(2).enumerate() {
                    if pair[0].partial_cmp(&pair[1]) != Some(std::cmp::Ordering::Less) {
                        errors.push(format!(
                            "{item_context}.values.values must be strictly increasing at indices {value_index} and {}",
                            value_index + 1
                        ));
                    }
                }
            }
        }
    }

    let probes = results
        .conditional_effect_probes
        .iter()
        .map(|probe| (probe.probe_id.as_str(), probe))
        .collect::<BTreeMap<_, _>>();
    let mut conditional_signatures = BTreeSet::new();
    let mut interaction_conditional_indices = BTreeMap::<String, Vec<u32>>::new();
    for (index, effect) in results.conditional_effects.iter().enumerate() {
        let item_context = format!("{context}.conditional_effects[{index}]");
        if !effect_ids.insert(effect.effect_id.as_str()) {
            errors.push(format!(
                "{item_context}.effect_id is duplicated across effect sections"
            ));
        }
        for (name, id) in [
            ("estimand_id", effect.estimand_id.as_str()),
            ("interaction_id", effect.interaction_id.as_str()),
            ("focal_relation_id", effect.focal_relation_id.as_str()),
            ("probe_id", effect.probe_id.as_str()),
            ("moderator_id", effect.moderator_id.as_str()),
        ] {
            require_stable_id(errors, id, &format!("{item_context}.{name}"));
        }
        match effect.interaction_effect_id.as_deref() {
            Some(effect_id) => {
                require_stable_id(
                    errors,
                    effect_id,
                    &format!("{item_context}.interaction_effect_id"),
                );
                match interaction_effects_by_id.get(effect_id) {
                    None => errors.push(format!(
                        "{item_context}.interaction_effect_id references a missing interaction effect"
                    )),
                    Some(interaction_effect)
                        if interaction_effect.interaction_id != effect.interaction_id
                            || interaction_effect.focal_relation_id != effect.focal_relation_id
                            || interaction_effect.moderator_id != effect.moderator_id
                            || interaction_effect.trace.capability_cell
                                != effect.trace.capability_cell =>
                    {
                        errors.push(format!(
                            "{item_context} contradicts its interaction effect authority"
                        ));
                    }
                    _ => {}
                }
            }
            None if !results.interaction_effects.is_empty() => errors.push(format!(
                "{item_context}.interaction_effect_id is required when interaction_effects are present"
            )),
            None => {}
        }
        let is_interaction_linked = effect
            .interaction_effect_id
            .as_deref()
            .is_some_and(|effect_id| interaction_effects_by_id.contains_key(effect_id));
        if let Some(effect_id) = effect
            .interaction_effect_id
            .as_deref()
            .filter(|effect_id| interaction_effects_by_id.contains_key(*effect_id))
        {
            interaction_conditional_indices
                .entry(effect_id.to_string())
                .or_default()
                .push(effect.probe_value_index);
        }
        validate_general_sem_trace(
            errors,
            &effect.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        if !effect.moderator_value.is_finite() {
            errors.push(format!("{item_context}.moderator_value must be finite"));
        }
        match probes.get(effect.probe_id.as_str()) {
            None => errors.push(format!(
                "{item_context}.probe_id references a missing conditional-effect probe"
            )),
            Some(probe) => {
                if effect.moderator_id != probe.moderator_id {
                    errors.push(format!("{item_context}.moderator_id contradicts its probe"));
                }
                if effect.trace.capability_cell != probe.trace.capability_cell {
                    errors.push(format!(
                        "{item_context}.trace contradicts its probe authority"
                    ));
                }
                if is_interaction_linked && !is_frozen_standardized_three_point_probe(probe) {
                    errors.push(format!(
                        "{item_context}.probe_id must use the frozen standardized -1/0/+1 interaction policy"
                    ));
                }
                match conditional_probe_value(probe, effect.probe_value_index) {
                    None => errors.push(format!(
                        "{item_context}.probe_value_index is outside its probe"
                    )),
                    Some(expected) if !approximately_equal(effect.moderator_value, expected) => {
                        errors.push(format!(
                            "{item_context}.moderator_value contradicts its probe value"
                        ));
                    }
                    _ => {}
                }
            }
        }
        if !conditional_signatures.insert(format!(
            "{}\0{}\0{}\0{}\0{}",
            effect.estimand_id,
            effect.interaction_id,
            effect.focal_relation_id,
            effect.probe_id,
            effect.probe_value_index
        )) {
            errors.push(format!(
                "{item_context} duplicates another conditional scientific effect"
            ));
        }
        validate_general_sem_estimate(errors, &effect.value, &format!("{item_context}.value"));
    }

    let mut interaction_plot_counts = BTreeMap::<String, usize>::new();
    for (index, plot) in results.interaction_plots.iter().enumerate() {
        let item_context = format!("{context}.interaction_plots[{index}]");
        validate_general_sem_trace(
            errors,
            &plot.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        for (name, id) in [
            ("interaction_id", plot.interaction_id.as_str()),
            ("focal_relation_id", plot.focal_relation_id.as_str()),
            ("focal_predictor_id", plot.focal_predictor_id.as_str()),
            ("moderator_id", plot.moderator_id.as_str()),
            ("outcome_id", plot.outcome_id.as_str()),
        ] {
            require_stable_id(errors, id, &format!("{item_context}.{name}"));
        }
        match plot.interaction_effect_id.as_deref() {
            Some(effect_id) => {
                require_stable_id(
                    errors,
                    effect_id,
                    &format!("{item_context}.interaction_effect_id"),
                );
                match interaction_effects_by_id.get(effect_id) {
                    None => errors.push(format!(
                        "{item_context}.interaction_effect_id references a missing interaction effect"
                    )),
                    Some(interaction_effect)
                        if interaction_effect.interaction_id != plot.interaction_id
                            || interaction_effect.focal_relation_id != plot.focal_relation_id
                            || interaction_effect.focal_predictor_id != plot.focal_predictor_id
                            || interaction_effect.moderator_id != plot.moderator_id
                            || interaction_effect.outcome_id != plot.outcome_id
                            || interaction_effect.trace.capability_cell
                                != plot.trace.capability_cell =>
                    {
                        errors.push(format!(
                            "{item_context} contradicts its interaction effect authority"
                        ));
                    }
                    _ => {}
                }
            }
            None if !results.interaction_effects.is_empty() => errors.push(format!(
                "{item_context}.interaction_effect_id is required when interaction_effects are present"
            )),
            None => {}
        }
        let linked_interaction_effect_id = plot
            .interaction_effect_id
            .as_deref()
            .filter(|effect_id| interaction_effects_by_id.contains_key(*effect_id));
        if let Some(effect_id) = linked_interaction_effect_id {
            *interaction_plot_counts
                .entry(effect_id.to_string())
                .or_default() += 1;
            if plot.series.len() != 3 {
                errors.push(format!(
                    "{item_context}.series must contain exactly the frozen -1/0/+1 interaction probes"
                ));
            }
        }
        if plot.focal_predictor_id == plot.moderator_id
            || plot.focal_predictor_id == plot.outcome_id
            || plot.moderator_id == plot.outcome_id
        {
            errors.push(format!(
                "{item_context} requires distinct focal, moderator, and outcome identities"
            ));
        }
        if plot.series.is_empty() {
            errors.push(format!("{item_context}.series must not be empty"));
        }
        require_canonical_stable_ids(
            errors,
            plot.series.iter().map(|series| series.series_id.as_str()),
            &format!("{item_context}.series"),
        );
        let mut expected_grid: Option<Vec<f64>> = None;
        let mut linked_series_probe_indices = BTreeSet::new();
        for (series_index, series) in plot.series.iter().enumerate() {
            let series_context = format!("{item_context}.series[{series_index}]");
            require_stable_id(
                errors,
                &series.probe_id,
                &format!("{series_context}.probe_id"),
            );
            if !series.moderator_value.is_finite() {
                errors.push(format!("{series_context}.moderator_value must be finite"));
            }
            match probes.get(series.probe_id.as_str()) {
                None => errors.push(format!(
                    "{series_context}.probe_id references a missing conditional-effect probe"
                )),
                Some(probe) => {
                    if probe.moderator_id != plot.moderator_id {
                        errors.push(format!(
                            "{series_context}.probe_id uses a different moderator"
                        ));
                    }
                    if probe.trace.capability_cell != plot.trace.capability_cell {
                        errors.push(format!(
                            "{series_context}.probe_id uses a different capability authority"
                        ));
                    }
                    if linked_interaction_effect_id.is_some()
                        && !is_frozen_standardized_three_point_probe(probe)
                    {
                        errors.push(format!(
                            "{series_context}.probe_id must use the frozen standardized -1/0/+1 interaction policy"
                        ));
                    }
                    match conditional_probe_value(probe, series.probe_value_index) {
                        None => errors.push(format!(
                            "{series_context}.probe_value_index is outside its probe"
                        )),
                        Some(expected)
                            if !approximately_equal(series.moderator_value, expected) =>
                        {
                            errors.push(format!(
                                "{series_context}.moderator_value contradicts its probe value"
                            ));
                        }
                        _ => {}
                    }
                }
            }
            if linked_interaction_effect_id.is_some() {
                linked_series_probe_indices.insert(series.probe_value_index);
            }
            if series.points.is_empty() {
                errors.push(format!("{series_context}.points must not be empty"));
            }
            let mut previous_focal_value = None;
            for (point_index, point) in series.points.iter().enumerate() {
                let point_context = format!("{series_context}.points[{point_index}]");
                if !point.focal_value.is_finite() {
                    errors.push(format!("{point_context}.focal_value must be finite"));
                }
                if !point.predicted_value.is_finite() {
                    errors.push(format!("{point_context}.predicted_value must be finite"));
                }
                if previous_focal_value.is_some_and(|previous| previous >= point.focal_value) {
                    errors.push(format!(
                        "{series_context}.points must use strictly increasing focal values"
                    ));
                }
                previous_focal_value = Some(point.focal_value);
                validate_general_sem_bounds(errors, point.lower, point.upper, &point_context);
            }
            let grid = series
                .points
                .iter()
                .map(|point| point.focal_value)
                .collect::<Vec<_>>();
            if let Some(expected) = &expected_grid {
                if grid.len() != expected.len()
                    || grid
                        .iter()
                        .zip(expected)
                        .any(|(left, right)| !approximately_equal(*left, *right))
                {
                    errors.push(format!(
                        "{series_context}.points must use the plot's common focal-value grid"
                    ));
                }
            } else {
                expected_grid = Some(grid);
            }
        }
        if linked_interaction_effect_id.is_some()
            && linked_series_probe_indices != BTreeSet::from([0_u32, 1, 2])
        {
            errors.push(format!(
                "{item_context}.series must cover probe indices 0, 1, and 2 exactly"
            ));
        }
    }

    for effect in &results.interaction_effects {
        if three_way_joint_stage {
            // Pairwise strong-hierarchy coefficients are point rows under the
            // three-way authority. Their conditional interpretation is owned
            // by the typed two-dimensional three-way probe grid below.
            continue;
        }
        let indices = interaction_conditional_indices
            .get(&effect.effect_id)
            .cloned()
            .unwrap_or_default();
        if indices.len() != 3
            || indices.into_iter().collect::<BTreeSet<_>>() != BTreeSet::from([0, 1, 2])
        {
            errors.push(format!(
                "{context}.interaction_effects must each have exactly three conditional rows at probe indices 0, 1, and 2"
            ));
        }
        if interaction_plot_counts.get(&effect.effect_id).copied() != Some(1) {
            errors.push(format!(
                "{context}.interaction_effects must each have exactly one cross-referenced interaction plot"
            ));
        }
    }

    let mut hoc_stage_signatures = BTreeSet::new();
    for (index, stage) in results.higher_order_stages.iter().enumerate() {
        let item_context = format!("{context}.higher_order_stages[{index}]");
        validate_general_sem_trace(
            errors,
            &stage.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        require_stable_id(
            errors,
            &stage.higher_order_construct_id,
            &format!("{item_context}.higher_order_construct_id"),
        );
        if stage.approach.is_some() != stage.measurement_type.is_some() {
            errors.push(format!(
                "{item_context}.approach and measurement_type must be present or absent together"
            ));
        }
        let expected_role = match (stage.approach.as_ref(), stage.stage_number) {
            (Some(crate::HigherOrderConstructionApproachV4::RepeatedIndicators), 1) => {
                Some(crate::CompiledPlsHocStageRoleV1::RepeatedIndicatorEstimation)
            }
            (Some(crate::HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators), 1) => {
                Some(crate::CompiledPlsHocStageRoleV1::ExtendedRepeatedIndicatorEstimation)
            }
            (Some(crate::HigherOrderConstructionApproachV4::EmbeddedTwoStage), 1) => {
                Some(crate::CompiledPlsHocStageRoleV1::EmbeddedRepeatedIndicatorEstimation)
            }
            (Some(crate::HigherOrderConstructionApproachV4::DisjointTwoStage), 1) => {
                Some(crate::CompiledPlsHocStageRoleV1::DisjointLowerOrderScoreEstimation)
            }
            (
                Some(
                    crate::HigherOrderConstructionApproachV4::EmbeddedTwoStage
                    | crate::HigherOrderConstructionApproachV4::DisjointTwoStage,
                ),
                2,
            ) => Some(crate::CompiledPlsHocStageRoleV1::HigherOrderFromLowerOrderScores),
            (None, 1) if stage.kind == CanonicalHocStageKindV1::LowerOrderScoreEstimation => {
                Some(crate::CompiledPlsHocStageRoleV1::DisjointLowerOrderScoreEstimation)
            }
            (None, 2) if stage.kind == CanonicalHocStageKindV1::HigherOrderEstimation => {
                Some(crate::CompiledPlsHocStageRoleV1::HigherOrderFromLowerOrderScores)
            }
            _ => None,
        };
        let expected_kind = expected_role.map(|role| match role {
            crate::CompiledPlsHocStageRoleV1::DisjointLowerOrderScoreEstimation
            | crate::CompiledPlsHocStageRoleV1::EmbeddedRepeatedIndicatorEstimation => {
                CanonicalHocStageKindV1::LowerOrderScoreEstimation
            }
            crate::CompiledPlsHocStageRoleV1::RepeatedIndicatorEstimation
            | crate::CompiledPlsHocStageRoleV1::ExtendedRepeatedIndicatorEstimation
            | crate::CompiledPlsHocStageRoleV1::HigherOrderFromLowerOrderScores => {
                CanonicalHocStageKindV1::HigherOrderEstimation
            }
        });
        if expected_kind != Some(stage.kind) {
            errors.push(format!(
                "{item_context}.stage_number, approach, and stage kind are inconsistent"
            ));
        }
        if let Some(receipt) = &stage.receipt {
            if receipt.receipt_version != "general_sem_pls_higher_order_point_stage_receipt_v1"
                || receipt.stage_number != stage.stage_number
                || Some(receipt.role) != expected_role
                || receipt.used_observations == 0
                || receipt.dataset_fingerprint != provenance.dataset_fingerprint
            {
                errors.push(format!(
                    "{item_context}.receipt contradicts the exact stage identity or row accounting"
                ));
            }
            for (name, digest) in [
                (
                    "projection_identity_sha256",
                    receipt.projection_identity_sha256.as_str(),
                ),
                (
                    "model_scientific_sha256",
                    receipt.model_scientific_sha256.as_str(),
                ),
                (
                    "compiled_plan_sha256",
                    receipt.compiled_plan_sha256.as_str(),
                ),
            ] {
                if !is_lowercase_sha256(digest) {
                    errors.push(format!(
                        "{item_context}.receipt.{name} must be a lowercase SHA-256"
                    ));
                }
            }
            if receipt.dataset_fingerprint.trim().is_empty() {
                errors.push(format!(
                    "{item_context}.receipt.dataset_fingerprint must be nonempty"
                ));
            }
            if let Some(generated) = &receipt.generated_score_dataset {
                if generated.receipt_version
                    != "general_sem_pls_disjoint_hoc_score_dataset_receipt_v1"
                    || generated.source_dataset_fingerprint != provenance.dataset_fingerprint
                    || generated.complete_case_row_count != receipt.used_observations
                    || generated.omitted_row_count != receipt.omitted_observations
                    || generated.generated_score_columns.is_empty()
                    || !is_lowercase_sha256(&generated.complete_case_rows_sha256)
                {
                    errors.push(format!(
                        "{item_context}.receipt.generated_score_dataset is incomplete"
                    ));
                }
                require_canonical_stable_ids(
                    errors,
                    generated
                        .generated_score_columns
                        .iter()
                        .map(|column| column.component_id.as_str()),
                    &format!(
                        "{item_context}.receipt.generated_score_dataset.generated_score_columns"
                    ),
                );
                for column in &generated.generated_score_columns {
                    require_stable_id(
                        errors,
                        &column.generated_score_variable_id,
                        &format!(
                            "{item_context}.receipt.generated_score_dataset.generated_score_variable_id"
                        ),
                    );
                    if column.observation_count != generated.complete_case_row_count
                        || !is_lowercase_sha256(&column.values_sha256)
                    {
                        errors.push(format!(
                            "{item_context}.receipt generated score row count or value digest is invalid"
                        ));
                    }
                }
            }
        }
        require_canonical_stable_ids(
            errors,
            stage
                .generated_variable_mappings
                .iter()
                .map(|mapping| mapping.component_id.as_str()),
            &format!("{item_context}.generated_variable_mappings"),
        );
        for mapping in &stage.generated_variable_mappings {
            for (name, id) in [
                (
                    "generated_score_variable_id",
                    mapping.generated_score_variable_id.as_str(),
                ),
                (
                    "generated_component_relation_id",
                    mapping.generated_component_relation_id.as_str(),
                ),
                (
                    "generated_component_parameter_id",
                    mapping.generated_component_parameter_id.as_str(),
                ),
                (
                    "component_relation_source_id",
                    mapping.component_relation_source_id.as_str(),
                ),
                (
                    "component_relation_target_id",
                    mapping.component_relation_target_id.as_str(),
                ),
            ] {
                require_stable_id(errors, id, &format!("{item_context}.{name}"));
            }
        }
        if !hoc_stage_signatures
            .insert((stage.higher_order_construct_id.as_str(), stage.stage_number))
        {
            errors.push(format!(
                "{item_context} duplicates a higher-order construct stage"
            ));
        }
        for (name, ids) in [
            ("input_construct_ids", &stage.input_construct_ids),
            ("output_variable_ids", &stage.output_variable_ids),
        ] {
            if ids.is_empty() {
                errors.push(format!("{item_context}.{name} must not be empty"));
            }
            require_canonical_stable_ids(
                errors,
                ids.iter().map(String::as_str),
                &format!("{item_context}.{name}"),
            );
        }
        require_canonical_stable_ids(
            errors,
            stage
                .relation_estimates
                .iter()
                .map(|relation| relation.relation_id.as_str()),
            &format!("{item_context}.relation_estimates"),
        );
        for (relation_index, relation) in stage.relation_estimates.iter().enumerate() {
            let relation_context = format!("{item_context}.relation_estimates[{relation_index}]");
            for (name, id) in [
                ("source_id", relation.source_id.as_str()),
                ("target_id", relation.target_id.as_str()),
            ] {
                require_stable_id(errors, id, &format!("{relation_context}.{name}"));
            }
            if relation.source_id == relation.target_id {
                errors.push(format!(
                    "{relation_context} requires distinct source_id and target_id"
                ));
            }
            if relation.parameter_id.is_some() != relation.kind.is_some() {
                errors.push(format!(
                    "{relation_context}.parameter_id and kind must be present or absent together"
                ));
            }
            if let Some(parameter_id) = &relation.parameter_id {
                require_stable_id(
                    errors,
                    parameter_id,
                    &format!("{relation_context}.parameter_id"),
                );
            }
            if relation
                .collinearity_vif
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
            {
                errors.push(format!(
                    "{relation_context}.collinearity_vif must be finite and positive"
                ));
            }
            validate_general_sem_estimate(
                errors,
                &relation.value,
                &format!("{relation_context}.value"),
            );
        }
    }

    for (index, row) in results.cbsem_parameters.iter().enumerate() {
        validate_cbsem_parameter_result_v1(
            errors,
            row,
            document_model_id,
            document_capability_ids,
            &format!("{context}.cbsem_parameters[{index}]"),
        );
    }

    for (index, fit) in results.cbsem_fit.iter().enumerate() {
        let item_context = format!("{context}.cbsem_fit[{index}]");
        validate_general_sem_trace(
            errors,
            &fit.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        if !fit.chi_square.is_finite() || fit.chi_square < 0.0 {
            errors.push(format!(
                "{item_context}.chi_square must be finite and nonnegative"
            ));
        }
        if fit
            .chi_square_p_value
            .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            errors.push(format!(
                "{item_context}.chi_square_p_value must be finite and between 0 and 1"
            ));
        }
        if fit.degrees_of_freedom == 0 && fit.chi_square_p_value.is_some() {
            errors.push(format!(
                "{item_context}.chi_square_p_value must be absent when degrees_of_freedom is zero"
            ));
        }
        for (name, value) in [
            ("rmsea", fit.rmsea),
            ("cfi", fit.cfi),
            ("tli", fit.tli),
            ("srmr", fit.srmr),
            ("aic", fit.aic),
            ("bic", fit.bic),
        ] {
            if value.is_some_and(|value| !value.is_finite()) {
                errors.push(format!("{item_context}.{name} must be finite"));
            }
        }
        if fit.rmsea.is_some_and(|value| value < 0.0) {
            errors.push(format!("{item_context}.rmsea must be nonnegative"));
        }
        if fit.srmr.is_some_and(|value| value < 0.0) {
            errors.push(format!("{item_context}.srmr must be nonnegative"));
        }
        if let Some(interval) = &fit.rmsea_interval {
            if fit.rmsea.is_none() {
                errors.push(format!(
                    "{item_context}.rmsea_interval requires an rmsea estimate"
                ));
            }
            if !interval.confidence_level.is_finite()
                || interval.confidence_level <= 0.0
                || interval.confidence_level >= 1.0
            {
                errors.push(format!(
                    "{item_context}.rmsea_interval.confidence_level must be finite and between 0 and 1"
                ));
            }
            validate_general_sem_bounds(
                errors,
                Some(interval.lower),
                Some(interval.upper),
                &format!("{item_context}.rmsea_interval"),
            );
            if interval.lower.is_finite() && interval.lower < 0.0 {
                errors.push(format!(
                    "{item_context}.rmsea_interval.lower must be nonnegative"
                ));
            }
        }
    }

    for (index, diagnostic) in results.identification_diagnostics.iter().enumerate() {
        let item_context = format!("{context}.identification_diagnostics[{index}]");
        validate_general_sem_trace(
            errors,
            &diagnostic.trace,
            document_model_id,
            document_capability_ids,
            &format!("{item_context}.trace"),
        );
        require_stable_id(
            errors,
            &diagnostic.subject_id,
            &format!("{item_context}.subject_id"),
        );
        require_stable_id(errors, &diagnostic.code, &format!("{item_context}.code"));
        if diagnostic.message.trim().is_empty() {
            errors.push(format!("{item_context}.message must be nonempty"));
        }
        if diagnostic.scope == CanonicalIdentificationScopeV1::Model
            && diagnostic.subject_id != document_model_id
        {
            errors.push(format!(
                "{item_context}.subject_id must equal provenance.model_id for model scope"
            ));
        }
        if diagnostic.status == CanonicalIdentificationStatusV1::Identified
            && diagnostic
                .degrees_of_freedom
                .is_some_and(|degrees| degrees < 0)
        {
            errors.push(format!(
                "{item_context} cannot be identified with negative degrees_of_freedom"
            ));
        }
    }

    validate_cbsem_bootstrap_v1(errors, results, provenance, document_capability_ids);
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_dataset_fingerprint_v1(value: &str) -> bool {
    value
        .strip_prefix("v2:")
        .map_or_else(|| is_lowercase_sha256(value), is_lowercase_sha256)
}

fn parse_timestamp(value: &str) -> Option<DateTime<chrono::FixedOffset>> {
    DateTime::parse_from_rfc3339(value).ok()
}

pub fn validate_canonical_result_document_v2(
    document: &CanonicalResultDocumentV2,
) -> CanonicalResultValidation {
    let mut errors = Vec::new();
    if document.schema_version != CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version must equal {CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION}"
        ));
    }
    require_stable_id(&mut errors, &document.document_id, "document_id");
    if document.title.trim().is_empty() {
        errors.push("title must be nonempty".to_string());
    }

    require_unique_ids(
        &mut errors,
        document.sections.iter().map(|item| item.id.as_str()),
        "sections",
    );
    require_unique_ids(
        &mut errors,
        document.tables.iter().map(|item| item.id.as_str()),
        "tables",
    );
    require_unique_ids(
        &mut errors,
        document.charts.iter().map(|item| item.id.as_str()),
        "charts",
    );
    require_unique_ids(
        &mut errors,
        document.notices.iter().map(|item| item.id.as_str()),
        "notices",
    );
    require_unique_ids(
        &mut errors,
        document.exclusions.iter().map(|item| item.id.as_str()),
        "exclusions",
    );
    require_unique_ids(
        &mut errors,
        document.footnotes.iter().map(|item| item.id.as_str()),
        "footnotes",
    );

    let section_ids: HashSet<&str> = document
        .sections
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let table_ids: HashSet<&str> = document
        .tables
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let chart_ids: HashSet<&str> = document
        .charts
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let footnote_ids: HashSet<&str> = document
        .footnotes
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let document_capability_ids = document.capability_cells.as_ref().map(|references| {
        let identities = validate_capability_set(&mut errors, references, "capability_cells");
        let identities = identities.into_iter().collect::<HashSet<_>>();
        let primary_identity =
            capability_cell_reference_identity_v2(&document.provenance.capability_cell);
        if !identities.contains(&primary_identity) {
            errors.push("capability_cells must include provenance.capability_cell".to_string());
        }
        identities
    });
    if let Some(results) = &document.general_sem_results {
        validate_general_sem_results_v1(
            &mut errors,
            results,
            &document.provenance,
            document_capability_ids.as_ref(),
        );
    }

    for section in &document.sections {
        if section.title.trim().is_empty() {
            errors.push(format!("section {} title must be nonempty", section.id));
        }
        for table_id in &section.table_ids {
            if !table_ids.contains(table_id.as_str()) {
                errors.push(format!(
                    "section {} references missing table {table_id}",
                    section.id
                ));
            }
        }
        for chart_id in &section.chart_ids {
            if !chart_ids.contains(chart_id.as_str()) {
                errors.push(format!(
                    "section {} references missing chart {chart_id}",
                    section.id
                ));
            }
        }
        match (&document_capability_ids, &section.capability_cells) {
            (Some(document_ids), Some(references)) => {
                let identities = validate_capability_set(
                    &mut errors,
                    references,
                    &format!("section {}.capability_cells", section.id),
                );
                for identity in identities {
                    if !document_ids.contains(&identity) {
                        errors.push(format!(
                            "section {} references an undeclared option cell {identity}",
                            section.id
                        ));
                    }
                }
            }
            (Some(_), None) => errors.push(format!(
                "section {} must declare capability_cells",
                section.id
            )),
            (None, Some(_)) => errors.push(format!(
                "section {} cannot declare capability_cells without a document capability_cells set",
                section.id
            )),
            (None, None) => {}
        }
    }

    for table in &document.tables {
        if table.title.trim().is_empty() {
            errors.push(format!("table {} title must be nonempty", table.id));
        }
        require_unique_ids(
            &mut errors,
            table.columns.iter().map(|item| item.id.as_str()),
            &format!("table {} columns", table.id),
        );
        require_unique_ids(
            &mut errors,
            table.rows.iter().map(|item| item.id.as_str()),
            &format!("table {} rows", table.id),
        );
        for column in &table.columns {
            if column.label.trim().is_empty() {
                errors.push(format!(
                    "table {} column {} label must be nonempty",
                    table.id, column.id
                ));
            }
            if column.description.trim().is_empty() {
                errors.push(format!(
                    "table {} column {} description must be nonempty",
                    table.id, column.id
                ));
            }
            if column
                .default_precision
                .is_some_and(|precision| !(0..=12).contains(&precision))
            {
                errors.push(format!(
                    "table {} column {} default_precision must be an integer from 0 to 12",
                    table.id, column.id
                ));
            }
        }
        for row in &table.rows {
            if row.cells.len() != table.columns.len() {
                errors.push(format!(
                    "table {} row {} has {} cells; expected {}",
                    table.id,
                    row.id,
                    row.cells.len(),
                    table.columns.len()
                ));
                continue;
            }
            for (cell, column) in row.cells.iter().zip(&table.columns) {
                if !cell.matches_column(column.data_type) {
                    errors.push(format!(
                        "table {} row {} cell {} is {}; expected {} or missing",
                        table.id,
                        row.id,
                        column.id,
                        cell.kind_name(),
                        match column.data_type {
                            CanonicalColumnType::Number => "number",
                            CanonicalColumnType::Text => "text",
                            CanonicalColumnType::Boolean => "boolean",
                        }
                    ));
                }
                if let CanonicalResultCell::Number { value, .. } = cell {
                    if !value.is_finite() {
                        errors.push(format!(
                            "table {} row {} cell {} must be finite",
                            table.id, row.id, column.id
                        ));
                    }
                }
            }
        }
        for footnote_id in &table.footnote_ids {
            if !footnote_ids.contains(footnote_id.as_str()) {
                errors.push(format!(
                    "table {} references missing footnote {footnote_id}",
                    table.id
                ));
            }
        }
        match (&document_capability_ids, &table.capability_cells) {
            (Some(document_ids), Some(references)) => {
                let identities = validate_capability_set(
                    &mut errors,
                    references,
                    &format!("table {}.capability_cells", table.id),
                );
                for identity in identities {
                    if !document_ids.contains(&identity) {
                        errors.push(format!(
                            "table {} references an undeclared option cell {identity}",
                            table.id
                        ));
                    }
                }
            }
            (Some(_), None) => {
                errors.push(format!("table {} must declare capability_cells", table.id))
            }
            (None, Some(_)) => errors.push(format!(
                "table {} cannot declare capability_cells without a document capability_cells set",
                table.id
            )),
            (None, None) => {}
        }
    }

    if document_capability_ids.is_some() {
        let table_by_id = document
            .tables
            .iter()
            .map(|table| (table.id.as_str(), table))
            .collect::<BTreeMap<_, _>>();
        for section in &document.sections {
            let Some(section_references) = &section.capability_cells else {
                continue;
            };
            let section_capabilities = section_references
                .iter()
                .map(capability_cell_reference_identity_v2)
                .collect::<HashSet<_>>();
            let required_by_tables = section
                .table_ids
                .iter()
                .filter_map(|table_id| table_by_id.get(table_id.as_str()))
                .filter_map(|table| table.capability_cells.as_ref())
                .flatten()
                .map(capability_cell_reference_identity_v2)
                .collect::<BTreeSet<_>>();
            for identity in required_by_tables {
                if !section_capabilities.contains(&identity) {
                    errors.push(format!(
                        "section {} is missing table option cell {identity}",
                        section.id
                    ));
                }
            }
        }
    }

    for chart in &document.charts {
        if chart.title.trim().is_empty() || chart.description.trim().is_empty() {
            errors.push(format!(
                "chart {} needs a title and accessible description",
                chart.id
            ));
        }
        if let Some(source_table_id) = &chart.source_table_id {
            if !table_ids.contains(source_table_id.as_str()) {
                errors.push(format!(
                    "chart {} references missing table {source_table_id}",
                    chart.id
                ));
            }
        }
        require_unique_ids(
            &mut errors,
            chart.series.iter().map(|item| item.id.as_str()),
            &format!("chart {} series", chart.id),
        );
        for series in &chart.series {
            for (point_index, point) in series.points.iter().enumerate() {
                if let CanonicalChartX::Number(value) = point.x {
                    if !value.is_finite() {
                        errors.push(format!(
                            "chart {} series {} point {point_index} x must be finite",
                            chart.id, series.id
                        ));
                    }
                }
                for (name, value) in [
                    ("y", Some(point.y)),
                    ("lower", point.lower),
                    ("upper", point.upper),
                ] {
                    if value.is_some_and(|number| !number.is_finite()) {
                        errors.push(format!(
                            "chart {} series {} point {point_index} {name} must be finite",
                            chart.id, series.id
                        ));
                    }
                }
                if matches!((point.lower, point.upper), (Some(lower), Some(upper)) if lower > upper)
                {
                    errors.push(format!(
                        "chart {} series {} point {point_index} lower exceeds upper",
                        chart.id, series.id
                    ));
                }
            }
        }
    }

    for notice in &document.notices {
        if notice.code.trim().is_empty() || notice.message.trim().is_empty() {
            errors.push(format!(
                "notice {} code and message must be nonempty",
                notice.id
            ));
        }
        for section_id in &notice.section_ids {
            if !section_ids.contains(section_id.as_str()) {
                errors.push(format!(
                    "notice {} references missing section {section_id}",
                    notice.id
                ));
            }
        }
        for table_id in &notice.table_ids {
            if !table_ids.contains(table_id.as_str()) {
                errors.push(format!(
                    "notice {} references missing table {table_id}",
                    notice.id
                ));
            }
        }
    }

    for exclusion in &document.exclusions {
        if exclusion.title.trim().is_empty() || exclusion.reason.trim().is_empty() {
            errors.push(format!(
                "exclusion {} title and reason must be nonempty",
                exclusion.id
            ));
        }
        if let Some(capability_cell) = &exclusion.capability_cell {
            validate_capability_reference(
                &mut errors,
                capability_cell,
                &format!("exclusion {}.capability_cell", exclusion.id),
            );
        }
    }

    let provenance = &document.provenance;
    for (name, value) in [
        ("run_id", provenance.run_id.as_str()),
        ("project_id", provenance.project_id.as_str()),
        ("model_id", provenance.model_id.as_str()),
        ("dataset_id", provenance.dataset_id.as_str()),
        ("recipe_id", provenance.recipe_id.as_str()),
        ("method_version", provenance.method_version.as_str()),
        ("engine_version", provenance.engine_version.as_str()),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("provenance.{name} must be nonempty"));
        }
    }
    if !is_lowercase_sha256(&provenance.model_digest) {
        errors.push("provenance.model_digest must be lowercase SHA-256".to_string());
    }
    if !is_dataset_fingerprint_v1(&provenance.dataset_fingerprint) {
        errors.push(
            "provenance.dataset_fingerprint must be bare lowercase SHA-256 or v2:<lowercase SHA-256>"
                .to_string(),
        );
    }
    if !is_lowercase_sha256(&provenance.recipe_digest) {
        errors.push("provenance.recipe_digest must be lowercase SHA-256".to_string());
    }
    if provenance.seed.is_some_and(|seed| seed < 0) {
        errors.push("provenance.seed must be a nonnegative safe integer or null".to_string());
    } else if provenance.seed.is_some_and(|seed| seed > MAX_SAFE_INTEGER) {
        errors.push("provenance.seed must be a nonnegative safe integer or null".to_string());
    }
    if provenance.workers < 1 {
        errors.push("provenance.workers must be a positive integer".to_string());
    }
    validate_capability_reference(
        &mut errors,
        &provenance.capability_cell,
        "provenance.capability_cell",
    );
    let started_at = parse_timestamp(&provenance.started_at);
    let completed_at = parse_timestamp(&provenance.completed_at);
    if started_at.is_none() {
        errors.push("provenance.started_at must be an ISO timestamp".to_string());
    }
    if completed_at.is_none() {
        errors.push("provenance.completed_at must be an ISO timestamp".to_string());
    }
    if matches!((started_at, completed_at), (Some(started), Some(completed)) if completed < started)
    {
        errors.push("provenance.completed_at precedes started_at".to_string());
    }

    let presentation = &document.presentation;
    if presentation
        .default_section_id
        .as_deref()
        .is_some_and(|id| !section_ids.contains(id))
    {
        errors.push("presentation.default_section_id is missing".to_string());
    }
    if presentation
        .default_table_id
        .as_deref()
        .is_some_and(|id| !table_ids.contains(id))
    {
        errors.push("presentation.default_table_id is missing".to_string());
    }
    if !(0..=12).contains(&presentation.precision) {
        errors.push("presentation.precision must be an integer from 0 to 12".to_string());
    }
    if presentation.missing_value_label.trim().is_empty() {
        errors.push("presentation.missing_value_label must be nonempty".to_string());
    }

    CanonicalResultValidation {
        passed: errors.is_empty(),
        errors,
    }
}

/// Product-use boundary for the optional capability-attribution extension.
/// Historical documents remain readable but are never silently upgraded from
/// their primary capability into comparison or qualification-export evidence.
pub fn canonical_result_use_eligibility_v2(
    document: &CanonicalResultDocumentV2,
) -> CanonicalResultUseEligibilityV2 {
    let validation = validate_canonical_result_document_v2(document);
    if !validation.passed {
        return CanonicalResultUseEligibilityV2 {
            readable: false,
            comparison_eligible: false,
            qualification_export_eligible: false,
            ineligibility: Some(CanonicalResultQualificationIneligibilityV2::InvalidDocument),
        };
    }
    if document.capability_cells.is_none() {
        return CanonicalResultUseEligibilityV2 {
            readable: true,
            comparison_eligible: false,
            qualification_export_eligible: false,
            ineligibility: Some(
                CanonicalResultQualificationIneligibilityV2::LegacyCapabilityAttributionMissing,
            ),
        };
    }
    let is_multimod = document.capability_cells.as_ref().is_some_and(|cells| {
        cells
            .iter()
            .any(|cell| cell.cell_id.starts_with("qpls.multimod."))
    });
    if is_multimod {
        let explicitly_release_qualified = document
            .tables
            .iter()
            .find(|table| table.id == "multimod_run_provenance")
            .and_then(|table| {
                let index = table
                    .columns
                    .iter()
                    .position(|column| column.id == "qualification")?;
                let row = table.rows.iter().find(|row| row.id == "run")?;
                match row.cells.get(index)? {
                    CanonicalResultCell::Text { value } => {
                        Some(value == "release_qualified_candidate")
                    }
                    _ => Some(false),
                }
            })
            .unwrap_or(false);
        if !explicitly_release_qualified {
            return CanonicalResultUseEligibilityV2 {
                readable: true,
                comparison_eligible: false,
                qualification_export_eligible: false,
                ineligibility: Some(
                    CanonicalResultQualificationIneligibilityV2::UnqualifiedLabsCapability,
                ),
            };
        }
    }
    CanonicalResultUseEligibilityV2 {
        readable: true,
        comparison_eligible: true,
        qualification_export_eligible: true,
        ineligibility: None,
    }
}

fn stable_value(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.into_iter().map(stable_value).collect()),
        Value::Object(values) => {
            let sorted: BTreeMap<String, Value> = values
                .into_iter()
                .map(|(key, value)| (key, stable_value(value)))
                .collect();
            let mut object = Map::new();
            for (key, value) in sorted {
                object.insert(key, value);
            }
            Value::Object(object)
        }
        other => other,
    }
}

fn stable_json(value: Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(&stable_value(value))
}

pub fn canonical_result_document_json(
    document: &CanonicalResultDocumentV2,
) -> Result<String, serde_json::Error> {
    stable_json(serde_json::to_value(document)?)
}

/// Scientific projection for semantic equality. Execution-only worker/timing
/// fields and display caches/defaults are excluded while scientific identity,
/// ordered tables, chart data, notices, and exclusions remain bound.
pub fn canonical_analytical_result_json(
    document: &CanonicalResultDocumentV2,
) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(document)?;
    let Some(root) = value.as_object_mut() else {
        unreachable!("CanonicalResultDocumentV2 always serializes as an object");
    };
    root.remove("presentation");

    if let Some(provenance) = root.get_mut("provenance").and_then(Value::as_object_mut) {
        provenance.remove("workers");
        provenance.remove("started_at");
        provenance.remove("completed_at");
    }
    if let Some(results) = root
        .get_mut("general_sem_results")
        .and_then(Value::as_object_mut)
    {
        for key in ["inference_receipt", "cbsem_bootstrap_receipt"] {
            if let Some(receipt) = results.get_mut(key).and_then(Value::as_object_mut) {
                // Worker count is execution provenance, not a scientific estimand.
                // The complete receipt still preserves it in the archival document.
                receipt.remove("workers");
            }
        }
    }

    if let Some(tables) = root.get_mut("tables").and_then(Value::as_array_mut) {
        for table in tables {
            let Some(rows) = table.get_mut("rows").and_then(Value::as_array_mut) else {
                continue;
            };
            for row in rows {
                let Some(cells) = row.get_mut("cells").and_then(Value::as_array_mut) else {
                    continue;
                };
                for cell in cells {
                    if let Some(cell) = cell.as_object_mut() {
                        cell.remove("display");
                    }
                }
            }
        }
    }

    if let Some(charts) = root.get_mut("charts").and_then(Value::as_array_mut) {
        for chart in charts {
            if let Some(chart) = chart.as_object_mut() {
                chart.remove("display");
            }
        }
    }

    stable_json(value)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LegacyStringResultTable {
    pub id: String,
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LegacyResultMigrationContext {
    pub document_id: String,
    pub title: String,
    pub provenance: CanonicalResultProvenanceV2,
}

/// Preserve historical string-only result tables without interpreting formatted
/// strings as numbers. Method-specific adapters must emit typed analytical cells
/// directly for all new runs.
pub fn canonical_result_document_from_legacy_tables(
    context: LegacyResultMigrationContext,
    legacy_tables: Vec<LegacyStringResultTable>,
) -> CanonicalResultDocumentV2 {
    let tables: Vec<CanonicalResultTable> = legacy_tables
        .iter()
        .map(|table| CanonicalResultTable {
            id: table.id.clone(),
            title: table.title.clone(),
            description: Some(
                "Historical string-table result preserved without numeric reinterpretation."
                    .to_string(),
            ),
            columns: table
                .columns
                .iter()
                .enumerate()
                .map(|(index, label)| CanonicalResultColumn {
                    id: format!("column_{}", index + 1),
                    label: label.clone(),
                    data_type: CanonicalColumnType::Text,
                    description: format!("Historical column {label}"),
                    role: None,
                    unit: None,
                    default_precision: None,
                })
                .collect(),
            rows: table
                .rows
                .iter()
                .enumerate()
                .map(|(index, cells)| CanonicalResultRow {
                    id: format!("row_{}", index + 1),
                    cells: cells
                        .iter()
                        .map(|value| CanonicalResultCell::Text {
                            value: value.clone(),
                        })
                        .collect(),
                })
                .collect(),
            footnote_ids: Vec::new(),
            capability_cells: None,
        })
        .collect();

    let notices = legacy_tables
        .iter()
        .filter_map(|table| {
            table.warning.as_ref().map(|warning| CanonicalResultNotice {
                id: format!("historical_{}", table.id),
                code: "historical_string_table".to_string(),
                severity: CanonicalNoticeSeverity::Information,
                message: warning.clone(),
                section_ids: vec!["historical_results".to_string()],
                table_ids: vec![table.id.clone()],
            })
        })
        .collect();

    CanonicalResultDocumentV2 {
        schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
        document_id: context.document_id,
        title: context.title,
        provenance: context.provenance,
        capability_cells: None,
        general_sem_results: None,
        sections: vec![CanonicalResultSection {
            id: "historical_results".to_string(),
            title: "Historical results".to_string(),
            description: None,
            table_ids: tables.iter().map(|table| table.id.clone()).collect(),
            chart_ids: Vec::new(),
            capability_cells: None,
        }],
        presentation: CanonicalResultPresentationV2 {
            default_section_id: Some("historical_results".to_string()),
            default_table_id: tables.first().map(|table| table.id.clone()),
            precision: 4,
            missing_value_label: "—".to_string(),
            chart_defaults: CanonicalChartDisplayOptions::default(),
        },
        tables,
        charts: Vec::new(),
        notices,
        exclusions: Vec::new(),
        footnotes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecipeV4CompilerTarget;

    fn capability_reference() -> CapabilityCellReferenceV2 {
        CapabilityCellReferenceV2 {
            registry_schema_version: 2,
            capability_id: "qpls3.pls.algorithm".to_string(),
            cell_id: "standard.reflective_recursive".to_string(),
            capability_version: "pls_algorithm_v2".to_string(),
        }
    }

    fn secondary_capability_reference() -> CapabilityCellReferenceV2 {
        general_sem_pls_bootstrap_capability_cell_v1()
    }

    fn general_sem_effect_capability_reference() -> CapabilityCellReferenceV2 {
        crate::pls_general_recursive_effects_capability_cell_v1()
    }

    fn provenance() -> CanonicalResultProvenanceV2 {
        CanonicalResultProvenanceV2 {
            run_id: "run-1".to_string(),
            project_id: "project-1".to_string(),
            model_id: "model-1".to_string(),
            model_digest: "a".repeat(64),
            dataset_id: "dataset-1".to_string(),
            dataset_fingerprint: "b".repeat(64),
            recipe_id: "recipe-1".to_string(),
            recipe_digest: "c".repeat(64),
            capability_cell: capability_reference(),
            method_version: "pls_algorithm_v2".to_string(),
            engine_version: "qpls-estimation-test".to_string(),
            seed: Some(42),
            workers: 4,
            started_at: "2026-08-14T00:00:00Z".to_string(),
            completed_at: "2026-08-14T00:00:01Z".to_string(),
        }
    }

    fn document_fixture() -> CanonicalResultDocumentV2 {
        CanonicalResultDocumentV2 {
            schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
            document_id: "result.document:1".to_string(),
            title: "PLS path results".to_string(),
            provenance: provenance(),
            capability_cells: Some(vec![capability_reference()]),
            general_sem_results: None,
            sections: vec![CanonicalResultSection {
                id: "structural".to_string(),
                title: "Structural model".to_string(),
                description: None,
                table_ids: vec!["paths".to_string()],
                chart_ids: vec!["path_plot".to_string()],
                capability_cells: Some(vec![capability_reference()]),
            }],
            tables: vec![CanonicalResultTable {
                id: "paths".to_string(),
                title: "Path coefficients".to_string(),
                description: None,
                columns: vec![
                    CanonicalResultColumn {
                        id: "path".to_string(),
                        label: "Path".to_string(),
                        data_type: CanonicalColumnType::Text,
                        description: "Directed structural path".to_string(),
                        role: Some(CanonicalColumnRole::Label),
                        unit: None,
                        default_precision: None,
                    },
                    CanonicalResultColumn {
                        id: "estimate".to_string(),
                        label: "Estimate".to_string(),
                        data_type: CanonicalColumnType::Number,
                        description: "Standardized path estimate".to_string(),
                        role: Some(CanonicalColumnRole::Estimate),
                        unit: None,
                        default_precision: Some(4),
                    },
                ],
                rows: vec![CanonicalResultRow {
                    id: "x_to_y".to_string(),
                    cells: vec![
                        CanonicalResultCell::Text {
                            value: "X → Y".to_string(),
                        },
                        CanonicalResultCell::Number {
                            value: 0.42,
                            display: Some("0.4200".to_string()),
                        },
                    ],
                }],
                footnote_ids: vec!["standardized".to_string()],
                capability_cells: Some(vec![capability_reference()]),
            }],
            charts: vec![CanonicalResultChart {
                id: "path_plot".to_string(),
                title: "Path coefficient".to_string(),
                description: "One bar showing the standardized X to Y path coefficient."
                    .to_string(),
                kind: CanonicalChartKind::Bar,
                series: vec![CanonicalChartSeries {
                    id: "estimate".to_string(),
                    label: "Estimate".to_string(),
                    group: None,
                    points: vec![CanonicalChartPoint {
                        x: CanonicalChartX::Text("X → Y".to_string()),
                        y: 0.42,
                        lower: None,
                        upper: None,
                        label: None,
                    }],
                }],
                source_table_id: Some("paths".to_string()),
                display: CanonicalChartDisplayOptions {
                    palette: Some("institutional_navy".to_string()),
                    show_values: Some(true),
                    ..CanonicalChartDisplayOptions::default()
                },
            }],
            notices: Vec::new(),
            exclusions: Vec::new(),
            footnotes: vec![CanonicalResultFootnote {
                id: "standardized".to_string(),
                text: "Standardized estimates.".to_string(),
                reference: None,
            }],
            presentation: CanonicalResultPresentationV2 {
                default_section_id: Some("structural".to_string()),
                default_table_id: Some("paths".to_string()),
                precision: 4,
                missing_value_label: "—".to_string(),
                chart_defaults: CanonicalChartDisplayOptions {
                    show_legend: Some(true),
                    ..CanonicalChartDisplayOptions::default()
                },
            },
        }
    }

    fn general_sem_trace() -> CanonicalGeneralSemResultTraceV1 {
        CanonicalGeneralSemResultTraceV1 {
            model_id: "model-1".to_string(),
            capability_cell: capability_reference(),
        }
    }

    fn effect_value(estimate: f64) -> CanonicalGeneralSemEstimateV1 {
        CanonicalGeneralSemEstimateV1 {
            estimate,
            bootstrap_mean: None,
            bootstrap_bias: None,
            standard_error: None,
            lower: None,
            upper: None,
            p_value: None,
            bootstrap_usable_replicates: None,
            bootstrap_two_sided_exceedances: None,
        }
    }

    fn inferred_effect_value(estimate: f64) -> CanonicalGeneralSemEstimateV1 {
        CanonicalGeneralSemEstimateV1 {
            estimate,
            bootstrap_mean: Some(estimate + 0.01),
            bootstrap_bias: Some(0.01),
            standard_error: Some(0.04),
            lower: Some(estimate - 0.08),
            upper: Some(estimate + 0.08),
            p_value: Some(0.2),
            bootstrap_usable_replicates: Some(9),
            bootstrap_two_sided_exceedances: Some(1),
        }
    }

    fn general_sem_results_fixture() -> CanonicalGeneralSemResultsV1 {
        let mut results = CanonicalGeneralSemResultsV1 {
            schema_version: CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION,
            inference_receipt: None,
            specific_indirect_effects: vec![CanonicalSpecificIndirectEffectResultV1 {
                effect_id: crate::specific_directed_path_identity_v1(&[
                    "relation_a".to_string(),
                    "relation_b".to_string(),
                ]),
                estimand_id: "estimand_specific_1".to_string(),
                trace: general_sem_trace(),
                source_id: "construct_x".to_string(),
                target_id: "construct_y".to_string(),
                ordered_relation_ids: vec!["relation_a".to_string(), "relation_b".to_string()],
                value: effect_value(0.12),
            }],
            aggregate_effects: vec![
                CanonicalAggregateEffectResultV1 {
                    effect_id: "estimand_total_indirect_1".to_string(),
                    estimand_id: "estimand_total_indirect_1".to_string(),
                    trace: general_sem_trace(),
                    kind: CanonicalAggregateEffectKindV1::TotalIndirect,
                    source_id: "construct_x".to_string(),
                    target_id: "construct_y".to_string(),
                    direct_relation_ids: Vec::new(),
                    contributing_path_identities: vec![crate::specific_directed_path_identity_v1(
                        &["relation_a".to_string(), "relation_b".to_string()],
                    )],
                    value: effect_value(0.18),
                },
                CanonicalAggregateEffectResultV1 {
                    effect_id: "estimand_total_effect_1".to_string(),
                    estimand_id: "estimand_total_effect_1".to_string(),
                    trace: general_sem_trace(),
                    kind: CanonicalAggregateEffectKindV1::TotalEffect,
                    source_id: "construct_x".to_string(),
                    target_id: "construct_y".to_string(),
                    direct_relation_ids: vec!["relation_direct".to_string()],
                    contributing_path_identities: vec![crate::specific_directed_path_identity_v1(
                        &["relation_a".to_string(), "relation_b".to_string()],
                    )],
                    value: effect_value(0.60),
                },
            ],
            joint_stage_structural_coefficients: Vec::new(),
            interaction_effects: Vec::new(),
            three_way_interaction_effects: Vec::new(),
            three_way_conditional_interaction_effects: Vec::new(),
            three_way_simple_slopes: Vec::new(),
            three_way_moderation_bootstrap_receipt: None,
            conditional_effect_probes: vec![
                CanonicalConditionalEffectProbeResultV1 {
                    probe_id: "probe_data".to_string(),
                    trace: general_sem_trace(),
                    moderator_id: "moderator_m".to_string(),
                    values:
                        CanonicalConditionalProbeValuesResultV1::DataDerivedMeanPlusMinusOneSd {
                            mean: 1.0,
                            standard_deviation: 1.0,
                        },
                },
                CanonicalConditionalEffectProbeResultV1 {
                    probe_id: "probe_explicit".to_string(),
                    trace: general_sem_trace(),
                    moderator_id: "moderator_m".to_string(),
                    values: CanonicalConditionalProbeValuesResultV1::Explicit {
                        values: vec![-1.0, 0.0, 1.0],
                    },
                },
            ],
            conditional_effects: vec![CanonicalConditionalEffectResultV1 {
                effect_id: "effect_conditional_1".to_string(),
                estimand_id: "estimand_conditional_1".to_string(),
                trace: general_sem_trace(),
                interaction_id: "interaction_1".to_string(),
                interaction_effect_id: None,
                focal_relation_id: "relation_focal_1".to_string(),
                probe_id: "probe_data".to_string(),
                moderator_id: "moderator_m".to_string(),
                probe_value_index: 1,
                moderator_value: 1.0,
                value: effect_value(0.42),
            }],
            conditional_indirect_effects: Vec::new(),
            moderated_mediation_indices: Vec::new(),
            interaction_plots: vec![CanonicalInteractionPlotResultV1 {
                plot_id: "interaction_plot_1".to_string(),
                trace: general_sem_trace(),
                interaction_id: "interaction_1".to_string(),
                interaction_effect_id: None,
                focal_relation_id: "relation_focal_1".to_string(),
                focal_predictor_id: "construct_x".to_string(),
                moderator_id: "moderator_m".to_string(),
                outcome_id: "construct_y".to_string(),
                series: vec![
                    CanonicalInteractionPlotSeriesV1 {
                        series_id: "series_01_low".to_string(),
                        probe_id: "probe_data".to_string(),
                        probe_value_index: 0,
                        moderator_value: 0.0,
                        points: vec![
                            CanonicalInteractionPlotPointV1 {
                                focal_value: -1.0,
                                predicted_value: -0.2,
                                lower: Some(-0.3),
                                upper: Some(-0.1),
                            },
                            CanonicalInteractionPlotPointV1 {
                                focal_value: 1.0,
                                predicted_value: 0.2,
                                lower: Some(0.1),
                                upper: Some(0.3),
                            },
                        ],
                    },
                    CanonicalInteractionPlotSeriesV1 {
                        series_id: "series_02_high".to_string(),
                        probe_id: "probe_data".to_string(),
                        probe_value_index: 2,
                        moderator_value: 2.0,
                        points: vec![
                            CanonicalInteractionPlotPointV1 {
                                focal_value: -1.0,
                                predicted_value: -0.5,
                                lower: Some(-0.6),
                                upper: Some(-0.4),
                            },
                            CanonicalInteractionPlotPointV1 {
                                focal_value: 1.0,
                                predicted_value: 0.5,
                                lower: Some(0.4),
                                upper: Some(0.6),
                            },
                        ],
                    },
                ],
            }],
            higher_order_stages: vec![
                CanonicalHocStageResultV1 {
                    stage_id: "hoc_stage_1".to_string(),
                    trace: general_sem_trace(),
                    higher_order_construct_id: "hoc_ab".to_string(),
                    stage_number: 1,
                    kind: CanonicalHocStageKindV1::LowerOrderScoreEstimation,
                    input_construct_ids: vec!["construct_a".to_string(), "construct_b".to_string()],
                    output_variable_ids: vec!["score_a".to_string(), "score_b".to_string()],
                    approach: None,
                    measurement_type: None,
                    generated_variable_mappings: Vec::new(),
                    receipt: None,
                    relation_estimates: Vec::new(),
                },
                CanonicalHocStageResultV1 {
                    stage_id: "hoc_stage_2".to_string(),
                    trace: general_sem_trace(),
                    higher_order_construct_id: "hoc_ab".to_string(),
                    stage_number: 2,
                    kind: CanonicalHocStageKindV1::HigherOrderEstimation,
                    input_construct_ids: vec!["score_a".to_string(), "score_b".to_string()],
                    output_variable_ids: vec!["hoc_ab".to_string()],
                    approach: None,
                    measurement_type: None,
                    generated_variable_mappings: Vec::new(),
                    receipt: None,
                    relation_estimates: vec![CanonicalHocRelationEstimateV1 {
                        relation_id: "relation_hoc_1".to_string(),
                        parameter_id: None,
                        source_id: "hoc_ab".to_string(),
                        target_id: "construct_y".to_string(),
                        kind: None,
                        value: effect_value(0.31),
                        collinearity_vif: None,
                    }],
                },
            ],
            higher_order_inference_receipt: None,
            cbsem_parameters: Vec::new(),
            cbsem_fit: vec![CanonicalCbsemFitResultV1 {
                fit_id: "cbsem_fit_1".to_string(),
                trace: general_sem_trace(),
                chi_square: 12.5,
                degrees_of_freedom: 8,
                chi_square_p_value: Some(0.13),
                rmsea: Some(0.04),
                rmsea_interval: Some(CanonicalGeneralSemIntervalV1 {
                    confidence_level: 0.90,
                    lower: 0.01,
                    upper: 0.08,
                }),
                cfi: Some(0.98),
                tli: Some(0.97),
                srmr: Some(0.03),
                aic: Some(101.2),
                bic: Some(120.4),
            }],
            identification_diagnostics: vec![CanonicalIdentificationDiagnosticV1 {
                diagnostic_id: "identification_model_1".to_string(),
                trace: general_sem_trace(),
                scope: CanonicalIdentificationScopeV1::Model,
                subject_id: "model-1".to_string(),
                status: CanonicalIdentificationStatusV1::Identified,
                code: "identified".to_string(),
                message: "The compiled model passed identification checks.".to_string(),
                degrees_of_freedom: Some(8),
            }],
            cbsem_bootstrap_receipt: None,
            cbsem_bootstrap_inference: Vec::new(),
        };
        results
            .aggregate_effects
            .sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
        results
    }

    fn general_sem_multiple_moderation_capability_reference() -> CapabilityCellReferenceV2 {
        crate::pls_general_multiple_moderation_point_capability_cell_v1()
    }

    fn general_sem_interaction_document_fixture() -> CanonicalResultDocumentV2 {
        let mut document = document_fixture();
        let moderation_cell = general_sem_multiple_moderation_capability_reference();
        document.capability_cells = Some(vec![capability_reference(), moderation_cell.clone()]);
        let mut results = general_sem_results_fixture();
        let interaction_effect_id = "relation_interaction_1_effect".to_string();
        let interaction_trace = CanonicalGeneralSemResultTraceV1 {
            model_id: document.provenance.model_id.clone(),
            capability_cell: moderation_cell,
        };
        results.joint_stage_structural_coefficients =
            vec![CanonicalJointStageStructuralCoefficientResultV1 {
                relation_id: "relation_focal_1".to_string(),
                parameter_id: "parameter_focal_1".to_string(),
                trace: interaction_trace.clone(),
                source_id: "construct_x".to_string(),
                target_id: "construct_y".to_string(),
                role: CanonicalStructuralRelationRoleV1::Structural,
                estimate: effect_value(0.4),
                stage: CanonicalStructuralEstimateStageV1::JointStageTwo,
                method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
                    .to_string(),
            }];
        results.interaction_effects = vec![CanonicalInteractionEffectResultV1 {
            effect_id: interaction_effect_id.clone(),
            trace: interaction_trace.clone(),
            interaction_id: "interaction_1".to_string(),
            focal_relation_id: "relation_focal_1".to_string(),
            interaction_effect_relation_id: interaction_effect_id.clone(),
            interaction_effect_parameter_id: "parameter_interaction_1_effect".to_string(),
            focal_predictor_id: "construct_x".to_string(),
            moderator_id: "moderator_m".to_string(),
            outcome_id: "construct_y".to_string(),
            generated_product_column_id: "generated_interaction_1_product".to_string(),
            stage_one_model_scientific_sha256: "d".repeat(64),
            method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1.to_string(),
            construction_method: CanonicalInteractionConstructionMethodV1::TwoStage,
            product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1.to_string(),
            hierarchy_policy: CanonicalInteractionHierarchyPolicyV1::Strong,
            hierarchy_policy_version: GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1
                .to_string(),
            conditioning_policy_version: GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1.to_string(),
            observation_count: 31,
            unstandardized_product_mean: 0.125,
            unstandardized_product_sample_standard_deviation: 0.5,
            standardized_product_coefficient: effect_value(0.2),
            scientific_rescaled_gamma: effect_value(0.4),
        }];
        let probe_id = "probe:interaction_1:standardized_minus1_zero_plus1".to_string();
        results.conditional_effect_probes = vec![CanonicalConditionalEffectProbeResultV1 {
            probe_id: probe_id.clone(),
            trace: interaction_trace.clone(),
            moderator_id: "moderator_m".to_string(),
            values: CanonicalConditionalProbeValuesResultV1::Explicit {
                values: vec![-1.0, 0.0, 1.0],
            },
        }];
        results.conditional_effects = [-1.0, 0.0, 1.0]
            .into_iter()
            .enumerate()
            .map(
                |(index, moderator_value)| CanonicalConditionalEffectResultV1 {
                    effect_id: format!("conditional:interaction_1:{index}"),
                    estimand_id: "conditional_slope:interaction_1".to_string(),
                    trace: interaction_trace.clone(),
                    interaction_id: "interaction_1".to_string(),
                    interaction_effect_id: Some(interaction_effect_id.clone()),
                    focal_relation_id: "relation_focal_1".to_string(),
                    probe_id: probe_id.clone(),
                    moderator_id: "moderator_m".to_string(),
                    probe_value_index: index as u32,
                    moderator_value,
                    value: effect_value(0.4 + 0.1 * moderator_value),
                },
            )
            .collect();
        results.interaction_plots = vec![CanonicalInteractionPlotResultV1 {
            plot_id: "plot:interaction_1".to_string(),
            trace: interaction_trace,
            interaction_id: "interaction_1".to_string(),
            interaction_effect_id: Some(interaction_effect_id),
            focal_relation_id: "relation_focal_1".to_string(),
            focal_predictor_id: "construct_x".to_string(),
            moderator_id: "moderator_m".to_string(),
            outcome_id: "construct_y".to_string(),
            series: [-1.0, 0.0, 1.0]
                .into_iter()
                .enumerate()
                .map(
                    |(index, moderator_value)| CanonicalInteractionPlotSeriesV1 {
                        series_id: format!("series:interaction_1:{index}"),
                        probe_id: probe_id.clone(),
                        probe_value_index: index as u32,
                        moderator_value,
                        points: [-1.0, 0.0, 1.0]
                            .into_iter()
                            .map(|focal_value| CanonicalInteractionPlotPointV1 {
                                focal_value,
                                predicted_value: focal_value * (0.4 + 0.1 * moderator_value),
                                lower: None,
                                upper: None,
                            })
                            .collect(),
                    },
                )
                .collect(),
        }];
        document.general_sem_results = Some(results);
        document
    }

    fn general_sem_inference_document_fixture() -> CanonicalResultDocumentV2 {
        let mut document = document_fixture();
        let mut results = general_sem_results_fixture();
        results.specific_indirect_effects[0].value = inferred_effect_value(0.12);
        results.specific_indirect_effects[0].trace.capability_cell =
            general_sem_effect_capability_reference();
        for effect in &mut results.aggregate_effects {
            effect.value = inferred_effect_value(effect.value.estimate);
            effect.trace.capability_cell = general_sem_effect_capability_reference();
        }
        let mut effect_ids = results
            .specific_indirect_effects
            .iter()
            .map(|effect| effect.effect_id.clone())
            .chain(
                results
                    .aggregate_effects
                    .iter()
                    .map(|effect| effect.effect_id.clone()),
            )
            .collect::<Vec<_>>();
        effect_ids.sort();
        let failed_replicates = vec![CanonicalGeneralSemFailedReplicateV1 {
            replicate_index: 7,
            reason_code: CanonicalGeneralSemFailedReplicateReasonV1::EstimationNonconvergence,
            message: "The complete PLS model did not converge for this draw.".to_string(),
        }];
        let usable_replicate_indices = (0..10_u32)
            .filter(|replicate_index| *replicate_index != 7)
            .collect::<Vec<_>>();
        results.inference_receipt = Some(CanonicalGeneralSemInferenceReceiptV1 {
            kind: CanonicalGeneralSemInferenceKindV1::CaseBootstrap,
            capability_cell: secondary_capability_reference(),
            capability_dependencies: Vec::new(),
            method_version: GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1.to_string(),
            resampling_operation_version: GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1
                .to_string(),
            resampling_stream_version: GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1
                .to_string(),
            quantile_method_version: GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1.to_string(),
            standard_error_method_version: GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1
                .to_string(),
            summation_method_version: GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1.to_string(),
            p_value_method_version: GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1
                .to_string(),
            failure_policy_version: GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1
                .to_string(),
            compilation_artifact_identity_sha256: "d".repeat(64),
            compiled_plan_sha256: "9".repeat(64),
            general_sem_config_sha256: "e".repeat(64),
            recipe_analytical_sha256: "c".repeat(64),
            model_scientific_sha256: "a".repeat(64),
            source_dataset_fingerprint: "b".repeat(64),
            complete_case_frame_sha256: "f".repeat(64),
            usable_replicate_indices_sha256: crate::sha256_serialized(&usable_replicate_indices),
            effect_identity_set_sha256: general_sem_effect_identity_set_sha256_v1(
                &canonical_general_sem_effect_identities_v1(&results),
            ),
            effect_ids,
            interval: CanonicalGeneralSemBootstrapIntervalV1::PercentileType7,
            tail: CanonicalGeneralSemInferenceTailV1::TwoSided,
            confidence_level: 0.95,
            resamples_requested: 10,
            resamples_usable: 9,
            minimum_usable_resamples: 9,
            seed: "42".to_string(),
            workers: 4,
            complete_model_reestimated_per_replicate: true,
            failed_replicates,
        });
        document.capability_cells = Some(vec![
            capability_reference(),
            secondary_capability_reference(),
            general_sem_effect_capability_reference(),
        ]);
        document.general_sem_results = Some(results);
        document
    }

    fn general_sem_moderation_inference_document_fixture() -> CanonicalResultDocumentV2 {
        let mut document = general_sem_interaction_document_fixture();
        let results = document.general_sem_results.as_mut().unwrap();
        results.specific_indirect_effects.clear();
        results.aggregate_effects.clear();
        results.interaction_effects[0].scientific_rescaled_gamma = inferred_effect_value(0.4);
        let effect_ids = results
            .interaction_effects
            .iter()
            .map(|effect| effect.effect_id.clone())
            .collect::<Vec<_>>();
        let effect_identities = canonical_general_sem_effect_identities_v1(results)
            .into_iter()
            .filter(|identity| {
                matches!(
                    identity,
                    CanonicalGeneralSemEffectIdentityV1::InteractionScientificRescaledGamma { .. }
                )
            })
            .collect::<Vec<_>>();
        let failed_replicates = vec![CanonicalGeneralSemFailedReplicateV1 {
            replicate_index: 7,
            reason_code: CanonicalGeneralSemFailedReplicateReasonV1::ConstantInteractionProduct,
            message: "One resampled interaction product was constant.".to_string(),
        }];
        let usable_replicate_indices = (0..10_u32)
            .filter(|replicate_index| *replicate_index != 7)
            .collect::<Vec<_>>();
        results.inference_receipt = Some(CanonicalGeneralSemInferenceReceiptV1 {
            kind: CanonicalGeneralSemInferenceKindV1::CaseBootstrap,
            capability_cell: crate::pls_general_multiple_moderation_bootstrap_capability_cell_v1(),
            capability_dependencies: Vec::new(),
            method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
                .to_string(),
            resampling_operation_version:
                GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
                    .to_string(),
            resampling_stream_version: GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1
                .to_string(),
            quantile_method_version: GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1.to_string(),
            standard_error_method_version: GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1
                .to_string(),
            summation_method_version: GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1.to_string(),
            p_value_method_version: GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1
                .to_string(),
            failure_policy_version: GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1
                .to_string(),
            compilation_artifact_identity_sha256: "d".repeat(64),
            compiled_plan_sha256: "9".repeat(64),
            general_sem_config_sha256: "e".repeat(64),
            recipe_analytical_sha256: "c".repeat(64),
            model_scientific_sha256: "a".repeat(64),
            source_dataset_fingerprint: "b".repeat(64),
            complete_case_frame_sha256: "f".repeat(64),
            usable_replicate_indices_sha256: crate::sha256_serialized(&usable_replicate_indices),
            effect_identity_set_sha256: general_sem_effect_identity_set_sha256_v1(
                &effect_identities,
            ),
            effect_ids,
            interval: CanonicalGeneralSemBootstrapIntervalV1::PercentileType7,
            tail: CanonicalGeneralSemInferenceTailV1::TwoSided,
            confidence_level: 0.95,
            resamples_requested: 10,
            resamples_usable: 9,
            minimum_usable_resamples: 9,
            seed: "42".to_string(),
            workers: 4,
            complete_model_reestimated_per_replicate: true,
            failed_replicates,
        });
        let cells = document.capability_cells.as_mut().unwrap();
        cells.push(crate::pls_general_multiple_moderation_bootstrap_capability_cell_v1());
        cells.sort_by_key(capability_cell_reference_identity_v2);
        document
    }

    fn general_sem_moderated_mediation_inference_document_fixture() -> CanonicalResultDocumentV2 {
        let mut document = general_sem_moderation_inference_document_fixture();
        let supplemental_cell =
            crate::pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1();
        let results = document.general_sem_results.as_mut().unwrap();
        results.joint_stage_structural_coefficients.push(
            CanonicalJointStageStructuralCoefficientResultV1 {
                relation_id: "relation_m_y".into(),
                parameter_id: "parameter_m_y".into(),
                trace: results.interaction_effects[0].trace.clone(),
                source_id: "construct_y".into(),
                target_id: "construct_z".into(),
                role: CanonicalStructuralRelationRoleV1::Structural,
                estimate: effect_value(0.5),
                stage: CanonicalStructuralEstimateStageV1::JointStageTwo,
                method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1.into(),
            },
        );
        results
            .joint_stage_structural_coefficients
            .sort_by(|left, right| left.relation_id.cmp(&right.relation_id));

        let target_id = "sem_moderated_mediation_target_v1_fixture";
        let estimand_id = "estimand:selected_x_m_y";
        let ordered_relation_ids = vec!["relation_focal_1".into(), "relation_m_y".into()];
        let derived_trace = CanonicalGeneralSemResultTraceV1 {
            model_id: document.provenance.model_id.clone(),
            capability_cell: supplemental_cell.clone(),
        };
        results.conditional_indirect_effects = [-1.0, 0.0, 1.0]
            .into_iter()
            .enumerate()
            .map(|(probe_value_index, moderator_value)| {
                CanonicalConditionalIndirectEffectResultV1 {
                    effect_id: crate::conditional_indirect_effect_identity_v1(
                        target_id,
                        probe_value_index as u32,
                    ),
                    target_id: target_id.into(),
                    estimand_id: estimand_id.into(),
                    trace: derived_trace.clone(),
                    moderated_stage: CanonicalModeratedMediationStageV1::FirstStage,
                    interaction_id: "interaction_1".into(),
                    x_id: "construct_x".into(),
                    mediator_id: "construct_y".into(),
                    y_id: "construct_z".into(),
                    moderator_id: "moderator_m".into(),
                    ordered_relation_ids: ordered_relation_ids.clone(),
                    probe_value_index: probe_value_index as u32,
                    moderator_value,
                    value: inferred_effect_value((0.4 + 0.4 * moderator_value) * 0.5),
                }
            })
            .collect();
        results
            .conditional_indirect_effects
            .sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
        results.moderated_mediation_indices = vec![CanonicalModeratedMediationIndexResultV1 {
            effect_id: crate::moderated_mediation_index_identity_v1(target_id),
            target_id: target_id.into(),
            estimand_id: estimand_id.into(),
            trace: derived_trace,
            moderated_stage: CanonicalModeratedMediationStageV1::FirstStage,
            interaction_id: "interaction_1".into(),
            x_id: "construct_x".into(),
            mediator_id: "construct_y".into(),
            y_id: "construct_z".into(),
            moderator_id: "moderator_m".into(),
            ordered_relation_ids,
            value: inferred_effect_value(0.2),
        }];

        let mut effect_ids = results
            .interaction_effects
            .iter()
            .map(|effect| effect.effect_id.clone())
            .chain(
                results
                    .conditional_indirect_effects
                    .iter()
                    .map(|effect| effect.effect_id.clone()),
            )
            .chain(
                results
                    .moderated_mediation_indices
                    .iter()
                    .map(|effect| effect.effect_id.clone()),
            )
            .collect::<Vec<_>>();
        effect_ids.sort();
        let effect_identities = canonical_general_sem_effect_identities_v1(results)
            .into_iter()
            .filter(|identity| {
                matches!(
                    identity,
                    CanonicalGeneralSemEffectIdentityV1::InteractionScientificRescaledGamma { .. }
                        | CanonicalGeneralSemEffectIdentityV1::ConditionalIndirect { .. }
                        | CanonicalGeneralSemEffectIdentityV1::ModeratedMediationIndex { .. }
                )
            })
            .collect::<Vec<_>>();
        let receipt = results.inference_receipt.as_mut().unwrap();
        receipt.capability_cell = supplemental_cell.clone();
        receipt.capability_dependencies = vec![
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            crate::pls_general_multiple_moderation_point_capability_cell_v1(),
        ];
        receipt
            .capability_dependencies
            .sort_by_key(capability_cell_reference_identity_v2);
        receipt.method_version =
            GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1.into();
        receipt.resampling_operation_version =
            GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1.into();
        receipt.effect_ids = effect_ids;
        receipt.effect_identity_set_sha256 =
            general_sem_effect_identity_set_sha256_v1(&effect_identities);

        let cells = document.capability_cells.as_mut().unwrap();
        cells.retain(|cell| {
            cell != &crate::pls_general_multiple_moderation_bootstrap_capability_cell_v1()
        });
        cells.push(RecipeV4CompilerTarget::PlsPlanV2.capability_cell());
        cells.push(supplemental_cell);
        cells.sort_by_key(capability_cell_reference_identity_v2);
        cells.dedup();
        document
    }

    #[test]
    fn valid_microcase_passes() {
        let validation = validate_canonical_result_document_v2(&document_fixture());
        assert!(validation.passed, "{:?}", validation.errors);
        assert!(validation.errors.is_empty());
        assert_eq!(
            canonical_result_use_eligibility_v2(&document_fixture()),
            CanonicalResultUseEligibilityV2 {
                readable: true,
                comparison_eligible: true,
                qualification_export_eligible: true,
                ineligibility: None,
            }
        );
    }

    #[test]
    fn absent_general_sem_extension_preserves_historical_wire_shape() {
        let document = document_fixture();
        let value = serde_json::to_value(&document).unwrap();
        assert!(value.get("general_sem_results").is_none());
        let decoded: CanonicalResultDocumentV2 = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, document);
        assert!(decoded.general_sem_results.is_none());
    }

    #[test]
    fn moderated_mediation_canonical_collections_are_additive_and_identity_bound() {
        let historical = serde_json::to_value(general_sem_results_fixture()).unwrap();
        assert!(historical.get("conditional_indirect_effects").is_none());
        assert!(historical.get("moderated_mediation_indices").is_none());

        let mut results = general_sem_results_fixture();
        let target_id = "sem_moderated_mediation_target_v1_fixture";
        let estimand_id = "estimand:selected_x_m_y";
        let interaction_id = "interaction:m_by_w_to_y";
        let relations = vec!["relation_x_m".to_string(), "relation_m_y".to_string()];
        let trace = CanonicalGeneralSemResultTraceV1 {
            model_id: "model-1".into(),
            capability_cell:
                crate::pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1(),
        };
        results.conditional_indirect_effects = [-1.0, 0.0, 1.0]
            .into_iter()
            .enumerate()
            .map(|(probe_value_index, moderator_value)| {
                CanonicalConditionalIndirectEffectResultV1 {
                    effect_id: crate::conditional_indirect_effect_identity_v1(
                        target_id,
                        probe_value_index as u32,
                    ),
                    target_id: target_id.into(),
                    estimand_id: estimand_id.into(),
                    trace: trace.clone(),
                    moderated_stage: CanonicalModeratedMediationStageV1::SecondStage,
                    interaction_id: interaction_id.into(),
                    x_id: "construct:x".into(),
                    mediator_id: "construct:m".into(),
                    y_id: "construct:y".into(),
                    moderator_id: "construct:w".into(),
                    ordered_relation_ids: relations.clone(),
                    probe_value_index: probe_value_index as u32,
                    moderator_value,
                    value: effect_value((0.4 + 0.2 * moderator_value) * 0.5),
                }
            })
            .collect();
        results
            .conditional_indirect_effects
            .sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
        results.moderated_mediation_indices = vec![CanonicalModeratedMediationIndexResultV1 {
            effect_id: crate::moderated_mediation_index_identity_v1(target_id),
            target_id: target_id.into(),
            estimand_id: estimand_id.into(),
            trace,
            moderated_stage: CanonicalModeratedMediationStageV1::SecondStage,
            interaction_id: interaction_id.into(),
            x_id: "construct:x".into(),
            mediator_id: "construct:m".into(),
            y_id: "construct:y".into(),
            moderator_id: "construct:w".into(),
            ordered_relation_ids: relations,
            value: effect_value(0.1),
        }];

        let encoded = serde_json::to_value(&results).unwrap();
        assert_eq!(
            encoded["conditional_indirect_effects"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert_eq!(
            encoded["moderated_mediation_indices"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            serde_json::from_value::<CanonicalGeneralSemResultsV1>(encoded).unwrap(),
            results
        );
        let identities = canonical_general_sem_effect_identities_v1(&results);
        assert_eq!(
            identities
                .iter()
                .filter(|identity| matches!(
                    identity,
                    CanonicalGeneralSemEffectIdentityV1::ConditionalIndirect { .. }
                ))
                .count(),
            3
        );
        assert!(identities.iter().any(|identity| matches!(
            identity,
            CanonicalGeneralSemEffectIdentityV1::ModeratedMediationIndex { .. }
        )));

        let inference_wire =
            serde_json::to_value(general_sem_inference_document_fixture()).unwrap();
        assert!(
            inference_wire["general_sem_results"]["inference_receipt"]
                .get("capability_dependencies")
                .is_none()
        );
    }

    #[test]
    fn every_general_sem_result_family_roundtrips_and_binds_analytical_identity() {
        let mut document = document_fixture();
        document.general_sem_results = Some(general_sem_results_fixture());

        let validation = validate_canonical_result_document_v2(&document);
        assert!(validation.passed, "{:?}", validation.errors);
        let encoded = serde_json::to_vec(&document).unwrap();
        let decoded: CanonicalResultDocumentV2 = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, document);

        let mut changed = document.clone();
        changed
            .general_sem_results
            .as_mut()
            .unwrap()
            .specific_indirect_effects[0]
            .value
            .estimate = 0.13;
        assert_ne!(
            canonical_analytical_result_json(&document).unwrap(),
            canonical_analytical_result_json(&changed).unwrap()
        );
    }

    #[test]
    fn general_sem_bootstrap_inference_requires_a_complete_exact_receipt() {
        let document = general_sem_inference_document_fixture();
        let validation = validate_canonical_result_document_v2(&document);
        assert!(validation.passed, "{:?}", validation.errors);

        let encoded = serde_json::to_value(&document).unwrap();
        let estimate = &encoded["general_sem_results"]["specific_indirect_effects"][0]["value"];
        assert_eq!(estimate["bootstrap_mean"], serde_json::json!(0.13));
        assert_eq!(estimate["bootstrap_bias"], serde_json::json!(0.01));
        assert_eq!(
            encoded["general_sem_results"]["inference_receipt"]["seed"],
            serde_json::json!("42")
        );

        let mut missing_receipt = document.clone();
        missing_receipt
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt = None;
        assert!(
            validate_canonical_result_document_v2(&missing_receipt)
                .errors
                .iter()
                .any(|error| error.contains("inference fields require inference_receipt"))
        );

        let mut partial_tuple = document.clone();
        partial_tuple
            .general_sem_results
            .as_mut()
            .unwrap()
            .specific_indirect_effects[0]
            .value
            .upper = None;
        assert!(
            validate_canonical_result_document_v2(&partial_tuple)
                .errors
                .iter()
                .any(|error| error.contains(
                    "bootstrap inference fields must be either all absent or all present"
                ))
        );

        let mut different_worker_count = document.clone();
        different_worker_count
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap()
            .workers = 3;
        assert_ne!(
            canonical_result_document_json(&document).unwrap(),
            canonical_result_document_json(&different_worker_count).unwrap()
        );
        assert_eq!(
            canonical_analytical_result_json(&document).unwrap(),
            canonical_analytical_result_json(&different_worker_count).unwrap()
        );
    }

    #[test]
    fn general_sem_bootstrap_receipt_identity_and_capability_tampering_fail_closed() {
        let mut undeclared = general_sem_inference_document_fixture();
        undeclared.capability_cells = Some(vec![capability_reference()]);
        assert!(
            validate_canonical_result_document_v2(&undeclared)
                .errors
                .iter()
                .any(|error| error.contains("references undeclared option cell"))
        );

        let mut wrong_cell = general_sem_inference_document_fixture();
        wrong_cell
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap()
            .capability_cell = capability_reference();
        assert!(
            validate_canonical_result_document_v2(&wrong_cell)
                .errors
                .iter()
                .any(|error| error.contains(
                    "must equal an exact General SEM mediation, moderation, or two-way moderated-mediation full-model bootstrap option cell"
                ))
        );

        let mut changed_effect_set = general_sem_inference_document_fixture();
        changed_effect_set
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap()
            .effect_ids[0] = "effect_other".to_string();
        let errors = validate_canonical_result_document_v2(&changed_effect_set).errors;
        assert!(
            errors.iter().any(
                |error| error.contains("must exactly cover specific and aggregate effect rows")
            )
        );

        let mut changed_typed_identity = general_sem_inference_document_fixture();
        changed_typed_identity
            .general_sem_results
            .as_mut()
            .unwrap()
            .aggregate_effects[0]
            .source_id = "construct_other".to_string();
        let errors = validate_canonical_result_document_v2(&changed_typed_identity).errors;
        assert!(
            errors
                .iter()
                .any(|error| error.contains("effect_identity_set_sha256 does not match"))
        );

        let mut wrong_effect_trace = general_sem_inference_document_fixture();
        wrong_effect_trace
            .general_sem_results
            .as_mut()
            .unwrap()
            .specific_indirect_effects[0]
            .trace
            .capability_cell = capability_reference();
        assert!(
            validate_canonical_result_document_v2(&wrong_effect_trace)
                .errors
                .iter()
                .any(|error| error.contains("must equal the PLS recursive-effects option cell"))
        );
    }

    #[test]
    fn general_sem_bootstrap_receipt_versions_plan_and_failure_ledger_fail_closed() {
        let mut document = general_sem_inference_document_fixture();
        let receipt = document
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap();
        receipt.method_version = "other_method_v1".to_string();
        receipt.interval = CanonicalGeneralSemBootstrapIntervalV1::Bca;
        receipt.tail = CanonicalGeneralSemInferenceTailV1::OneSidedUpper;
        receipt.seed = "01".to_string();
        receipt.resamples_usable = 8;
        receipt.failed_replicates[0].replicate_index = 10;

        let errors = validate_canonical_result_document_v2(&document).errors;
        for expected in [
            "method_version must equal",
            "interval must equal percentile_type7",
            "tail must equal two_sided",
            "seed must be a canonical decimal u64",
            "requested count must equal usable plus failed replicates",
            "replicate_index is outside the requested plan",
            "resamples_usable contradicts the failure ledger",
            "usable_replicate_indices_sha256 does not match",
        ] {
            assert!(
                errors.iter().any(|error| error.contains(expected)),
                "missing {expected:?} in {errors:?}"
            );
        }
    }

    #[test]
    fn moderation_bootstrap_covers_only_typed_scientific_gamma_targets() {
        let document = general_sem_moderation_inference_document_fixture();
        let validation = validate_canonical_result_document_v2(&document);
        assert!(validation.passed, "{:?}", validation.errors);

        let encoded = serde_json::to_value(&document).unwrap();
        let gamma =
            &encoded["general_sem_results"]["interaction_effects"][0]["scientific_rescaled_gamma"];
        assert!(approximately_equal(
            gamma["bootstrap_mean"].as_f64().unwrap(),
            0.41
        ));
        assert!(encoded["general_sem_results"]["interaction_effects"][0]
            ["standardized_product_coefficient"]
            .get("bootstrap_mean")
            .is_none());
        let identities = canonical_general_sem_effect_identities_v1(
            document.general_sem_results.as_ref().unwrap(),
        );
        assert!(identities.iter().any(|identity| matches!(
            identity,
            CanonicalGeneralSemEffectIdentityV1::InteractionScientificRescaledGamma { .. }
        )));
        let gamma_identity =
            serde_json::to_value(
                identities
                    .iter()
                    .find(|identity| {
                        matches!(
                    identity,
                    CanonicalGeneralSemEffectIdentityV1::InteractionScientificRescaledGamma { .. }
                )
                    })
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            gamma_identity["kind"],
            serde_json::json!("interaction_scientific_rescaled_gamma")
        );
        assert_eq!(
            gamma_identity["generated_product_column_id"],
            serde_json::json!("generated_interaction_1_product")
        );
        assert_eq!(
            gamma_identity["stage_one_model_scientific_sha256"],
            "d".repeat(64)
        );
        assert_eq!(
            gamma_identity["product_scale_version"],
            serde_json::json!(GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1)
        );
        assert_eq!(
            gamma_identity["method_version"],
            serde_json::json!(GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1)
        );
    }

    #[test]
    fn moderation_bootstrap_rejects_missing_extra_and_tampered_gamma_authority() {
        let fixture = general_sem_moderation_inference_document_fixture();

        let mut missing = fixture.clone();
        missing
            .general_sem_results
            .as_mut()
            .unwrap()
            .interaction_effects[0]
            .scientific_rescaled_gamma
            .upper = None;
        let errors = validate_canonical_result_document_v2(&missing).errors;
        assert!(errors.iter().any(|error| {
            error.contains("bootstrap inference fields must be either all absent or all present")
        }));
        assert!(errors.iter().any(|error| {
            error.contains("complete inference fields for every scientific rescaled gamma")
        }));

        let mut wrong_target = fixture.clone();
        wrong_target
            .general_sem_results
            .as_mut()
            .unwrap()
            .interaction_effects[0]
            .moderator_id = "moderator_other".to_string();
        assert!(
            validate_canonical_result_document_v2(&wrong_target)
                .errors
                .iter()
                .any(|error| error.contains("effect_identity_set_sha256 does not match"))
        );

        let locked_identity_mutations: [(&str, fn(&mut CanonicalInteractionEffectResultV1)); 4] = [
            ("generated product", |effect| {
                effect.generated_product_column_id = "generated_other_product".to_string();
            }),
            ("stage-one digest", |effect| {
                effect.stage_one_model_scientific_sha256 = "1".repeat(64);
            }),
            ("product scale", |effect| {
                effect.product_scale_version = "other_product_scale_v1".to_string();
            }),
            ("point method", |effect| {
                effect.method_version = "other_point_method_v1".to_string();
            }),
        ];
        for (label, mutate) in locked_identity_mutations {
            let mut changed = fixture.clone();
            mutate(
                &mut changed
                    .general_sem_results
                    .as_mut()
                    .unwrap()
                    .interaction_effects[0],
            );
            let errors = validate_canonical_result_document_v2(&changed).errors;
            assert!(
                errors
                    .iter()
                    .any(|error| error.contains("effect_identity_set_sha256 does not match")),
                "{label} was not bound by the typed gamma identity: {errors:?}"
            );
        }

        let mut extra_target = fixture.clone();
        extra_target
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap()
            .effect_ids
            .push("relation_interaction_extra_effect".to_string());
        assert!(
            validate_canonical_result_document_v2(&extra_target)
                .errors
                .iter()
                .any(|error| error.contains(
                    "effect_ids must exactly cover scientific rescaled gamma interaction rows"
                ))
        );
    }

    #[test]
    fn moderation_bootstrap_rejects_inference_on_every_point_only_surface() {
        let fixture = general_sem_moderation_inference_document_fixture();
        let assert_gamma_only_error =
            |document: &CanonicalResultDocumentV2| {
                let errors = validate_canonical_result_document_v2(document).errors;
                assert!(
                    errors.iter().any(|error| error
                        .contains("permits inference only for scientific_rescaled_gamma")),
                    "{errors:?}"
                );
            };

        let mut standardized_product = fixture.clone();
        standardized_product
            .general_sem_results
            .as_mut()
            .unwrap()
            .interaction_effects[0]
            .standardized_product_coefficient = inferred_effect_value(0.2);
        assert_gamma_only_error(&standardized_product);

        let mut joint_stage = fixture.clone();
        joint_stage
            .general_sem_results
            .as_mut()
            .unwrap()
            .joint_stage_structural_coefficients[0]
            .estimate = inferred_effect_value(0.4);
        assert_gamma_only_error(&joint_stage);

        let mut conditional = fixture.clone();
        conditional
            .general_sem_results
            .as_mut()
            .unwrap()
            .conditional_effects[0]
            .value = inferred_effect_value(0.3);
        assert_gamma_only_error(&conditional);

        let mut plot_band = fixture.clone();
        let point = &mut plot_band
            .general_sem_results
            .as_mut()
            .unwrap()
            .interaction_plots[0]
            .series[0]
            .points[0];
        point.lower = Some(point.predicted_value - 0.1);
        point.upper = Some(point.predicted_value + 0.1);
        assert_gamma_only_error(&plot_band);

        let mut higher_order = fixture;
        higher_order
            .general_sem_results
            .as_mut()
            .unwrap()
            .higher_order_stages[1]
            .relation_estimates[0]
            .value = inferred_effect_value(0.31);
        let errors = validate_canonical_result_document_v2(&higher_order).errors;
        assert!(
            errors.iter().any(|error| error.contains(
                "higher_order_inference_receipt is required when higher-order relations contain inference"
            )),
            "{errors:?}"
        );
    }

    #[test]
    fn moderation_bootstrap_receipt_versions_and_point_trace_fail_closed() {
        let mut document = general_sem_moderation_inference_document_fixture();
        let results = document.general_sem_results.as_mut().unwrap();
        results.inference_receipt.as_mut().unwrap().method_version =
            GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1.to_string();
        results
            .inference_receipt
            .as_mut()
            .unwrap()
            .resampling_operation_version =
            GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1.to_string();
        results.interaction_effects[0].trace.capability_cell = capability_reference();
        let errors = validate_canonical_result_document_v2(&document).errors;
        for expected in [
            GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
            GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
            "trace.capability_cell must equal the General SEM multiple two-way moderation point option cell",
        ] {
            assert!(
                errors.iter().any(|error| error.contains(expected)),
                "missing {expected:?} in {errors:?}"
            );
        }
    }

    #[test]
    fn moderated_mediation_receipt_and_five_target_formulas_fail_closed() {
        let document = general_sem_moderated_mediation_inference_document_fixture();
        let validation = validate_canonical_result_document_v2(&document);
        assert!(validation.passed, "{:?}", validation.errors);

        let mut formula_tamper = document.clone();
        formula_tamper
            .general_sem_results
            .as_mut()
            .unwrap()
            .conditional_indirect_effects[0]
            .value
            .estimate += 0.01;
        assert!(
            validate_canonical_result_document_v2(&formula_tamper)
                .errors
                .iter()
                .any(|error| error.contains("contradicts the bounded formula"))
        );

        let mut dependency_tamper = document;
        dependency_tamper
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap()
            .capability_dependencies
            .pop();
        assert!(
            validate_canonical_result_document_v2(&dependency_tamper)
                .errors
                .iter()
                .any(|error| error.contains("must exactly declare the base PLS"))
        );
    }

    #[test]
    fn point_only_general_sem_estimates_keep_the_legacy_extension_shape() {
        let mut document = document_fixture();
        document.general_sem_results = Some(general_sem_results_fixture());
        let value = serde_json::to_value(&document).unwrap();
        let results = &value["general_sem_results"];
        assert!(results.get("inference_receipt").is_none());
        for absent in [
            "cbsem_parameters",
            "cbsem_bootstrap_receipt",
            "cbsem_bootstrap_inference",
        ] {
            assert!(results.get(absent).is_none(), "unexpected {absent}");
        }
        let estimate = &results["specific_indirect_effects"][0]["value"];
        for absent in [
            "bootstrap_mean",
            "bootstrap_bias",
            "standard_error",
            "lower",
            "upper",
            "p_value",
        ] {
            assert!(estimate.get(absent).is_none(), "unexpected {absent}");
        }
    }

    #[test]
    fn interaction_authority_roundtrips_and_scale_or_cross_reference_tampering_fails_closed() {
        let document = general_sem_interaction_document_fixture();
        let validation = validate_canonical_result_document_v2(&document);
        assert!(validation.passed, "{:?}", validation.errors);
        let encoded = canonical_result_document_json(&document).unwrap();
        let decoded: CanonicalResultDocumentV2 = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, document);

        let mut wrong_gamma = document.clone();
        wrong_gamma
            .general_sem_results
            .as_mut()
            .unwrap()
            .interaction_effects[0]
            .scientific_rescaled_gamma
            .estimate = 0.41;
        assert!(
            validate_canonical_result_document_v2(&wrong_gamma)
                .errors
                .iter()
                .any(|error| error.contains("scientific_rescaled_gamma must equal"))
        );

        let mut wrong_projection = document.clone();
        let source_model_digest = wrong_projection.provenance.model_digest.clone();
        wrong_projection
            .general_sem_results
            .as_mut()
            .unwrap()
            .interaction_effects[0]
            .stage_one_model_scientific_sha256 = source_model_digest;
        assert!(
            validate_canonical_result_document_v2(&wrong_projection)
                .errors
                .iter()
                .any(|error| error.contains("projected interaction-free scoring model"))
        );

        let mut missing_cross_reference = document.clone();
        missing_cross_reference
            .general_sem_results
            .as_mut()
            .unwrap()
            .conditional_effects[0]
            .interaction_effect_id = None;
        assert!(
            validate_canonical_result_document_v2(&missing_cross_reference)
                .errors
                .iter()
                .any(|error| error.contains("interaction_effect_id is required"))
        );

        let mut wrong_probe_policy = document.clone();
        wrong_probe_policy
            .general_sem_results
            .as_mut()
            .unwrap()
            .conditional_effect_probes[0]
            .values = CanonicalConditionalProbeValuesResultV1::Explicit {
            values: vec![-2.0, 0.0, 1.0],
        };
        assert!(
            validate_canonical_result_document_v2(&wrong_probe_policy)
                .errors
                .iter()
                .any(|error| error.contains("frozen standardized -1/0/+1 interaction policy"))
        );

        let mut omitted_conditional_row = document.clone();
        omitted_conditional_row
            .general_sem_results
            .as_mut()
            .unwrap()
            .conditional_effects
            .pop();
        assert!(
            validate_canonical_result_document_v2(&omitted_conditional_row)
                .errors
                .iter()
                .any(|error| error.contains("exactly three conditional rows"))
        );

        let mut omitted_plot = document.clone();
        omitted_plot
            .general_sem_results
            .as_mut()
            .unwrap()
            .interaction_plots
            .clear();
        assert!(
            validate_canonical_result_document_v2(&omitted_plot)
                .errors
                .iter()
                .any(|error| error.contains("exactly one cross-referenced interaction plot"))
        );

        let mut wrong_trace = document;
        wrong_trace
            .general_sem_results
            .as_mut()
            .unwrap()
            .interaction_effects[0]
            .trace
            .capability_cell = capability_reference();
        assert!(
            validate_canonical_result_document_v2(&wrong_trace)
                .errors
                .iter()
                .any(|error| error.contains("multiple two-way moderation point option cell"))
        );
    }

    #[test]
    fn joint_stage_structural_ledger_is_exact_sorted_point_only_moderation_authority() {
        let document = general_sem_interaction_document_fixture();
        let validation = validate_canonical_result_document_v2(&document);
        assert!(validation.passed, "{:?}", validation.errors);

        let mut omitted = document.clone();
        omitted
            .general_sem_results
            .as_mut()
            .unwrap()
            .joint_stage_structural_coefficients
            .clear();
        assert!(
            validate_canonical_result_document_v2(&omitted)
                .errors
                .iter()
                .any(|error| error.contains(
                    "joint_stage_structural_coefficients and interaction_effects must both be present"
                ))
        );

        let mut wrong_trace_and_method = document.clone();
        let coefficient = &mut wrong_trace_and_method
            .general_sem_results
            .as_mut()
            .unwrap()
            .joint_stage_structural_coefficients[0];
        coefficient.trace.capability_cell = capability_reference();
        coefficient.method_version = "other_joint_stage_method_v1".into();
        coefficient.estimate.standard_error = Some(0.1);
        let errors = validate_canonical_result_document_v2(&wrong_trace_and_method).errors;
        for expected in [
            "must equal the General SEM multiple two-way moderation point option cell",
            "method_version must equal",
            "estimate must contain point estimation only",
        ] {
            assert!(
                errors.iter().any(|error| error.contains(expected)),
                "missing {expected:?} in {errors:?}"
            );
        }

        let mut duplicate_parameter = document.clone();
        let mut duplicate = duplicate_parameter
            .general_sem_results
            .as_ref()
            .unwrap()
            .joint_stage_structural_coefficients[0]
            .clone();
        duplicate.relation_id = "relation_focal_2".into();
        duplicate_parameter
            .general_sem_results
            .as_mut()
            .unwrap()
            .joint_stage_structural_coefficients
            .push(duplicate);
        assert!(
            validate_canonical_result_document_v2(&duplicate_parameter)
                .errors
                .iter()
                .any(|error| error.contains("parameter_id is duplicated"))
        );

        let mut unsorted = duplicate_parameter;
        let ledger = &mut unsorted
            .general_sem_results
            .as_mut()
            .unwrap()
            .joint_stage_structural_coefficients;
        ledger[1].parameter_id = "parameter_focal_2".into();
        ledger.swap(0, 1);
        assert!(
            validate_canonical_result_document_v2(&unsorted)
                .errors
                .iter()
                .any(|error| error.contains("must be ordered by exact stable identifier"))
        );
    }

    #[test]
    fn general_sem_identity_path_and_aggregate_contradictions_fail_closed() {
        let mut document = document_fixture();
        let mut results = general_sem_results_fixture();
        results.specific_indirect_effects[0].trace.model_id = "model-other".to_string();
        results.specific_indirect_effects[0].trace.capability_cell =
            secondary_capability_reference();
        results.specific_indirect_effects[0].ordered_relation_ids =
            vec!["relation_a".to_string(), "relation_a".to_string()];
        results.aggregate_effects[0].target_id = "construct_x".to_string();
        results.aggregate_effects[0].estimand_id =
            results.specific_indirect_effects[0].estimand_id.clone();
        results.aggregate_effects[0].effect_id = results.aggregate_effects[0].estimand_id.clone();
        results.aggregate_effects[1].effect_id =
            results.specific_indirect_effects[0].effect_id.clone();
        document.general_sem_results = Some(results);

        let validation = validate_canonical_result_document_v2(&document);
        assert!(!validation.passed);
        for expected in [
            "model_id must equal provenance.model_id",
            "references undeclared option cell",
            "ordered_relation_ids contains duplicate IDs",
            "requires distinct source_id and target_id",
            "estimand_id is duplicated across effect sections",
            "effect_id is duplicated across effect sections",
        ] {
            assert!(
                validation
                    .errors
                    .iter()
                    .any(|error| error.contains(expected)),
                "missing {expected:?} in {:?}",
                validation.errors
            );
        }
    }

    #[test]
    fn conditional_probe_effect_and_plot_contradictions_fail_closed() {
        let mut document = document_fixture();
        let mut results = general_sem_results_fixture();
        results.conditional_effect_probes[1].values =
            CanonicalConditionalProbeValuesResultV1::Explicit {
                values: vec![0.0, f64::NAN],
            };
        results.conditional_effects[0].probe_id = "probe_missing".to_string();
        results.conditional_effects[0].moderator_value = f64::INFINITY;
        results.interaction_plots[0].series[0].probe_value_index = 99;
        results.interaction_plots[0].series[0].points[0].lower = Some(0.5);
        results.interaction_plots[0].series[0].points[0].upper = Some(0.4);
        results.interaction_plots[0].series[1].points[1].focal_value = 2.0;
        document.general_sem_results = Some(results);

        let validation = validate_canonical_result_document_v2(&document);
        assert!(!validation.passed);
        for expected in [
            "values.values[1] must be finite",
            "probe_id references a missing conditional-effect probe",
            "moderator_value must be finite",
            "probe_value_index is outside its probe",
            "lower must not exceed upper",
            "common focal-value grid",
        ] {
            assert!(
                validation
                    .errors
                    .iter()
                    .any(|error| error.contains(expected)),
                "missing {expected:?} in {:?}",
                validation.errors
            );
        }
    }

    #[test]
    fn hoc_cbsem_fit_and_identification_contradictions_fail_closed() {
        let mut document = document_fixture();
        let mut results = general_sem_results_fixture();
        results.higher_order_stages[1].stage_number = 1;
        results.cbsem_fit[0].chi_square = f64::NAN;
        results.cbsem_fit[0].degrees_of_freedom = 0;
        results.cbsem_fit[0].chi_square_p_value = Some(2.0);
        results.cbsem_fit[0].rmsea = None;
        results.cbsem_fit[0].rmsea_interval = Some(CanonicalGeneralSemIntervalV1 {
            confidence_level: 1.0,
            lower: 0.2,
            upper: 0.1,
        });
        results.identification_diagnostics[0].subject_id = "model-other".to_string();
        results.identification_diagnostics[0].message.clear();
        results.identification_diagnostics[0].degrees_of_freedom = Some(-1);
        document.general_sem_results = Some(results);

        let validation = validate_canonical_result_document_v2(&document);
        assert!(!validation.passed);
        for expected in [
            "stage_number, approach, and stage kind are inconsistent",
            "duplicates a higher-order construct stage",
            "chi_square must be finite and nonnegative",
            "chi_square_p_value must be finite and between 0 and 1",
            "chi_square_p_value must be absent when degrees_of_freedom is zero",
            "rmsea_interval requires an rmsea estimate",
            "confidence_level must be finite and between 0 and 1",
            "lower must not exceed upper",
            "subject_id must equal provenance.model_id for model scope",
            "message must be nonempty",
            "cannot be identified with negative degrees_of_freedom",
        ] {
            assert!(
                validation
                    .errors
                    .iter()
                    .any(|error| error.contains(expected)),
                "missing {expected:?} in {:?}",
                validation.errors
            );
        }
    }

    #[test]
    fn general_sem_extension_is_versioned_canonical_and_strict() {
        let mut document = document_fixture();
        let mut results = general_sem_results_fixture();
        results.schema_version = 2;
        results.aggregate_effects.reverse();
        document.general_sem_results = Some(results);
        let validation = validate_canonical_result_document_v2(&document);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("general_sem_results.schema_version must equal 1"))
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("must be ordered by exact stable identifier"))
        );

        let mut empty = document_fixture();
        empty.general_sem_results = Some(CanonicalGeneralSemResultsV1 {
            schema_version: CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION,
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
            cbsem_parameters: Vec::new(),
            cbsem_fit: Vec::new(),
            identification_diagnostics: Vec::new(),
            cbsem_bootstrap_receipt: None,
            cbsem_bootstrap_inference: Vec::new(),
        });
        assert!(
            validate_canonical_result_document_v2(&empty)
                .errors
                .iter()
                .any(|error| error.contains("at least one typed result section"))
        );

        let mut value = serde_json::to_value(document_fixture()).unwrap();
        value.as_object_mut().unwrap().insert(
            "general_sem_results".to_string(),
            serde_json::json!({
                "schema_version": 1,
                "specific_indirect_effects": [],
                "unexpected": true
            }),
        );
        assert!(serde_json::from_value::<CanonicalResultDocumentV2>(value).is_err());
    }

    #[test]
    fn multi_capability_sets_roundtrip_with_primary_and_table_attribution() {
        let mut document = document_fixture();
        let capabilities = vec![capability_reference(), secondary_capability_reference()];
        document.capability_cells = Some(capabilities.clone());
        document.sections[0].capability_cells = Some(capabilities.clone());
        document.tables[0].capability_cells = Some(capabilities);

        let validation = validate_canonical_result_document_v2(&document);
        assert!(validation.passed, "{:?}", validation.errors);
        let encoded = serde_json::to_vec(&document).unwrap();
        let decoded: CanonicalResultDocumentV2 = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, document);
        assert_eq!(
            canonical_result_use_eligibility_v2(&decoded),
            CanonicalResultUseEligibilityV2 {
                readable: true,
                comparison_eligible: true,
                qualification_export_eligible: true,
                ineligibility: None,
            }
        );
    }

    #[test]
    fn capability_set_order_duplicates_primary_and_cross_level_tampering_fail_closed() {
        let primary = capability_reference();
        let secondary = secondary_capability_reference();
        let mut unsorted = document_fixture();
        unsorted.capability_cells = Some(vec![secondary.clone(), primary.clone()]);
        unsorted.sections[0].capability_cells = Some(vec![primary.clone(), secondary.clone()]);
        unsorted.tables[0].capability_cells = Some(vec![primary.clone(), secondary.clone()]);
        assert!(
            validate_canonical_result_document_v2(&unsorted)
                .errors
                .iter()
                .any(|error| error.contains("must be ordered by exact option-cell identity"))
        );

        let mut duplicate = document_fixture();
        duplicate.capability_cells = Some(vec![primary.clone(), primary.clone()]);
        assert!(
            validate_canonical_result_document_v2(&duplicate)
                .errors
                .iter()
                .any(|error| error.contains("contains duplicate references"))
        );

        let mut missing_primary = document_fixture();
        missing_primary.capability_cells = Some(vec![secondary.clone()]);
        missing_primary.sections[0].capability_cells = Some(vec![secondary.clone()]);
        missing_primary.tables[0].capability_cells = Some(vec![secondary.clone()]);
        assert!(
            validate_canonical_result_document_v2(&missing_primary)
                .errors
                .iter()
                .any(|error| error.contains("must include provenance.capability_cell"))
        );

        let mut undeclared_table = document_fixture();
        undeclared_table.tables[0].capability_cells = Some(vec![secondary.clone()]);
        assert!(
            validate_canonical_result_document_v2(&undeclared_table)
                .errors
                .iter()
                .any(|error| error.contains("table paths references an undeclared option cell"))
        );

        let mut incomplete_section = document_fixture();
        incomplete_section.capability_cells = Some(vec![primary.clone(), secondary.clone()]);
        incomplete_section.tables[0].capability_cells =
            Some(vec![primary.clone(), secondary.clone()]);
        incomplete_section.sections[0].capability_cells = Some(vec![primary]);
        assert!(
            validate_canonical_result_document_v2(&incomplete_section)
                .errors
                .iter()
                .any(|error| error.contains("section structural is missing table option cell"))
        );

        let mut partial_legacy = document_fixture();
        partial_legacy.capability_cells = None;
        assert!(
            validate_canonical_result_document_v2(&partial_legacy)
                .errors
                .iter()
                .any(|error| error.contains("cannot declare capability_cells without a document"))
        );
    }

    #[test]
    fn capability_sets_are_part_of_analytical_identity() {
        let single = document_fixture();
        let mut multiple = document_fixture();
        let capabilities = vec![capability_reference(), secondary_capability_reference()];
        multiple.capability_cells = Some(capabilities.clone());
        multiple.sections[0].capability_cells = Some(capabilities.clone());
        multiple.tables[0].capability_cells = Some(capabilities);
        assert_ne!(
            canonical_analytical_result_json(&single).unwrap(),
            canonical_analytical_result_json(&multiple).unwrap()
        );
    }

    #[test]
    fn duplicate_dangling_nonfinite_type_and_row_errors_fail_closed() {
        let mut document = document_fixture();
        document.sections.push(document.sections[0].clone());
        document.tables[0].rows[0].cells = vec![CanonicalResultCell::Number {
            value: f64::NAN,
            display: None,
        }];
        document.tables[0].rows.push(CanonicalResultRow {
            id: "wrong_type".to_string(),
            cells: vec![
                CanonicalResultCell::Boolean { value: true },
                CanonicalResultCell::Number {
                    value: f64::INFINITY,
                    display: None,
                },
            ],
        });
        document.charts[0].source_table_id = Some("missing_table".to_string());
        document.presentation.default_section_id = Some("missing_section".to_string());

        let validation = validate_canonical_result_document_v2(&document);
        assert!(!validation.passed);
        for expected in [
            "sections contains duplicate IDs",
            "has 1 cells; expected 2",
            "is boolean; expected text or missing",
            "must be finite",
            "references missing table missing_table",
            "default_section_id is missing",
        ] {
            assert!(
                validation
                    .errors
                    .iter()
                    .any(|error| error.contains(expected)),
                "missing {expected:?} in {:?}",
                validation.errors
            );
        }
    }

    #[test]
    fn canonical_json_is_deterministic_for_object_key_order() {
        let first = document_fixture();
        let first_json = serde_json::to_string(&first).unwrap();
        let value = serde_json::from_str::<Value>(&first_json).unwrap();
        let reordered = stable_value(value);
        let second: CanonicalResultDocumentV2 = serde_json::from_value(reordered).unwrap();

        assert_eq!(
            canonical_result_document_json(&first).unwrap(),
            canonical_result_document_json(&second).unwrap()
        );
    }

    #[test]
    fn analytical_projection_ignores_presentation_workers_timing_and_display_caches() {
        let first = document_fixture();
        let mut second = document_fixture();
        second.presentation.precision = 6;
        second.presentation.chart_defaults.palette = Some("high_contrast".to_string());
        second.tables[0].rows[0].cells[1] = CanonicalResultCell::Number {
            value: 0.42,
            display: Some("0.420000".to_string()),
        };
        second.charts[0].display.palette = Some("journal_mono".to_string());
        second.provenance.workers = 1;
        second.provenance.completed_at = "2026-08-14T00:00:09Z".to_string();

        assert_ne!(
            canonical_result_document_json(&first).unwrap(),
            canonical_result_document_json(&second).unwrap()
        );
        assert_eq!(
            canonical_analytical_result_json(&first).unwrap(),
            canonical_analytical_result_json(&second).unwrap()
        );
    }

    #[test]
    fn analytical_projection_changes_with_analytical_values_and_order() {
        let first = document_fixture();
        let mut changed_value = document_fixture();
        changed_value.tables[0].rows[0].cells[1] = CanonicalResultCell::Number {
            value: 0.43,
            display: Some("0.4300".to_string()),
        };
        assert_ne!(
            canonical_analytical_result_json(&first).unwrap(),
            canonical_analytical_result_json(&changed_value).unwrap()
        );

        let mut changed_order = document_fixture();
        let row = changed_order.tables[0].rows[0].clone();
        changed_order.tables[0].rows.push(CanonicalResultRow {
            id: "z_to_y".to_string(),
            ..row
        });
        let before = canonical_analytical_result_json(&changed_order).unwrap();
        changed_order.tables[0].rows.reverse();
        assert_ne!(
            before,
            canonical_analytical_result_json(&changed_order).unwrap()
        );
    }

    #[test]
    fn historical_string_tables_migrate_losslessly_without_numeric_inference() {
        let migrated = canonical_result_document_from_legacy_tables(
            LegacyResultMigrationContext {
                document_id: "historical.result:1".to_string(),
                title: "Historical result".to_string(),
                provenance: provenance(),
            },
            vec![LegacyStringResultTable {
                id: "legacy_paths".to_string(),
                title: "Paths".to_string(),
                columns: vec!["Path".to_string(), "Estimate".to_string()],
                rows: vec![vec!["X → Y".to_string(), "0.4200".to_string()]],
                warning: Some(
                    "This result was created by a historical method version.".to_string(),
                ),
            }],
        );

        let validation = validate_canonical_result_document_v2(&migrated);
        assert!(validation.passed, "{:?}", validation.errors);
        assert_eq!(
            migrated.tables[0].rows[0].cells[1],
            CanonicalResultCell::Text {
                value: "0.4200".to_string()
            }
        );
        assert_eq!(migrated.notices.len(), 1);
        assert!(migrated.capability_cells.is_none());
        assert!(migrated.sections[0].capability_cells.is_none());
        assert!(migrated.tables[0].capability_cells.is_none());
        assert_eq!(
            canonical_result_use_eligibility_v2(&migrated),
            CanonicalResultUseEligibilityV2 {
                readable: true,
                comparison_eligible: false,
                qualification_export_eligible: false,
                ineligibility: Some(
                    CanonicalResultQualificationIneligibilityV2::LegacyCapabilityAttributionMissing
                ),
            }
        );
    }

    fn cbsem_v3_canonical_document_fixture() -> CanonicalResultDocumentV2 {
        let mut document = document_fixture();
        let point_cell = crate::cbsem_general_sem_ml_capability_cell_v1();
        let bootstrap_cell = crate::cbsem_recursive_sem_bootstrap_capability_cell_v1();
        document.provenance.capability_cell = point_cell.clone();
        document.provenance.method_version = "cbsem_general_sem_ml_v1".into();
        document.capability_cells = Some(vec![point_cell.clone(), bootstrap_cell.clone()]);
        for section in &mut document.sections {
            section.capability_cells = Some(vec![point_cell.clone()]);
        }
        for table in &mut document.tables {
            table.capability_cells = Some(vec![point_cell.clone()]);
        }
        let point_trace = CanonicalGeneralSemResultTraceV1 {
            model_id: document.provenance.model_id.clone(),
            capability_cell: point_cell,
        };
        let bootstrap_trace = CanonicalGeneralSemResultTraceV1 {
            model_id: document.provenance.model_id.clone(),
            capability_cell: bootstrap_cell.clone(),
        };
        document.general_sem_results = Some(CanonicalGeneralSemResultsV1 {
            schema_version: CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION,
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
            cbsem_parameters: vec![CanonicalCbsemParameterResultV1 {
                parameter_id: "parameter_loading_x1".into(),
                trace: point_trace,
                role: CanonicalCbsemParameterRoleV1::Loading,
                target: CanonicalCbsemParameterTargetV1::Loading {
                    factor_id: "construct_x".into(),
                    indicator_id: "observed_x1".into(),
                },
                relation_id: Some("relation_loading_x1".into()),
                state: CanonicalCbsemParameterStateV1::Free {
                    equality_label: Some("metric equality".into()),
                    lower: Some(-2.0),
                    upper: Some(2.0),
                },
                estimate: 0.8,
                standard_error: Some(0.1),
                z_value: Some(8.0),
                p_value: Some(0.0),
                standardized_estimate: Some(0.75),
            }],
            cbsem_fit: Vec::new(),
            identification_diagnostics: Vec::new(),
            cbsem_bootstrap_receipt: Some(CanonicalCbsemBootstrapReceiptV1 {
                capability_cell: bootstrap_cell,
                method_version: CBSEM_RECURSIVE_SEM_BOOTSTRAP_METHOD_VERSION_V1.into(),
                resampling_operation_version: CBSEM_RECURSIVE_SEM_BOOTSTRAP_OPERATION_VERSION_V1
                    .into(),
                quantile_method_version: GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1.into(),
                compiled_plan_sha256: "d".repeat(64),
                base_plan_sha256: "e".repeat(64),
                parameter_inventory_sha256: crate::sha256_serialized(&vec!["parameter_loading_x1"]),
                model_scientific_sha256: document.provenance.model_digest.clone(),
                general_sem_config_sha256: "1".repeat(64),
                recipe_analytical_sha256: document.provenance.recipe_digest.clone(),
                source_dataset_fingerprint: document.provenance.dataset_fingerprint.clone(),
                complete_case_frame_sha256: "2".repeat(64),
                usable_replicate_indices_sha256: "3".repeat(64),
                confidence_level: 0.95,
                resamples_requested: 500,
                resamples_usable: 500,
                minimum_usable_resamples: 450,
                seed: document.provenance.seed.unwrap().to_string(),
                workers: u32::try_from(document.provenance.workers).unwrap(),
                complete_model_reestimated_per_replicate: true,
                failed_replicates: Vec::new(),
            }),
            cbsem_bootstrap_inference: vec![CanonicalCbsemBootstrapParameterInferenceV1 {
                parameter_id: "parameter_loading_x1".into(),
                trace: bootstrap_trace,
                point_estimate: 0.8,
                outcome: CanonicalCbsemBootstrapInferenceOutcomeV1::Available {
                    value: CanonicalGeneralSemEstimateV1 {
                        estimate: 0.8,
                        bootstrap_mean: Some(0.81),
                        bootstrap_bias: Some(0.01),
                        standard_error: Some(0.05),
                        lower: Some(0.70),
                        upper: Some(0.90),
                        p_value: Some(1.0 / 501.0),
                        bootstrap_usable_replicates: Some(500),
                        bootstrap_two_sided_exceedances: Some(0),
                    },
                },
            }],
        });
        document
    }

    #[test]
    fn cbsem_v3_parameter_and_recursive_bootstrap_contracts_round_trip_and_fail_closed() {
        let document = cbsem_v3_canonical_document_fixture();
        let validation = validate_canonical_result_document_v2(&document);
        assert!(validation.passed, "{:?}", validation.errors);
        let encoded = canonical_result_document_json(&document).unwrap();
        let decoded: CanonicalResultDocumentV2 = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, document);

        let mut bound_tamper = document.clone();
        let CanonicalCbsemParameterStateV1::Free { upper, .. } = &mut bound_tamper
            .general_sem_results
            .as_mut()
            .unwrap()
            .cbsem_parameters[0]
            .state
        else {
            unreachable!()
        };
        *upper = Some(0.5);
        assert!(
            validate_canonical_result_document_v2(&bound_tamper)
                .errors
                .iter()
                .any(|error| error.contains("estimate must satisfy its declared bounds"))
        );

        let mut fixed_inference = document.clone();
        let fixed_row = &mut fixed_inference
            .general_sem_results
            .as_mut()
            .unwrap()
            .cbsem_parameters[0];
        fixed_row.state = CanonicalCbsemParameterStateV1::Fixed { value: 0.8 };
        fixed_row.standard_error = None;
        fixed_row.z_value = None;
        fixed_row.p_value = None;
        assert!(
            validate_canonical_result_document_v2(&fixed_inference)
                .errors
                .iter()
                .any(|error| error.contains("cannot publish inference for a fixed parameter"))
        );

        let mut parameter_tamper = document;
        parameter_tamper
            .general_sem_results
            .as_mut()
            .unwrap()
            .cbsem_bootstrap_inference[0]
            .parameter_id = "parameter_missing".into();
        assert!(
            validate_canonical_result_document_v2(&parameter_tamper)
                .errors
                .iter()
                .any(|error| error.contains("must reference cbsem_parameters"))
        );
    }

    #[test]
    fn serde_rejects_unknown_fields() {
        let mut value = serde_json::to_value(document_fixture()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected".to_string(), Value::Bool(true));
        assert!(serde_json::from_value::<CanonicalResultDocumentV2>(value).is_err());
    }

    #[test]
    fn provenance_constraints_fail_closed() {
        let mut document = document_fixture();
        document.provenance.recipe_digest = "ABC".to_string();
        document.provenance.workers = 0;
        document.provenance.seed = Some(MAX_SAFE_INTEGER + 1);
        document.provenance.completed_at = "2026-08-13T23:59:59Z".to_string();
        document.presentation.precision = 13;

        let validation = validate_canonical_result_document_v2(&document);
        assert!(!validation.passed);
        for expected in [
            "recipe_digest must be lowercase SHA-256",
            "workers must be a positive integer",
            "seed must be a nonnegative safe integer or null",
            "completed_at precedes started_at",
            "precision must be an integer from 0 to 12",
        ] {
            assert!(
                validation
                    .errors
                    .iter()
                    .any(|error| error.contains(expected)),
                "missing {expected:?} in {:?}",
                validation.errors
            );
        }
    }
}
