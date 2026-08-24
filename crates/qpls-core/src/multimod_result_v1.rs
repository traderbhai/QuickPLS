use crate::{
    CapabilityCellReferenceV2, HeterogeneityAlgorithmV2, HeterogeneityInferenceLockReceiptV2,
    HeterogeneityInteractionProfileV2, InferenceAlternativeV1, MgaModelProfileV1,
    MultiplicityAdjustmentV1,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PLS_MULTIGROUP_ANALYSIS_V1_SCHEMA_VERSION: u32 = 1;
pub const PLS_HETEROGENEITY_ANALYSIS_V2_SCHEMA_VERSION: u32 = 2;
pub const GENERAL_SEM_CONDITIONAL_PROCESS_RESULT_V2_SCHEMA_VERSION: u32 = 2;
pub const INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION: u32 = 1;
pub const MULTIMOD_RESULT_SIDECAR_DESCRIPTOR_V1_SCHEMA_VERSION: u32 = 1;
pub const MULTIMOD_CANDIDATE_QUALIFICATION_RECEIPT_V1_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultimodQualificationStateV1 {
    UnqualifiedLabs,
    ReleaseQualifiedCandidate,
    FailedClosed,
}

/// Immutable receipt proving that native promotion used the candidate
/// authority embedded at build time. The cell inventory is the exact sorted
/// subset required by this typed recipe/result, never a family-level or
/// wildcard claim.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultimodCandidateQualificationReceiptV1 {
    pub schema_version: u32,
    pub authority_binding_sha256: String,
    pub candidate_commit_sha: String,
    pub candidate_version: String,
    pub qualification_plan_sha256: String,
    pub gate_binding_sha256: String,
    pub capability_index_sha256: String,
    pub prepackage_manifest_set_sha256: String,
    pub required_profile_cells: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultimodProvenanceV1 {
    pub method_version: String,
    pub recipe_id: String,
    pub recipe_analytical_sha256: String,
    pub config_sha256: String,
    pub model_id: String,
    pub model_scientific_sha256: String,
    pub dataset_id: String,
    pub dataset_fingerprint: String,
    pub engine_version: String,
    pub seed: u64,
    pub capability_cell: CapabilityCellReferenceV2,
    pub qualification: MultimodQualificationStateV1,
    /// Absent for historical and current Labs results. It is required exactly
    /// when `qualification` is `release_qualified_candidate`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_qualification_receipt: Option<MultimodCandidateQualificationReceiptV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultimodResultSidecarDescriptorV1 {
    pub schema_version: u32,
    pub entry_name: String,
    /// Stable scientific evidence role (for example a FIMIX posterior matrix
    /// or a conditional-process replicate ledger), never an arbitrary label.
    pub evidence_role: String,
    /// Versioned Arrow field/type/metadata contract for this evidence role.
    pub arrow_schema_contract_id: String,
    pub arrow_schema_contract_version: u32,
    pub media_type: String,
    pub compression: String,
    /// SHA-256 of the canonical Arrow schema identity. This field is required:
    /// pre-identity MultiMod V1 descriptors must fail closed rather than reopen
    /// against a table that merely has the same row and column counts.
    pub arrow_schema_sha256: String,
    pub row_count: u64,
    pub column_count: u32,
    pub uncompressed_bytes: u64,
    pub sha256: String,
    pub identity_sha256: String,
    pub required_for_scientific_reopen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultimodReplicateFailureKindV1 {
    Cancelled,
    EmptyGroup,
    InsufficientCases,
    RankDeficient,
    ConstantScore,
    ConstantProduct,
    NonfiniteEstimate,
    EstimatorDidNotConverge,
    ClassCollapsed,
    VarianceCollapsed,
    UnstableMultistart,
    AmbiguousLabelAlignment,
    ComparabilityFailed,
    TargetInventoryMismatch,
    InnerStandardErrorUnavailable,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultimodReplicateFailureV1 {
    pub replicate_index: u32,
    pub kind: MultimodReplicateFailureKindV1,
    pub stable_code: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultimodReplicateLedgerSummaryV1 {
    pub requested: u32,
    pub usable: u32,
    pub minimum_required: u32,
    pub usable_fraction: f64,
    pub complete: bool,
    pub ledger_sha256: String,
    pub failure_counts: BTreeMap<String, u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<MultimodReplicateFailureV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultimodIntervalV1 {
    pub confidence_level: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
    pub family: String,
    pub alternative: InferenceAlternativeV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultimodParameterEstimateV1 {
    pub target_id: String,
    pub target_kind: String,
    pub estimate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard_error: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<MultimodIntervalV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MgaGroupEligibilityV1 {
    pub group_id: String,
    pub label: String,
    pub complete_cases: u64,
    pub selected_rows: u64,
    pub eligible: bool,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MgaGroupParameterV1 {
    pub group_id: String,
    pub parameter: MultimodParameterEstimateV1,
}

/// Scientific interpretation of the MICOM steps reported for a PLS-based
/// multigroup run. MICOM establishes invariance of composite scores; it must
/// never be relabelled as common-factor measurement invariance, including
/// when the structural coefficients are subsequently corrected by PLSc.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MicomInvarianceInterpretationV1 {
    CompositeInvariance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MicomPairResultV1 {
    pub left_group_id: String,
    pub right_group_id: String,
    pub construct_id: String,
    pub interpretation: MicomInvarianceInterpretationV1,
    pub configural_invariance_confirmed: bool,
    pub compositional_correlation: f64,
    pub compositional_lower_quantile: f64,
    pub compositional_p_value: f64,
    pub compositional_invariance: bool,
    pub partial_invariance: bool,
    pub equal_mean_p_value: f64,
    pub equal_variance_p_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MgaPairwiseComparisonV1 {
    pub procedure: String,
    pub left_group_id: String,
    pub right_group_id: String,
    pub target_id: String,
    pub difference_left_minus_right: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_p_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjusted_p_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directional_probability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<MultimodIntervalV1>,
    pub measurement_comparability_satisfied: bool,
    pub interpretation_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MgaOmnibusComparisonV1 {
    pub procedure: String,
    pub target_id: String,
    pub statistic: f64,
    pub degrees_of_freedom: u32,
    pub p_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExcludedRowReasonV1 {
    UnselectedGroupValue,
    MissingGroupValue,
    MissingModelValue,
    InvalidWeight,
    NonfiniteValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExcludedRowReceiptV1 {
    pub stable_row_token: String,
    pub typed_group_value: String,
    pub reason: ExcludedRowReasonV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsMultigroupAnalysisV1 {
    pub schema_version: u32,
    pub provenance: MultimodProvenanceV1,
    pub profile: MgaModelProfileV1,
    pub group_eligibility: Vec<MgaGroupEligibilityV1>,
    pub group_parameters: Vec<MgaGroupParameterV1>,
    pub micom_pairs: Vec<MicomPairResultV1>,
    pub omnibus: Vec<MgaOmnibusComparisonV1>,
    pub pairwise: Vec<MgaPairwiseComparisonV1>,
    pub multiplicity: MultiplicityAdjustmentV1,
    pub replicate_ledgers: Vec<MultimodReplicateLedgerSummaryV1>,
    pub excluded_rows: Vec<ExcludedRowReceiptV1>,
    #[serde(default)]
    pub sidecars: Vec<MultimodResultSidecarDescriptorV1>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeterogeneityCandidateStateV2 {
    Eligible,
    ConvergedStable,
    Ineligible,
    Failed,
    Unstable,
}

/// Result-only identity that keeps the pooled K=1 reference separate from all
/// actual latent-segmentation algorithms.  Configuration choices remain the
/// three variants of `HeterogeneityAlgorithmV2`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HeterogeneityCandidateMethodV2 {
    PooledBaselineV1,
    Segmentation { algorithm: HeterogeneityAlgorithmV2 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HeterogeneityCandidateV2 {
    pub method: HeterogeneityCandidateMethodV2,
    pub k: u8,
    pub state: HeterogeneityCandidateStateV2,
    pub converged_starts: u32,
    pub stable_starts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_likelihood: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective: Option<f64>,
    #[serde(default)]
    pub criteria: BTreeMap<String, f64>,
    #[serde(default)]
    pub class_or_segment_shares: Vec<f64>,
    /// Populated only by the nonselectable pooled K=1 baseline.  Segmentation
    /// candidates retain their class/segment estimates in the locked result.
    #[serde(default)]
    pub pooled_parameters: Vec<MultimodParameterEstimateV1>,
    #[serde(default)]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HeterogeneityClassParameterV2 {
    pub class_id: u8,
    pub parameter: MultimodParameterEstimateV1,
    pub metric: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HeterogeneityClassContrastV2 {
    pub left_class_id: u8,
    pub right_class_id: u8,
    pub target_id: String,
    pub difference: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<MultimodIntervalV1>,
    pub common_metric_comparability_satisfied: bool,
    pub inferential_interpretation_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsHeterogeneityAnalysisV2 {
    pub schema_version: u32,
    pub provenance: MultimodProvenanceV1,
    pub profile: HeterogeneityInteractionProfileV2,
    pub candidates: Vec<HeterogeneityCandidateV2>,
    /// Canonical identity of the pooled baseline plus every requested
    /// discovery candidate.  Inference reruns that inventory and must match
    /// the analyst-provided lock before publication.
    pub discovery_result_identity_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_lock: Option<HeterogeneityInferenceLockReceiptV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked_algorithm: Option<HeterogeneityAlgorithmV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked_k: Option<u8>,
    pub parameters: Vec<HeterogeneityClassParameterV2>,
    pub contrasts: Vec<HeterogeneityClassContrastV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_ledger: Option<MultimodReplicateLedgerSummaryV1>,
    #[serde(default)]
    pub sidecars: Vec<MultimodResultSidecarDescriptorV1>,
    pub descriptive_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalProcessTargetKindV2 {
    ConditionalSpecificIndirect,
    ConditionalTotalIndirect,
    ConditionalTotalEffect,
    ScalarIndexOfModeratedMediation,
    LocalFirstDerivative,
    LocalSecondDerivative,
    LocalCrossDerivative,
    ProbeContrast,
    GroupContrast,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessTargetResultV2 {
    pub target_id: String,
    pub kind: ConditionalProcessTargetKindV2,
    pub path_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default)]
    pub probe_values: BTreeMap<String, f64>,
    #[serde(default)]
    pub derivative_variables: Vec<String>,
    pub estimate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<MultimodIntervalV1>,
    pub usable_replicates: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemConditionalProcessResultV2 {
    pub schema_version: u32,
    pub provenance: MultimodProvenanceV1,
    pub profile_id: String,
    pub targets: Vec<ConditionalProcessTargetResultV2>,
    pub replicate_ledger: MultimodReplicateLedgerSummaryV1,
    #[serde(default)]
    pub sidecars: Vec<MultimodResultSidecarDescriptorV1>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CausalPositivityDiagnosticV1 {
    pub variable_id: String,
    pub observed_minimum: f64,
    pub observed_maximum: f64,
    pub requested_value: f64,
    pub support_count: u64,
    pub minimum_required_count: u64,
    pub support_rule: String,
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterventionalEffectResultV1 {
    pub target_id: String,
    pub path_id: String,
    pub estimand: String,
    pub estimate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<MultimodIntervalV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterventionalMediationResultV1 {
    pub schema_version: u32,
    pub provenance: MultimodProvenanceV1,
    pub interpretation_label: String,
    pub identification_assumptions: Vec<String>,
    pub positivity: Vec<CausalPositivityDiagnosticV1>,
    pub effects: Vec<InterventionalEffectResultV1>,
    pub replicate_ledger: MultimodReplicateLedgerSummaryV1,
    #[serde(default)]
    pub sidecars: Vec<MultimodResultSidecarDescriptorV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "analysis",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MultiModAnalysisResultV1 {
    PlsMultigroupAnalysisV1(PlsMultigroupAnalysisV1),
    PlsHeterogeneityAnalysisV2(PlsHeterogeneityAnalysisV2),
    GeneralSemConditionalProcessResultV2(GeneralSemConditionalProcessResultV2),
    InterventionalMediationResultV1(InterventionalMediationResultV1),
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("{code} at {path}: {message}")]
pub struct MultiModResultValidationErrorV1 {
    pub code: String,
    pub path: String,
    pub message: String,
}

fn result_invalid(
    code: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
) -> MultiModResultValidationErrorV1 {
    MultiModResultValidationErrorV1 {
        code: code.into(),
        path: path.into(),
        message: message.into(),
    }
}

fn nonempty(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}

fn lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn lowercase_sha1(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn exact_profile_cell(value: &str) -> bool {
    if !nonempty(value)
        || value.contains('*')
        || value.contains('?')
        || value.contains('[')
        || value.contains(']')
    {
        return false;
    }
    let mut pieces = value.split("::");
    matches!((pieces.next(), pieces.next(), pieces.next()), (Some(profile), Some(cell), None) if nonempty(profile) && nonempty(cell))
}

fn validate_candidate_qualification_receipt(
    receipt: &MultimodCandidateQualificationReceiptV1,
    path: &str,
) -> Result<(), MultiModResultValidationErrorV1> {
    if receipt.schema_version != MULTIMOD_CANDIDATE_QUALIFICATION_RECEIPT_V1_SCHEMA_VERSION
        || !lowercase_sha256(&receipt.authority_binding_sha256)
        || !lowercase_sha1(&receipt.candidate_commit_sha)
        || !nonempty(&receipt.candidate_version)
        || !lowercase_sha256(&receipt.qualification_plan_sha256)
        || !lowercase_sha256(&receipt.gate_binding_sha256)
        || !lowercase_sha256(&receipt.capability_index_sha256)
        || !lowercase_sha256(&receipt.prepackage_manifest_set_sha256)
        || receipt.required_profile_cells.is_empty()
        || receipt
            .required_profile_cells
            .iter()
            .any(|cell| !exact_profile_cell(cell))
        || !receipt
            .required_profile_cells
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    {
        return Err(result_invalid(
            "multimod_result.candidate_qualification_receipt",
            path,
            "candidate receipt identities must be exact and its profile cells must be nonempty, sorted, unique, and wildcard-free",
        ));
    }
    Ok(())
}

fn probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn validate_provenance(
    provenance: &MultimodProvenanceV1,
    path: &str,
) -> Result<(), MultiModResultValidationErrorV1> {
    if !nonempty(&provenance.method_version)
        || !nonempty(&provenance.recipe_id)
        || !nonempty(&provenance.model_id)
        || !nonempty(&provenance.dataset_id)
        || !nonempty(&provenance.dataset_fingerprint)
        || !nonempty(&provenance.engine_version)
        || !lowercase_sha256(&provenance.recipe_analytical_sha256)
        || !lowercase_sha256(&provenance.config_sha256)
        || !lowercase_sha256(&provenance.model_scientific_sha256)
        || provenance.capability_cell.registry_schema_version != 2
        || !nonempty(&provenance.capability_cell.capability_id)
        || !nonempty(&provenance.capability_cell.cell_id)
        || !nonempty(&provenance.capability_cell.capability_version)
    {
        return Err(result_invalid(
            "multimod_result.provenance",
            path,
            "provenance identities, digests, or capability reference are invalid",
        ));
    }
    match (
        provenance.qualification,
        provenance.candidate_qualification_receipt.as_ref(),
    ) {
        (MultimodQualificationStateV1::ReleaseQualifiedCandidate, Some(receipt)) => {
            validate_candidate_qualification_receipt(
                receipt,
                &format!("{path}.candidate_qualification_receipt"),
            )?
        }
        (MultimodQualificationStateV1::ReleaseQualifiedCandidate, None) => {
            return Err(result_invalid(
                "multimod_result.candidate_qualification_receipt_missing",
                format!("{path}.candidate_qualification_receipt"),
                "release-qualified-candidate provenance requires its exact build-embedded authority receipt",
            ));
        }
        (_, Some(_)) => {
            return Err(result_invalid(
                "multimod_result.candidate_qualification_receipt_unexpected",
                format!("{path}.candidate_qualification_receipt"),
                "Labs and failed-closed provenance cannot carry a candidate authority receipt",
            ));
        }
        (_, None) => {}
    }
    Ok(())
}

fn validate_interval(
    interval: &MultimodIntervalV1,
    path: &str,
) -> Result<(), MultiModResultValidationErrorV1> {
    let endpoints_valid = match (interval.alternative, interval.lower, interval.upper) {
        (InferenceAlternativeV1::TwoSided, Some(lower), Some(upper)) => {
            lower.is_finite() && upper.is_finite() && lower <= upper
        }
        (InferenceAlternativeV1::Less, None, Some(upper)) => upper.is_finite(),
        (InferenceAlternativeV1::Greater, Some(lower), None) => lower.is_finite(),
        _ => false,
    };
    if !probability(interval.confidence_level)
        || interval.confidence_level == 0.0
        || interval.confidence_level == 1.0
        || !endpoints_valid
        || !nonempty(&interval.family)
    {
        return Err(result_invalid(
            "multimod_result.interval",
            path,
            "interval confidence, alternative-specific endpoints, or family are invalid",
        ));
    }
    Ok(())
}

fn validate_parameter(
    parameter: &MultimodParameterEstimateV1,
    path: &str,
) -> Result<(), MultiModResultValidationErrorV1> {
    if !nonempty(&parameter.target_id)
        || !nonempty(&parameter.target_kind)
        || !parameter.estimate.is_finite()
        || parameter
            .standard_error
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        || parameter.p_value.is_some_and(|value| !probability(value))
    {
        return Err(result_invalid(
            "multimod_result.parameter",
            path,
            "parameter identity or numeric value is invalid",
        ));
    }
    if let Some(interval) = &parameter.interval {
        validate_interval(interval, &format!("{path}.interval"))?;
    }
    Ok(())
}

fn validate_ledger(
    ledger: &MultimodReplicateLedgerSummaryV1,
    path: &str,
) -> Result<(), MultiModResultValidationErrorV1> {
    let expected_fraction = if ledger.requested == 0 {
        0.0
    } else {
        f64::from(ledger.usable) / f64::from(ledger.requested)
    };
    let failed = ledger.requested.saturating_sub(ledger.usable);
    let failure_count = ledger
        .failure_counts
        .values()
        .try_fold(0u32, |total, value| total.checked_add(*value));
    if ledger.requested == 0
        || ledger.usable > ledger.requested
        || ledger.minimum_required == 0
        || ledger.minimum_required > ledger.requested
        || !ledger.usable_fraction.is_finite()
        || (ledger.usable_fraction - expected_fraction).abs() > 1e-12
        || ledger.complete != (ledger.usable >= ledger.minimum_required)
        || !lowercase_sha256(&ledger.ledger_sha256)
        || failure_count != Some(failed)
        || ledger.failure_counts.keys().any(|code| !nonempty(code))
        || ledger.failures.iter().any(|failure| {
            failure.replicate_index >= ledger.requested
                || !nonempty(&failure.stable_code)
                || failure.detail.trim() != failure.detail
        })
    {
        return Err(result_invalid(
            "multimod_result.ledger",
            path,
            "replicate counts, usable fraction, digest, or failure inventory are inconsistent",
        ));
    }
    Ok(())
}

fn validate_sidecars(
    sidecars: &[MultimodResultSidecarDescriptorV1],
    path: &str,
) -> Result<(), MultiModResultValidationErrorV1> {
    let mut entries = BTreeSet::new();
    let mut total_bytes = 0u64;
    for (index, sidecar) in sidecars.iter().enumerate() {
        if sidecar.schema_version != MULTIMOD_RESULT_SIDECAR_DESCRIPTOR_V1_SCHEMA_VERSION
            || !nonempty(&sidecar.entry_name)
            || !entries.insert(sidecar.entry_name.as_str())
            || !nonempty(&sidecar.evidence_role)
            || !nonempty(&sidecar.arrow_schema_contract_id)
            || sidecar.arrow_schema_contract_version != 1
            || !nonempty(&sidecar.media_type)
            || !nonempty(&sidecar.compression)
            || !lowercase_sha256(&sidecar.arrow_schema_sha256)
            || sidecar.row_count == 0
            || sidecar.column_count == 0
            || sidecar.uncompressed_bytes == 0
            || sidecar.uncompressed_bytes > crate::MULTIMOD_SIDECAR_MAX_BYTES_V1
            || !lowercase_sha256(&sidecar.sha256)
            || !lowercase_sha256(&sidecar.identity_sha256)
            || !sidecar.required_for_scientific_reopen
        {
            return Err(result_invalid(
                "multimod_result.sidecar",
                format!("{path}[{index}]"),
                "sidecar identity, shape, digest, or reopen requirement is invalid",
            ));
        }
        total_bytes = total_bytes
            .checked_add(sidecar.uncompressed_bytes)
            .ok_or_else(|| {
                result_invalid(
                    "multimod_result.sidecar_total",
                    path,
                    "aggregate sidecar byte count overflowed the per-run evidence contract",
                )
            })?;
    }
    if total_bytes > crate::MULTIMOD_SIDECAR_MAX_BYTES_V1 {
        return Err(result_invalid(
            "multimod_result.sidecar_total",
            path,
            "aggregate sidecar evidence exceeds the 512 MiB per-run cap",
        ));
    }
    Ok(())
}

fn unique_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> Result<(), MultiModResultValidationErrorV1> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !nonempty(value) || !seen.insert(value) {
            return Err(result_invalid(
                "multimod_result.target_identity",
                "result.targets",
                "scientific target identities must be nonempty and unique",
            ));
        }
    }
    Ok(())
}

impl MultiModAnalysisResultV1 {
    pub fn provenance(&self) -> &MultimodProvenanceV1 {
        match self {
            Self::PlsMultigroupAnalysisV1(value) => &value.provenance,
            Self::PlsHeterogeneityAnalysisV2(value) => &value.provenance,
            Self::GeneralSemConditionalProcessResultV2(value) => &value.provenance,
            Self::InterventionalMediationResultV1(value) => &value.provenance,
        }
    }

    pub fn provenance_mut(&mut self) -> &mut MultimodProvenanceV1 {
        match self {
            Self::PlsMultigroupAnalysisV1(value) => &mut value.provenance,
            Self::PlsHeterogeneityAnalysisV2(value) => &mut value.provenance,
            Self::GeneralSemConditionalProcessResultV2(value) => &mut value.provenance,
            Self::InterventionalMediationResultV1(value) => &mut value.provenance,
        }
    }

    pub fn sidecars(&self) -> &[MultimodResultSidecarDescriptorV1] {
        match self {
            Self::PlsMultigroupAnalysisV1(value) => &value.sidecars,
            Self::PlsHeterogeneityAnalysisV2(value) => &value.sidecars,
            Self::GeneralSemConditionalProcessResultV2(value) => &value.sidecars,
            Self::InterventionalMediationResultV1(value) => &value.sidecars,
        }
    }

    pub fn ensure_valid(&self) -> Result<(), MultiModResultValidationErrorV1> {
        match self {
            Self::PlsMultigroupAnalysisV1(result) => {
                if result.schema_version != PLS_MULTIGROUP_ANALYSIS_V1_SCHEMA_VERSION
                    || result.group_eligibility.len() < 2
                    || result.group_eligibility.len() > 20
                {
                    return Err(result_invalid(
                        "multimod_result.mga_schema_or_groups",
                        "result.pls_multigroup",
                        "MGA result schema or group count is invalid",
                    ));
                }
                validate_provenance(&result.provenance, "result.provenance")?;
                unique_ids(
                    result
                        .group_eligibility
                        .iter()
                        .map(|group| group.group_id.as_str()),
                )?;
                if result.group_eligibility.iter().any(|group| {
                    !nonempty(&group.label)
                        || group.complete_cases > group.selected_rows
                        || group.eligible != group.blockers.is_empty()
                }) {
                    return Err(result_invalid(
                        "multimod_result.mga_eligibility",
                        "result.group_eligibility",
                        "MGA eligibility counts or blocker state are inconsistent",
                    ));
                }
                let mut parameter_keys = BTreeSet::new();
                for (index, row) in result.group_parameters.iter().enumerate() {
                    validate_parameter(
                        &row.parameter,
                        &format!("result.group_parameters[{index}]"),
                    )?;
                    if !parameter_keys
                        .insert((row.group_id.as_str(), row.parameter.target_id.as_str()))
                    {
                        return Err(result_invalid(
                            "multimod_result.mga_parameter_duplicate",
                            "result.group_parameters",
                            "group-parameter identities must be unique",
                        ));
                    }
                }
                for (index, pair) in result.micom_pairs.iter().enumerate() {
                    if pair.left_group_id == pair.right_group_id
                        || !nonempty(&pair.construct_id)
                        || !pair.compositional_correlation.is_finite()
                        || !(-1.0..=1.0).contains(&pair.compositional_correlation)
                        || !pair.compositional_lower_quantile.is_finite()
                        || !(-1.0..=1.0).contains(&pair.compositional_lower_quantile)
                        || !probability(pair.compositional_p_value)
                        || !probability(pair.equal_mean_p_value)
                        || !probability(pair.equal_variance_p_value)
                        || pair.compositional_invariance
                            != (pair.compositional_correlation >= pair.compositional_lower_quantile)
                        || pair.partial_invariance
                            != (pair.configural_invariance_confirmed
                                && pair.compositional_invariance)
                    {
                        return Err(result_invalid(
                            "multimod_result.micom_pair",
                            format!("result.micom_pairs[{index}]"),
                            "MICOM pair identity, probability, or partial-invariance state is invalid",
                        ));
                    }
                }
                for (index, pair) in result.pairwise.iter().enumerate() {
                    if pair.left_group_id == pair.right_group_id
                        || !nonempty(&pair.procedure)
                        || !nonempty(&pair.target_id)
                        || !pair.difference_left_minus_right.is_finite()
                        || pair.raw_p_value.is_some_and(|value| !probability(value))
                        || pair
                            .adjusted_p_value
                            .is_some_and(|value| !probability(value))
                        || pair
                            .directional_probability
                            .is_some_and(|value| !probability(value))
                        || pair.interpretation_blocked == pair.measurement_comparability_satisfied
                    {
                        return Err(result_invalid(
                            "multimod_result.mga_pairwise",
                            format!("result.pairwise[{index}]"),
                            "pairwise comparison identity, inference, or comparability state is invalid",
                        ));
                    }
                    if let Some(interval) = &pair.interval {
                        validate_interval(interval, &format!("result.pairwise[{index}].interval"))?;
                    }
                }
                for (index, row) in result.omnibus.iter().enumerate() {
                    if !nonempty(&row.procedure)
                        || !nonempty(&row.target_id)
                        || !row.statistic.is_finite()
                        || row.degrees_of_freedom == 0
                        || !probability(row.p_value)
                    {
                        return Err(result_invalid(
                            "multimod_result.mga_omnibus",
                            format!("result.omnibus[{index}]"),
                            "omnibus comparison is invalid",
                        ));
                    }
                }
                for (index, ledger) in result.replicate_ledgers.iter().enumerate() {
                    validate_ledger(ledger, &format!("result.replicate_ledgers[{index}]"))?;
                }
                if result
                    .replicate_ledgers
                    .iter()
                    .any(|ledger| !ledger.complete)
                    && (!result.micom_pairs.is_empty()
                        || !result.omnibus.is_empty()
                        || result.pairwise.iter().any(|row| {
                            row.raw_p_value.is_some()
                                || row.adjusted_p_value.is_some()
                                || row.directional_probability.is_some()
                                || row.interval.is_some()
                        }))
                {
                    return Err(result_invalid(
                        "multimod_result.mga_inference_requires_complete_ledger",
                        "result.replicate_ledgers",
                        "incomplete MGA resampling ledgers cannot publish MICOM, permutation/bootstrap, or omnibus inference",
                    ));
                }
                unique_ids(
                    result
                        .excluded_rows
                        .iter()
                        .map(|row| row.stable_row_token.as_str()),
                )?;
                validate_sidecars(&result.sidecars, "result.sidecars")?;
            }
            Self::PlsHeterogeneityAnalysisV2(result) => {
                if result.schema_version != PLS_HETEROGENEITY_ANALYSIS_V2_SCHEMA_VERSION {
                    return Err(result_invalid(
                        "multimod_result.heterogeneity_schema",
                        "result.pls_heterogeneity",
                        "heterogeneity result schema must equal 2",
                    ));
                }
                validate_provenance(&result.provenance, "result.provenance")?;
                if !lowercase_sha256(&result.discovery_result_identity_sha256) {
                    return Err(result_invalid(
                        "multimod_result.heterogeneity_discovery_identity",
                        "result.discovery_result_identity_sha256",
                        "heterogeneity discovery identity must be one lowercase SHA-256 digest",
                    ));
                }
                let mut candidate_keys = BTreeSet::new();
                let mut pooled_baseline_count = 0_usize;
                for (index, candidate) in result.candidates.iter().enumerate() {
                    let method_shape_valid = match candidate.method {
                        HeterogeneityCandidateMethodV2::PooledBaselineV1 => {
                            pooled_baseline_count += 1;
                            candidate.k == 1
                                && candidate.state == HeterogeneityCandidateStateV2::Eligible
                                && candidate.converged_starts == 0
                                && candidate.stable_starts == 0
                                && candidate.log_likelihood.is_none()
                                && candidate.objective.is_none()
                                && candidate.class_or_segment_shares.is_empty()
                                && candidate.blockers.is_empty()
                                && !candidate.pooled_parameters.is_empty()
                        }
                        HeterogeneityCandidateMethodV2::Segmentation { .. } => {
                            (2..=5).contains(&candidate.k)
                                && candidate.pooled_parameters.is_empty()
                                && (candidate.class_or_segment_shares.is_empty()
                                    || candidate.class_or_segment_shares.len()
                                        == candidate.k as usize)
                        }
                    };
                    if !method_shape_valid
                        || !candidate_keys.insert((candidate.method, candidate.k))
                        || candidate
                            .log_likelihood
                            .is_some_and(|value| !value.is_finite())
                        || candidate.objective.is_some_and(|value| !value.is_finite())
                        || candidate.criteria.values().any(|value| !value.is_finite())
                        || candidate
                            .class_or_segment_shares
                            .iter()
                            .any(|value| !probability(*value))
                        || (!candidate.class_or_segment_shares.is_empty()
                            && (candidate.class_or_segment_shares.iter().sum::<f64>() - 1.0).abs()
                                > 1e-8)
                    {
                        return Err(result_invalid(
                            "multimod_result.heterogeneity_candidate",
                            format!("result.candidates[{index}]"),
                            "candidate identity, criterion, or segment shares are invalid",
                        ));
                    }
                    let mut pooled_parameter_ids = BTreeSet::new();
                    for (parameter_index, parameter) in
                        candidate.pooled_parameters.iter().enumerate()
                    {
                        validate_parameter(
                            parameter,
                            &format!(
                                "result.candidates[{index}].pooled_parameters[{parameter_index}]"
                            ),
                        )?;
                        if !pooled_parameter_ids.insert(parameter.target_id.as_str()) {
                            return Err(result_invalid(
                                "multimod_result.heterogeneity_pooled_parameter",
                                format!(
                                    "result.candidates[{index}].pooled_parameters[{parameter_index}]"
                                ),
                                "pooled baseline parameter identity is duplicated",
                            ));
                        }
                    }
                }
                if pooled_baseline_count != 1
                    || result.locked_algorithm.is_some() != result.locked_k.is_some()
                    || result.inference_lock.is_some() != result.locked_algorithm.is_some()
                    || result
                        .locked_k
                        .is_some_and(|value| !(2..=5).contains(&value))
                    || result.descriptive_only
                        && result.contrasts.iter().any(|contrast| {
                            !contrast.inferential_interpretation_blocked
                                || contrast.p_value.is_some()
                                || contrast.interval.is_some()
                        })
                {
                    return Err(result_invalid(
                        "multimod_result.heterogeneity_lock_or_gate",
                        "result.pls_heterogeneity",
                        "locked algorithm/K or descriptive-only comparability gate is inconsistent",
                    ));
                }
                if let Some(lock) = &result.inference_lock {
                    lock.ensure_valid().map_err(|error| {
                        result_invalid(
                            "multimod_result.heterogeneity_inference_lock",
                            "result.inference_lock",
                            error.to_string(),
                        )
                    })?;
                    if result.locked_algorithm != Some(lock.selected_algorithm)
                        || result.locked_k != Some(lock.selected_k)
                        || result.discovery_result_identity_sha256
                            != lock.discovery_result_identity_sha256
                    {
                        return Err(result_invalid(
                            "multimod_result.heterogeneity_inference_lock_identity",
                            "result.inference_lock",
                            "published lock, selected algorithm/K, and reproduced discovery identity differ",
                        ));
                    }
                    let actual_inventory = result
                        .candidates
                        .iter()
                        .filter_map(|candidate| match candidate.method {
                            HeterogeneityCandidateMethodV2::Segmentation { algorithm } => {
                                Some((algorithm, candidate.k))
                            }
                            HeterogeneityCandidateMethodV2::PooledBaselineV1 => None,
                        })
                        .collect::<BTreeSet<_>>();
                    let expected_inventory = lock
                        .discovery_algorithms
                        .iter()
                        .flat_map(|algorithm| {
                            lock.discovery_candidate_k
                                .iter()
                                .map(move |k| (*algorithm, *k))
                        })
                        .collect::<BTreeSet<_>>();
                    if actual_inventory != expected_inventory {
                        return Err(result_invalid(
                            "multimod_result.heterogeneity_inference_inventory",
                            "result.candidates",
                            "reproduced candidate inventory differs from the locked discovery inventory",
                        ));
                    }
                }
                let mut parameter_keys = BTreeSet::new();
                for (index, row) in result.parameters.iter().enumerate() {
                    validate_parameter(&row.parameter, &format!("result.parameters[{index}]"))?;
                    if row.class_id == 0
                        || !nonempty(&row.metric)
                        || !parameter_keys.insert((row.class_id, row.parameter.target_id.as_str()))
                    {
                        return Err(result_invalid(
                            "multimod_result.heterogeneity_parameter",
                            format!("result.parameters[{index}]"),
                            "class parameter identity is invalid or duplicated",
                        ));
                    }
                }
                for (index, contrast) in result.contrasts.iter().enumerate() {
                    if contrast.left_class_id == 0
                        || contrast.right_class_id == 0
                        || contrast.left_class_id == contrast.right_class_id
                        || !nonempty(&contrast.target_id)
                        || !contrast.difference.is_finite()
                        || contrast.p_value.is_some_and(|value| !probability(value))
                        || contrast.inferential_interpretation_blocked
                            == contrast.common_metric_comparability_satisfied
                    {
                        return Err(result_invalid(
                            "multimod_result.heterogeneity_contrast",
                            format!("result.contrasts[{index}]"),
                            "class contrast or common-metric gate is invalid",
                        ));
                    }
                    if let Some(interval) = &contrast.interval {
                        validate_interval(
                            interval,
                            &format!("result.contrasts[{index}].interval"),
                        )?;
                    }
                }
                if let Some(ledger) = &result.bootstrap_ledger {
                    validate_ledger(ledger, "result.bootstrap_ledger")?;
                    if !ledger.complete
                        && result.contrasts.iter().any(|contrast| {
                            contrast.p_value.is_some()
                                || contrast.interval.is_some()
                                || !contrast.inferential_interpretation_blocked
                        })
                    {
                        return Err(result_invalid(
                            "multimod_result.heterogeneity_inference_requires_complete_ledger",
                            "result.bootstrap_ledger",
                            "an incomplete fixed-K bootstrap ledger permits descriptive segment estimates only",
                        ));
                    }
                }
                validate_sidecars(&result.sidecars, "result.sidecars")?;
            }
            Self::GeneralSemConditionalProcessResultV2(result) => {
                if result.schema_version != GENERAL_SEM_CONDITIONAL_PROCESS_RESULT_V2_SCHEMA_VERSION
                    || !nonempty(&result.profile_id)
                    || result.targets.is_empty()
                {
                    return Err(result_invalid(
                        "multimod_result.conditional_schema_or_targets",
                        "result.conditional_process",
                        "conditional-process schema, profile, or target inventory is invalid",
                    ));
                }
                validate_provenance(&result.provenance, "result.provenance")?;
                unique_ids(
                    result
                        .targets
                        .iter()
                        .map(|target| target.target_id.as_str()),
                )?;
                for (index, target) in result.targets.iter().enumerate() {
                    if !nonempty(&target.path_id)
                        || !target.estimate.is_finite()
                        || target.p_value.is_some_and(|value| !probability(value))
                        || target.probe_values.values().any(|value| !value.is_finite())
                        || target.usable_replicates > result.replicate_ledger.usable
                    {
                        return Err(result_invalid(
                            "multimod_result.conditional_target",
                            format!("result.targets[{index}]"),
                            "conditional target identity, estimate, or replicate count is invalid",
                        ));
                    }
                    if let Some(interval) = &target.interval {
                        validate_interval(interval, &format!("result.targets[{index}].interval"))?;
                    }
                }
                validate_ledger(&result.replicate_ledger, "result.replicate_ledger")?;
                if !result.replicate_ledger.complete
                    && result
                        .targets
                        .iter()
                        .any(|target| target.p_value.is_some() || target.interval.is_some())
                {
                    return Err(result_invalid(
                        "multimod_result.conditional_inference_requires_complete_ledger",
                        "result.replicate_ledger",
                        "an incomplete shared conditional-process ledger cannot publish probabilities or intervals",
                    ));
                }
                validate_sidecars(&result.sidecars, "result.sidecars")?;
            }
            Self::InterventionalMediationResultV1(result) => {
                if result.schema_version != INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION
                    || result.interpretation_label != "assumption-dependent interventional estimate"
                    || result.identification_assumptions.is_empty()
                    || result.effects.is_empty()
                {
                    return Err(result_invalid(
                        "multimod_result.causal_schema_or_label",
                        "result.interventional_mediation",
                        "causal result schema, assumption inventory, effects, or interpretation label is invalid",
                    ));
                }
                validate_provenance(&result.provenance, "result.provenance")?;
                unique_ids(
                    result
                        .effects
                        .iter()
                        .map(|effect| effect.target_id.as_str()),
                )?;
                for (index, diagnostic) in result.positivity.iter().enumerate() {
                    if !nonempty(&diagnostic.variable_id)
                        || !diagnostic.observed_minimum.is_finite()
                        || !diagnostic.observed_maximum.is_finite()
                        || diagnostic.observed_minimum > diagnostic.observed_maximum
                        || !diagnostic.requested_value.is_finite()
                        || diagnostic.minimum_required_count == 0
                        || !nonempty(&diagnostic.support_rule)
                        || diagnostic.supported
                            != (diagnostic.requested_value >= diagnostic.observed_minimum
                                && diagnostic.requested_value <= diagnostic.observed_maximum
                                && diagnostic.support_count >= diagnostic.minimum_required_count)
                    {
                        return Err(result_invalid(
                            "multimod_result.causal_positivity",
                            format!("result.positivity[{index}]"),
                            "positivity diagnostic is inconsistent",
                        ));
                    }
                }
                for (index, effect) in result.effects.iter().enumerate() {
                    if !nonempty(&effect.path_id)
                        || !nonempty(&effect.estimand)
                        || !effect.estimate.is_finite()
                        || effect.p_value.is_some_and(|value| !probability(value))
                    {
                        return Err(result_invalid(
                            "multimod_result.causal_effect",
                            format!("result.effects[{index}]"),
                            "interventional effect identity or estimate is invalid",
                        ));
                    }
                    if let Some(interval) = &effect.interval {
                        validate_interval(interval, &format!("result.effects[{index}].interval"))?;
                    }
                }
                validate_ledger(&result.replicate_ledger, "result.replicate_ledger")?;
                if !result.replicate_ledger.complete
                    && result
                        .effects
                        .iter()
                        .any(|effect| effect.p_value.is_some() || effect.interval.is_some())
                {
                    return Err(result_invalid(
                        "multimod_result.causal_inference_requires_complete_ledger",
                        "result.replicate_ledger",
                        "an incomplete interventional bootstrap ledger cannot publish probabilities or intervals",
                    ));
                }
                validate_sidecars(&result.sidecars, "result.sidecars")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provenance() -> MultimodProvenanceV1 {
        MultimodProvenanceV1 {
            method_version: "interventional_causal_mediation_v1".into(),
            recipe_id: "00000000-0000-0000-0000-000000000101".into(),
            recipe_analytical_sha256: "a".repeat(64),
            config_sha256: "b".repeat(64),
            model_id: "00000000-0000-0000-0000-000000000203".into(),
            model_scientific_sha256: "c".repeat(64),
            dataset_id: "00000000-0000-0000-0000-000000000204".into(),
            dataset_fingerprint: "dataset-fingerprint".into(),
            engine_version: "2.56.0".into(),
            seed: 42,
            capability_cell: CapabilityCellReferenceV2 {
                registry_schema_version: 2,
                capability_id: "quickpls.multimod".into(),
                cell_id: "qpls.multimod.interventional_causal_mediation_v1".into(),
                capability_version: "interventional_causal_mediation_v1".into(),
            },
            qualification: MultimodQualificationStateV1::UnqualifiedLabs,
            candidate_qualification_receipt: None,
        }
    }

    fn ledger() -> MultimodReplicateLedgerSummaryV1 {
        MultimodReplicateLedgerSummaryV1 {
            requested: 500,
            usable: 500,
            minimum_required: 450,
            usable_fraction: 1.0,
            complete: true,
            ledger_sha256: "d".repeat(64),
            failure_counts: BTreeMap::new(),
            failures: Vec::new(),
        }
    }

    fn valid_causal_result() -> MultiModAnalysisResultV1 {
        MultiModAnalysisResultV1::InterventionalMediationResultV1(InterventionalMediationResultV1 {
            schema_version: INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION,
            provenance: provenance(),
            interpretation_label: "assumption-dependent interventional estimate".into(),
            identification_assumptions: vec!["No unmeasured confounding".into()],
            positivity: vec![CausalPositivityDiagnosticV1 {
                variable_id: "treatment".into(),
                observed_minimum: 0.0,
                observed_maximum: 1.0,
                requested_value: 1.0,
                support_count: 50,
                minimum_required_count: 10,
                support_rule: "binary_arm_count".into(),
                supported: true,
            }],
            effects: vec![InterventionalEffectResultV1 {
                target_id: "effect:path-1".into(),
                path_id: "path-1".into(),
                estimand: "interventional_indirect".into(),
                estimate: 0.25,
                p_value: Some(0.04),
                interval: Some(MultimodIntervalV1 {
                    confidence_level: 0.95,
                    lower: Some(0.01),
                    upper: Some(0.49),
                    family: "percentile_type7".into(),
                    alternative: InferenceAlternativeV1::TwoSided,
                }),
            }],
            replicate_ledger: ledger(),
            sidecars: Vec::new(),
        })
    }

    fn candidate_receipt() -> MultimodCandidateQualificationReceiptV1 {
        MultimodCandidateQualificationReceiptV1 {
            schema_version: MULTIMOD_CANDIDATE_QUALIFICATION_RECEIPT_V1_SCHEMA_VERSION,
            authority_binding_sha256: "1".repeat(64),
            candidate_commit_sha: "2".repeat(40),
            candidate_version: "2.56.0".into(),
            qualification_plan_sha256: "3".repeat(64),
            gate_binding_sha256: "4".repeat(64),
            capability_index_sha256: "5".repeat(64),
            prepackage_manifest_set_sha256: "6".repeat(64),
            required_profile_cells: vec![
                "interventional.observed_gcomp.v1::parametric_g_computation".into(),
            ],
        }
    }

    #[test]
    fn valid_result_contract_passes() {
        valid_causal_result().ensure_valid().unwrap();
    }

    #[test]
    fn candidate_state_requires_its_exact_receipt() {
        let mut result = valid_causal_result();
        result.provenance_mut().qualification =
            MultimodQualificationStateV1::ReleaseQualifiedCandidate;
        assert_eq!(
            result.ensure_valid().unwrap_err().code,
            "multimod_result.candidate_qualification_receipt_missing"
        );

        result.provenance_mut().candidate_qualification_receipt = Some(candidate_receipt());
        result.ensure_valid().unwrap();
    }

    #[test]
    fn labs_state_remains_backward_readable_but_rejects_a_candidate_receipt() {
        let result = valid_causal_result();
        let legacy_labs = serde_json::to_value(&result).unwrap();
        assert!(
            legacy_labs
                .to_string()
                .find("candidate_qualification_receipt")
                .is_none()
        );
        let reparsed: MultiModAnalysisResultV1 = serde_json::from_value(legacy_labs).unwrap();
        reparsed.ensure_valid().unwrap();

        let mut invalid = result;
        invalid.provenance_mut().candidate_qualification_receipt = Some(candidate_receipt());
        assert_eq!(
            invalid.ensure_valid().unwrap_err().code,
            "multimod_result.candidate_qualification_receipt_unexpected"
        );
    }

    #[test]
    fn one_sided_interval_requires_exactly_the_selected_finite_bound() {
        let mut result = valid_causal_result();
        let MultiModAnalysisResultV1::InterventionalMediationResultV1(value) = &mut result else {
            unreachable!();
        };
        let interval = value.effects[0].interval.as_mut().unwrap();
        interval.alternative = InferenceAlternativeV1::Greater;
        interval.upper = None;
        result.ensure_valid().unwrap();

        let MultiModAnalysisResultV1::InterventionalMediationResultV1(value) = &mut result else {
            unreachable!();
        };
        value.effects[0].interval.as_mut().unwrap().upper = Some(0.49);
        assert!(matches!(
            result.ensure_valid(),
            Err(MultiModResultValidationErrorV1 { code, .. })
                if code == "multimod_result.interval"
        ));
    }

    #[test]
    fn two_sided_interval_cannot_omit_an_endpoint() {
        let mut result = valid_causal_result();
        let MultiModAnalysisResultV1::InterventionalMediationResultV1(value) = &mut result else {
            unreachable!();
        };
        value.effects[0].interval.as_mut().unwrap().lower = None;
        assert!(matches!(
            result.ensure_valid(),
            Err(MultiModResultValidationErrorV1 { code, .. })
                if code == "multimod_result.interval"
        ));
    }

    #[test]
    fn causal_wording_cannot_be_upgraded_to_an_unqualified_claim() {
        let mut result = valid_causal_result();
        let MultiModAnalysisResultV1::InterventionalMediationResultV1(value) = &mut result else {
            unreachable!();
        };
        value.interpretation_label = "causality established".into();
        assert!(matches!(
            result.ensure_valid(),
            Err(MultiModResultValidationErrorV1 { code, .. })
                if code == "multimod_result.causal_schema_or_label"
        ));
    }

    #[test]
    fn incomplete_ledger_cannot_claim_complete() {
        let mut result = valid_causal_result();
        let MultiModAnalysisResultV1::InterventionalMediationResultV1(value) = &mut result else {
            unreachable!();
        };
        value.replicate_ledger.usable = 449;
        value.replicate_ledger.usable_fraction = 449.0 / 500.0;
        value.replicate_ledger.complete = true;
        value
            .replicate_ledger
            .failure_counts
            .insert("rank_deficient".into(), 51);
        assert!(matches!(
            result.ensure_valid(),
            Err(MultiModResultValidationErrorV1 { code, .. })
                if code == "multimod_result.ledger"
        ));
    }

    fn mark_causal_ledger_incomplete(result: &mut MultiModAnalysisResultV1) {
        let MultiModAnalysisResultV1::InterventionalMediationResultV1(value) = result else {
            unreachable!();
        };
        value.replicate_ledger.usable = 449;
        value.replicate_ledger.usable_fraction = 449.0 / 500.0;
        value.replicate_ledger.complete = false;
        value
            .replicate_ledger
            .failure_counts
            .insert("rank_deficient".into(), 51);
    }

    #[test]
    fn incomplete_causal_ledger_rejects_retained_inferential_fields() {
        let mut result = valid_causal_result();
        mark_causal_ledger_incomplete(&mut result);
        assert!(matches!(
            result.ensure_valid(),
            Err(MultiModResultValidationErrorV1 { code, .. })
                if code == "multimod_result.causal_inference_requires_complete_ledger"
        ));
    }

    #[test]
    fn incomplete_causal_ledger_allows_only_the_assumption_labelled_point_estimate() {
        let mut result = valid_causal_result();
        mark_causal_ledger_incomplete(&mut result);
        let MultiModAnalysisResultV1::InterventionalMediationResultV1(value) = &mut result else {
            unreachable!();
        };
        value.effects[0].p_value = None;
        value.effects[0].interval = None;
        result.ensure_valid().unwrap();
    }

    #[test]
    fn duplicate_target_identity_fails_closed() {
        let mut result = valid_causal_result();
        let MultiModAnalysisResultV1::InterventionalMediationResultV1(value) = &mut result else {
            unreachable!();
        };
        value.effects.push(value.effects[0].clone());
        assert!(matches!(
            result.ensure_valid(),
            Err(MultiModResultValidationErrorV1 { code, .. })
                if code == "multimod_result.target_identity"
        ));
    }
}
