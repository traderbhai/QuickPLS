use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MGA_MULTIGROUP_V1_SCHEMA_VERSION: u32 = 1;
pub const PLS_HETEROGENEITY_V2_SCHEMA_VERSION: u32 = 2;
pub const GENERAL_SEM_CONDITIONAL_PROCESS_V2_SCHEMA_VERSION: u32 = 2;
pub const INTERVENTIONAL_CAUSAL_MEDIATION_V1_SCHEMA_VERSION: u32 = 1;

pub const MULTIMOD_MAX_GROUPS_V1: usize = 20;
pub const MULTIMOD_MIN_GROUP_SIZE_V1: usize = 10;
pub const MULTIMOD_WARN_GROUP_SIZE_V1: usize = 30;
pub const MULTIMOD_MAX_GROUP_IMBALANCE_V1: f64 = 10.0;
pub const MULTIMOD_WARN_GROUP_IMBALANCE_V1: f64 = 2.0;
pub const MGA_MIN_RESAMPLES_V1: u32 = 5_000;
pub const MGA_MAX_RESAMPLES_V1: u32 = 10_000;
pub const HETEROGENEITY_MIN_BOOTSTRAP_V2: u32 = 500;
pub const HETEROGENEITY_MAX_BOOTSTRAP_V2: u32 = 10_000;
pub const HETEROGENEITY_INFERENCE_LOCK_V2_SCHEMA_VERSION: u32 = 1;
pub const CONDITIONAL_PROCESS_MIN_BOOTSTRAP_V2: u32 = 500;
pub const CONDITIONAL_PROCESS_MAX_BOOTSTRAP_V2: u32 = 10_000;
pub const CONDITIONAL_PROCESS_MAX_TARGETS_V2: usize = 1_024;
pub const CONDITIONAL_PROCESS_MAX_CELLS_V2: usize = 512;
pub const CONDITIONAL_RAW_PROBE_TRANSFORMATION_CONTRACT_V2: &str =
    "qpls.conditional.raw_probe.sample_standardization.v1";
pub const CONDITIONAL_RAW_PROBE_FIT_METRIC_CONTRACT_V2: &str =
    "qpls.conditional.raw_probe.fit_metric.v2";
pub const MULTIMOD_SIDECAR_WARN_BYTES_V1: u64 = 128 * 1024 * 1024;
pub const MULTIMOD_SIDECAR_MAX_BYTES_V1: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, thiserror::Error)]
#[serde(deny_unknown_fields)]
#[error("{code} at {path}: {message}")]
pub struct MultiModConfigErrorV1 {
    pub code: String,
    pub path: String,
    pub message: String,
}

fn invalid(
    code: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
) -> MultiModConfigErrorV1 {
    MultiModConfigErrorV1 {
        code: code.into(),
        path: path.into(),
        message: message.into(),
    }
}

fn stable_id(value: &str, path: &str) -> Result<(), MultiModConfigErrorV1> {
    if value.is_empty() || value.trim() != value {
        return Err(invalid(
            "multimod.stable_id",
            path,
            "stable identifiers must be nonempty and have no surrounding whitespace",
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(invalid(
            "multimod.stable_id_control",
            path,
            "stable identifiers cannot contain control characters",
        ));
    }
    Ok(())
}

fn finite_probability(value: f64, path: &str) -> Result<(), MultiModConfigErrorV1> {
    if value.is_finite() && (0.0..1.0).contains(&value) {
        Ok(())
    } else {
        Err(invalid(
            "multimod.probability",
            path,
            "the value must be finite and strictly between zero and one",
        ))
    }
}

fn lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn unique_stable_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
    path: &str,
) -> Result<(), MultiModConfigErrorV1> {
    let mut seen = BTreeSet::new();
    for (index, value) in values.into_iter().enumerate() {
        stable_id(value, &format!("{path}[{index}]"))?;
        if !seen.insert(value) {
            return Err(invalid(
                "multimod.duplicate_id",
                format!("{path}[{index}]"),
                format!("duplicate stable identifier {value}"),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TypedGroupValueV1 {
    Text { value: String },
    Integer { value: i64 },
    Number { value: f64 },
    Boolean { value: bool },
}

impl TypedGroupValueV1 {
    pub fn ensure_valid(&self, path: &str) -> Result<(), MultiModConfigErrorV1> {
        match self {
            Self::Text { value } if value.is_empty() => Err(invalid(
                "multimod.group_value_empty",
                path,
                "text group values cannot be empty",
            )),
            Self::Text { .. } | Self::Integer { .. } | Self::Boolean { .. } => Ok(()),
            Self::Number { value } if value.is_finite() => Ok(()),
            Self::Number { .. } => Err(invalid(
                "multimod.group_value_nonfinite",
                path,
                "numeric group values must be finite",
            )),
        }
    }

    pub fn canonical_key(&self) -> String {
        match self {
            Self::Text { value } => format!("text:{value}"),
            Self::Boolean { value } => format!("boolean:{value}"),
            Self::Integer { value } => format!("integer:{value}"),
            Self::Number { value } => {
                let normalized = if *value == 0.0 { 0.0 } else { *value };
                format!("number:{:016x}", normalized.to_bits())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SelectedGroupV1 {
    pub group_id: String,
    pub label: String,
    pub value: TypedGroupValueV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum InferenceAlternativeV1 {
    TwoSided,
    Less,
    Greater,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MultiplicityAdjustmentV1 {
    Holm,
    Bonferroni,
    Sidak,
    BenjaminiHochbergExploratory,
    NoneExplicit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MicomConfiguralChecklistV1 {
    pub identical_indicators_and_coding: bool,
    pub identical_data_treatment: bool,
    pub identical_algorithm_settings: bool,
    pub identical_model_specification: bool,
    pub deterministic_sign_orientation_reviewed: bool,
    pub analyst_review_confirmed: bool,
}

impl MicomConfiguralChecklistV1 {
    pub fn complete(&self) -> bool {
        self.identical_indicators_and_coding
            && self.identical_data_treatment
            && self.identical_algorithm_settings
            && self.identical_model_specification
            && self.deterministic_sign_orientation_reviewed
            && self.analyst_review_confirmed
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MgaProcedureV1 {
    MicomPairwise,
    PairwisePermutation,
    OmnibusMaxSpreadPermutation,
    HenselerPlsMga,
    BootstrapDifferenceBc,
    ParametricPooledVariance,
    ParametricWelchSatterthwaite,
    ParametricWaldOmnibus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MgaModelProfileV1 {
    GeneralSemPls,
    MultipleTwoWayModeration,
    BoundedThreeWayModeration,
    BoundedTwoWayModeratedMediation,
    MultipleNonnestedHoc,
    CaseWeightedPls,
    FrequencyWeightedPls,
    ReflectivePlsc,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroupPairV1 {
    pub left_group_id: String,
    pub right_group_id: String,
}

impl GroupPairV1 {
    pub fn canonical_key(&self) -> (String, String) {
        if self.left_group_id <= self.right_group_id {
            (self.left_group_id.clone(), self.right_group_id.clone())
        } else {
            (self.right_group_id.clone(), self.left_group_id.clone())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MgaComparisonPlanV1 {
    ReferenceVsRest { reference_group_id: String },
    SelectedPairs { pairs: Vec<GroupPairV1> },
    AllPairs { heavy_run_confirmed: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnalysisWeightBindingV1 {
    Case { column: String },
    Frequency { column: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MgaMultigroupV1 {
    pub schema_version: u32,
    pub profile: MgaModelProfileV1,
    pub grouping_column: String,
    pub groups: Vec<SelectedGroupV1>,
    pub comparison_plan: MgaComparisonPlanV1,
    pub procedures: Vec<MgaProcedureV1>,
    pub permutation_samples: u32,
    pub bootstrap_samples: u32,
    pub seed: u64,
    pub confidence_level: f64,
    pub alpha: f64,
    pub alternative: InferenceAlternativeV1,
    pub multiplicity: MultiplicityAdjustmentV1,
    pub configural_checklist: MicomConfiguralChecklistV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<AnalysisWeightBindingV1>,
    #[serde(default)]
    pub selected_parameter_ids: Vec<String>,
}

impl MgaMultigroupV1 {
    pub fn ensure_valid(&self) -> Result<(), MultiModConfigErrorV1> {
        if self.schema_version != MGA_MULTIGROUP_V1_SCHEMA_VERSION {
            return Err(invalid(
                "mga_multigroup_v1.schema",
                "mga_multigroup.schema_version",
                "MGA multigroup configuration requires schema version 1",
            ));
        }
        stable_id(&self.grouping_column, "mga_multigroup.grouping_column")?;
        if !(2..=MULTIMOD_MAX_GROUPS_V1).contains(&self.groups.len()) {
            return Err(invalid(
                "mga_multigroup_v1.group_count",
                "mga_multigroup.groups",
                "select between 2 and 20 groups",
            ));
        }
        unique_stable_ids(
            self.groups.iter().map(|group| group.group_id.as_str()),
            "mga_multigroup.groups.group_id",
        )?;
        let mut values = BTreeSet::new();
        for (index, group) in self.groups.iter().enumerate() {
            if group.label.trim().is_empty() {
                return Err(invalid(
                    "mga_multigroup_v1.group_label",
                    format!("mga_multigroup.groups[{index}].label"),
                    "group labels cannot be empty",
                ));
            }
            group
                .value
                .ensure_valid(&format!("mga_multigroup.groups[{index}].value"))?;
            if !values.insert(group.value.canonical_key()) {
                return Err(invalid(
                    "mga_multigroup_v1.group_value_duplicate",
                    format!("mga_multigroup.groups[{index}].value"),
                    "selected typed group values must be distinct",
                ));
            }
        }
        if self.procedures.is_empty() {
            return Err(invalid(
                "mga_multigroup_v1.procedures_empty",
                "mga_multigroup.procedures",
                "select at least one MGA procedure",
            ));
        }
        let unique_procedures = self.procedures.iter().copied().collect::<BTreeSet<_>>();
        if unique_procedures.len() != self.procedures.len() {
            return Err(invalid(
                "mga_multigroup_v1.procedure_duplicate",
                "mga_multigroup.procedures",
                "MGA procedures must be unique",
            ));
        }
        for (name, value) in [
            ("permutation_samples", self.permutation_samples),
            ("bootstrap_samples", self.bootstrap_samples),
        ] {
            if !(MGA_MIN_RESAMPLES_V1..=MGA_MAX_RESAMPLES_V1).contains(&value) {
                return Err(invalid(
                    "mga_multigroup_v1.resamples",
                    format!("mga_multigroup.{name}"),
                    "MGA permutation and bootstrap counts must be between 5000 and 10000",
                ));
            }
        }
        finite_probability(self.confidence_level, "mga_multigroup.confidence_level")?;
        finite_probability(self.alpha, "mga_multigroup.alpha")?;
        if self.confidence_level.to_bits() != 0.95f64.to_bits()
            || self.alpha.to_bits() != 0.05f64.to_bits()
        {
            return Err(invalid(
                "mga_multigroup_v1.fixed_error_rates",
                "mga_multigroup.confidence_level",
                "MGA V1 uses the frozen 95% confidence and alpha .05 contract",
            ));
        }
        if !self.configural_checklist.complete() {
            return Err(invalid(
                "mga_multigroup_v1.micom_step1_incomplete",
                "mga_multigroup.configural_checklist",
                "every MGA run requires explicit review of all MICOM Step 1 configural-invariance checks",
            ));
        }
        if self.groups.len() < 3
            && unique_procedures.contains(&MgaProcedureV1::OmnibusMaxSpreadPermutation)
        {
            return Err(invalid(
                "mga_multigroup_v1.omnibus_requires_three_groups",
                "mga_multigroup.procedures",
                "the max-spread omnibus procedure requires at least three selected groups",
            ));
        }
        let requests_pairwise_follow_up = unique_procedures.iter().any(|procedure| {
            matches!(
                procedure,
                MgaProcedureV1::MicomPairwise
                    | MgaProcedureV1::PairwisePermutation
                    | MgaProcedureV1::HenselerPlsMga
                    | MgaProcedureV1::BootstrapDifferenceBc
                    | MgaProcedureV1::ParametricPooledVariance
                    | MgaProcedureV1::ParametricWelchSatterthwaite
            )
        });
        if self.groups.len() >= 3
            && requests_pairwise_follow_up
            && !unique_procedures.contains(&MgaProcedureV1::OmnibusMaxSpreadPermutation)
        {
            return Err(invalid(
                "mga_multigroup_v1.omnibus_required",
                "mga_multigroup.procedures",
                "three through twenty groups require the max-spread omnibus permutation gate before pairwise follow-up",
            ));
        }

        let group_ids = self
            .groups
            .iter()
            .map(|group| group.group_id.as_str())
            .collect::<BTreeSet<_>>();
        match &self.comparison_plan {
            MgaComparisonPlanV1::ReferenceVsRest { reference_group_id } => {
                if !group_ids.contains(reference_group_id.as_str()) {
                    return Err(invalid(
                        "mga_multigroup_v1.reference_unknown",
                        "mga_multigroup.comparison_plan.reference_group_id",
                        "reference group must be one of the selected groups",
                    ));
                }
            }
            MgaComparisonPlanV1::SelectedPairs { pairs } => {
                if pairs.is_empty() {
                    return Err(invalid(
                        "mga_multigroup_v1.pairs_empty",
                        "mga_multigroup.comparison_plan.pairs",
                        "select at least one group pair",
                    ));
                }
                let mut seen = BTreeSet::new();
                for (index, pair) in pairs.iter().enumerate() {
                    if pair.left_group_id == pair.right_group_id
                        || !group_ids.contains(pair.left_group_id.as_str())
                        || !group_ids.contains(pair.right_group_id.as_str())
                    {
                        return Err(invalid(
                            "mga_multigroup_v1.pair_invalid",
                            format!("mga_multigroup.comparison_plan.pairs[{index}]"),
                            "group-pair members must be distinct selected groups",
                        ));
                    }
                    if !seen.insert(pair.canonical_key()) {
                        return Err(invalid(
                            "mga_multigroup_v1.pair_duplicate",
                            format!("mga_multigroup.comparison_plan.pairs[{index}]"),
                            "group-pair comparisons must be unique",
                        ));
                    }
                }
            }
            MgaComparisonPlanV1::AllPairs {
                heavy_run_confirmed,
            } if self.groups.len() == MULTIMOD_MAX_GROUPS_V1 && !heavy_run_confirmed => {
                return Err(invalid(
                    "mga_multigroup_v1.heavy_run_confirmation",
                    "mga_multigroup.comparison_plan.heavy_run_confirmed",
                    "all 190 comparisons at 20 groups require explicit heavy-run confirmation",
                ));
            }
            MgaComparisonPlanV1::AllPairs { .. } => {}
        }

        match (&self.profile, &self.weight) {
            (
                MgaModelProfileV1::CaseWeightedPls,
                Some(AnalysisWeightBindingV1::Case { column }),
            )
            | (
                MgaModelProfileV1::FrequencyWeightedPls,
                Some(AnalysisWeightBindingV1::Frequency { column }),
            ) => stable_id(column, "mga_multigroup.weight.column")?,
            (MgaModelProfileV1::CaseWeightedPls | MgaModelProfileV1::FrequencyWeightedPls, _) => {
                return Err(invalid(
                    "mga_multigroup_v1.weight_profile_mismatch",
                    "mga_multigroup.weight",
                    "weighted profiles require the matching explicit weight binding",
                ));
            }
            (_, Some(_)) => {
                return Err(invalid(
                    "mga_multigroup_v1.weight_profile_mismatch",
                    "mga_multigroup.weight",
                    "weights cannot be combined with this MGA profile",
                ));
            }
            (_, None) => {}
        }
        unique_stable_ids(
            self.selected_parameter_ids.iter().map(String::as_str),
            "mga_multigroup.selected_parameter_ids",
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum HeterogeneityInteractionProfileV2 {
    P0Structural,
    P2MultiTwoWay,
    P23AllCurrent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum HeterogeneityAlgorithmV2 {
    FimixPlsV2,
    PlsPosPublishedV2,
    PlsPosDestinationScoredInteractionsV2,
}

/// Frozen relative numerical allowance used only to distinguish floating-point
/// jitter from a material FIMIX observed-likelihood decrease. Accepted traces
/// themselves remain exactly non-decreasing.
pub const FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2: f64 = 1.0e-9;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixSettingsV2 {
    pub starts: u32,
    pub max_iterations: u32,
    pub relative_log_likelihood_tolerance: f64,
    pub consecutive_converged_iterations: u32,
    pub likelihood_decrease_tolerance: f64,
    pub residual_variance_floor: f64,
    pub rank_tolerance: f64,
    pub minimum_class_share: f64,
    pub required_reproducing_starts: u32,
    pub optimum_relative_log_likelihood_tolerance: f64,
    pub optimum_maximum_coefficient_difference: f64,
    pub optimum_mean_posterior_difference: f64,
}

impl Default for FimixSettingsV2 {
    fn default() -> Self {
        Self {
            starts: 30,
            max_iterations: 5_000,
            relative_log_likelihood_tolerance: 1e-10,
            consecutive_converged_iterations: 3,
            likelihood_decrease_tolerance: FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2,
            residual_variance_floor: 1e-8,
            rank_tolerance: 1e-11,
            minimum_class_share: 0.05,
            required_reproducing_starts: 2,
            optimum_relative_log_likelihood_tolerance: 1e-8,
            optimum_maximum_coefficient_difference: 1e-6,
            optimum_mean_posterior_difference: 1e-4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsPosSettingsV2 {
    pub starts: u32,
    pub strict_improvement_tolerance: f64,
    pub stable_objective_tolerance: f64,
    pub minimum_reproducing_starts: u32,
}

impl Default for PlsPosSettingsV2 {
    fn default() -> Self {
        Self {
            starts: 10,
            strict_improvement_tolerance: 1e-10,
            stable_objective_tolerance: 1e-10,
            minimum_reproducing_starts: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosCommonMetricComparabilityV1 {
    pub schema_version: u32,
    pub request_segment_contrasts: bool,
    pub permutation_samples: u32,
    pub configural_checklist: MicomConfiguralChecklistV1,
    pub require_partial_compositional_invariance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SegmentationBootstrapV2 {
    pub resamples: u32,
    pub seed: u64,
    pub confidence_level: f64,
}

/// Immutable analyst lock that binds fixed-K inference to one completed
/// discovery inventory.  The runner deterministically reproduces the entire
/// declared inventory and refuses inference when its scientific identity no
/// longer matches `discovery_result_identity_sha256`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HeterogeneityInferenceLockReceiptV2 {
    pub schema_version: u32,
    pub discovery_result_identity_sha256: String,
    pub discovery_candidate_k: Vec<u8>,
    pub discovery_algorithms: Vec<HeterogeneityAlgorithmV2>,
    pub selected_algorithm: HeterogeneityAlgorithmV2,
    pub selected_k: u8,
    pub analyst_lock_confirmed: bool,
    pub tandem_fimix_same_k_start_required: bool,
}

impl HeterogeneityInferenceLockReceiptV2 {
    pub fn ensure_valid(&self) -> Result<(), MultiModConfigErrorV1> {
        if self.schema_version != HETEROGENEITY_INFERENCE_LOCK_V2_SCHEMA_VERSION {
            return Err(invalid(
                "pls_heterogeneity_v2.inference_lock_schema",
                "pls_heterogeneity.phase.lock.schema_version",
                "heterogeneity inference lock requires schema version 1",
            ));
        }
        if !lowercase_sha256(&self.discovery_result_identity_sha256) {
            return Err(invalid(
                "pls_heterogeneity_v2.discovery_result_identity",
                "pls_heterogeneity.phase.lock.discovery_result_identity_sha256",
                "discovery result identity must be one lowercase SHA-256 digest",
            ));
        }
        if self.discovery_candidate_k.is_empty()
            || self
                .discovery_candidate_k
                .iter()
                .any(|k| !(2..=5).contains(k))
            || !self
                .discovery_candidate_k
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        {
            return Err(invalid(
                "pls_heterogeneity_v2.inference_lock_candidate_k",
                "pls_heterogeneity.phase.lock.discovery_candidate_k",
                "locked discovery K values must be unique, ascending, and between 2 and 5",
            ));
        }
        if self.discovery_algorithms.is_empty()
            || !self
                .discovery_algorithms
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || !self.discovery_algorithms.contains(&self.selected_algorithm)
        {
            return Err(invalid(
                "pls_heterogeneity_v2.inference_lock_algorithms",
                "pls_heterogeneity.phase.lock.discovery_algorithms",
                "locked discovery algorithms must be unique, canonical, and contain the selected algorithm",
            ));
        }
        if !self.discovery_candidate_k.contains(&self.selected_k) {
            return Err(invalid(
                "pls_heterogeneity_v2.inference_lock_selected_k",
                "pls_heterogeneity.phase.lock.selected_k",
                "selected K must be one of the locked discovery candidates and cannot be the pooled K=1 baseline",
            ));
        }
        if !self.analyst_lock_confirmed {
            return Err(invalid(
                "pls_heterogeneity_v2.inference_lock_confirmation",
                "pls_heterogeneity.phase.lock.analyst_lock_confirmed",
                "fixed-K inference requires explicit analyst lock confirmation",
            ));
        }
        let tandem_required = self.selected_algorithm != HeterogeneityAlgorithmV2::FimixPlsV2
            && self
                .discovery_algorithms
                .contains(&HeterogeneityAlgorithmV2::FimixPlsV2);
        if self.tandem_fimix_same_k_start_required != tandem_required {
            return Err(invalid(
                "pls_heterogeneity_v2.inference_lock_tandem_fimix",
                "pls_heterogeneity.phase.lock.tandem_fimix_same_k_start_required",
                "tandem FIMIX start authority must exactly reflect the locked discovery inventory",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HeterogeneityPhaseV2 {
    Discovery {
        candidate_k: Vec<u8>,
        algorithms: Vec<HeterogeneityAlgorithmV2>,
    },
    Inference {
        lock: HeterogeneityInferenceLockReceiptV2,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsUnobservedHeterogeneityConfigV2 {
    pub schema_version: u32,
    pub profile: HeterogeneityInteractionProfileV2,
    pub phase: HeterogeneityPhaseV2,
    pub seed: u64,
    pub fimix: FimixSettingsV2,
    pub pls_pos: PlsPosSettingsV2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos_common_metric: Option<PosCommonMetricComparabilityV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap: Option<SegmentationBootstrapV2>,
}

impl PlsUnobservedHeterogeneityConfigV2 {
    pub fn ensure_valid(&self) -> Result<(), MultiModConfigErrorV1> {
        if self.schema_version != PLS_HETEROGENEITY_V2_SCHEMA_VERSION {
            return Err(invalid(
                "pls_heterogeneity_v2.schema",
                "pls_heterogeneity.schema_version",
                "PLS heterogeneity configuration requires schema version 2",
            ));
        }
        if self.fimix.starts != 30
            || self.fimix.max_iterations == 0
            || self.fimix.max_iterations > 5_000
            || !self.fimix.relative_log_likelihood_tolerance.is_finite()
            || self.fimix.relative_log_likelihood_tolerance.to_bits() != 1e-10f64.to_bits()
            || self.fimix.consecutive_converged_iterations != 3
            || !self.fimix.likelihood_decrease_tolerance.is_finite()
            || self.fimix.likelihood_decrease_tolerance.to_bits()
                != FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2.to_bits()
            || !self.fimix.residual_variance_floor.is_finite()
            || self.fimix.residual_variance_floor < 1e-8
            || !self.fimix.rank_tolerance.is_finite()
            || self.fimix.rank_tolerance <= 0.0
            || !self.fimix.minimum_class_share.is_finite()
            || !(0.05..0.5).contains(&self.fimix.minimum_class_share)
            || self.fimix.required_reproducing_starts < 2
            || self.fimix.required_reproducing_starts > self.fimix.starts
            || !self
                .fimix
                .optimum_relative_log_likelihood_tolerance
                .is_finite()
            || self.fimix.optimum_relative_log_likelihood_tolerance <= 0.0
            || !self
                .fimix
                .optimum_maximum_coefficient_difference
                .is_finite()
            || self.fimix.optimum_maximum_coefficient_difference <= 0.0
            || !self.fimix.optimum_mean_posterior_difference.is_finite()
            || self.fimix.optimum_mean_posterior_difference <= 0.0
        {
            return Err(invalid(
                "pls_heterogeneity_v2.fimix_settings",
                "pls_heterogeneity.fimix",
                "FIMIX settings are outside the qualified numerical envelope",
            ));
        }
        if self.pls_pos.starts != 10
            || !self.pls_pos.strict_improvement_tolerance.is_finite()
            || self.pls_pos.strict_improvement_tolerance <= 0.0
            || !self.pls_pos.stable_objective_tolerance.is_finite()
            || self.pls_pos.stable_objective_tolerance <= 0.0
            || self.pls_pos.minimum_reproducing_starts != 2
        {
            return Err(invalid(
                "pls_heterogeneity_v2.pos_settings",
                "pls_heterogeneity.pls_pos",
                "PLS-POS requires ten starts, positive strict-improvement tolerance, and two reproducing starts",
            ));
        }

        let algorithms = match &self.phase {
            HeterogeneityPhaseV2::Discovery {
                candidate_k,
                algorithms,
            } => {
                if candidate_k.is_empty()
                    || candidate_k.iter().any(|k| !(2..=5).contains(k))
                    || !candidate_k.windows(2).all(|pair| pair[0] < pair[1])
                {
                    return Err(invalid(
                        "pls_heterogeneity_v2.candidate_k",
                        "pls_heterogeneity.phase.candidate_k",
                        "candidate K values must be unique, ascending, and between 2 and 5",
                    ));
                }
                if algorithms.is_empty()
                    || algorithms.iter().copied().collect::<BTreeSet<_>>().len() != algorithms.len()
                {
                    return Err(invalid(
                        "pls_heterogeneity_v2.algorithms",
                        "pls_heterogeneity.phase.algorithms",
                        "select one or more unique segmentation algorithms",
                    ));
                }
                if self.bootstrap.is_some() {
                    return Err(invalid(
                        "pls_heterogeneity_v2.discovery_bootstrap",
                        "pls_heterogeneity.bootstrap",
                        "bootstrap inference requires an explicitly locked algorithm and K",
                    ));
                }
                algorithms.clone()
            }
            HeterogeneityPhaseV2::Inference { lock } => {
                lock.ensure_valid()?;
                lock.discovery_algorithms.clone()
            }
        };

        if self.profile != HeterogeneityInteractionProfileV2::P0Structural
            && algorithms.contains(&HeterogeneityAlgorithmV2::PlsPosPublishedV2)
        {
            return Err(invalid(
                "pls_heterogeneity_v2.published_pos_interaction",
                "pls_heterogeneity.profile",
                "publication-faithful PLS-POS supports the structural P0 profile only",
            ));
        }
        if self.profile == HeterogeneityInteractionProfileV2::P0Structural
            && algorithms.contains(&HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2)
        {
            return Err(invalid(
                "pls_heterogeneity_v2.pos_extension_without_interaction",
                "pls_heterogeneity.profile",
                "the destination-scored PLS-POS extension is reserved for P2/P23 interactions",
            ));
        }

        if let Some(comparability) = &self.pos_common_metric {
            if comparability.schema_version != 1
                || !(MGA_MIN_RESAMPLES_V1..=MGA_MAX_RESAMPLES_V1)
                    .contains(&comparability.permutation_samples)
                || (comparability.request_segment_contrasts
                    && (!comparability.configural_checklist.complete()
                        || !comparability.require_partial_compositional_invariance))
            {
                return Err(invalid(
                    "pls_heterogeneity_v2.common_metric",
                    "pls_heterogeneity.pos_common_metric",
                    "POS contrasts require the complete common-metric comparability contract",
                ));
            }
            if comparability.request_segment_contrasts {
                let locked_pos = matches!(
                    &self.phase,
                    HeterogeneityPhaseV2::Inference { lock }
                        if lock.selected_algorithm != HeterogeneityAlgorithmV2::FimixPlsV2
                );
                if !locked_pos || self.bootstrap.is_none() {
                    return Err(invalid(
                        "pls_heterogeneity_v2.common_metric_locked_pos_bootstrap",
                        "pls_heterogeneity.pos_common_metric.request_segment_contrasts",
                        "POS common-metric contrasts require a locked POS algorithm/K and fixed-K bootstrap",
                    ));
                }
            }
        }
        if let Some(bootstrap) = &self.bootstrap {
            if !(HETEROGENEITY_MIN_BOOTSTRAP_V2..=HETEROGENEITY_MAX_BOOTSTRAP_V2)
                .contains(&bootstrap.resamples)
            {
                return Err(invalid(
                    "pls_heterogeneity_v2.bootstrap_resamples",
                    "pls_heterogeneity.bootstrap.resamples",
                    "segmentation bootstrap requires 500 through 10000 resamples",
                ));
            }
            finite_probability(
                bootstrap.confidence_level,
                "pls_heterogeneity.bootstrap.confidence_level",
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalProcessProfileV2 {
    MultiTwoWayPercentile,
    MultiTwoWayBca,
    MultiTwoWayStudentized,
    BoundedThreeWayPercentile,
    MultipleHocPercentile,
    GroupedPercentile,
    CaseWeightedPercentile,
    FrequencyWeightedPercentile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessPathV2 {
    pub path_id: String,
    pub ordered_relation_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalProbeScaleV2 {
    StandardizedScore,
    RawObservedWithTransformationReceipt,
}

/// Frozen numerical receipt for converting authored raw-unit probe values to
/// the exact standardized score metric used by the point fit. The analysis-row
/// mask is included because listwise/grouped/weighted profiles can have a
/// different center and sample denominator from the full dataset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalRawProbeTransformationReceiptV2 {
    pub contract: String,
    pub moderator_id: String,
    pub source_column: String,
    pub dataset_fingerprint: String,
    pub analysis_row_mask_sha256: String,
    pub center: f64,
    pub sample_standard_deviation: f64,
    pub orientation_sign: i8,
}

impl ConditionalRawProbeTransformationReceiptV2 {
    pub fn ensure_valid(&self, path: &str) -> Result<(), MultiModConfigErrorV1> {
        if self.contract != CONDITIONAL_RAW_PROBE_TRANSFORMATION_CONTRACT_V2 {
            return Err(invalid(
                "general_sem_conditional_process_v2.raw_probe_contract",
                format!("{path}.contract"),
                "raw-unit probes require the frozen sample-standardization contract",
            ));
        }
        stable_id(&self.moderator_id, &format!("{path}.moderator_id"))?;
        stable_id(&self.source_column, &format!("{path}.source_column"))?;
        stable_id(
            &self.dataset_fingerprint,
            &format!("{path}.dataset_fingerprint"),
        )?;
        if !lowercase_sha256(&self.analysis_row_mask_sha256) {
            return Err(invalid(
                "general_sem_conditional_process_v2.raw_probe_row_mask",
                format!("{path}.analysis_row_mask_sha256"),
                "the analysis-row mask requires a lowercase SHA-256 identity",
            ));
        }
        if !self.center.is_finite()
            || !self.sample_standard_deviation.is_finite()
            || self.sample_standard_deviation <= 0.0
            || !matches!(self.orientation_sign, -1 | 1)
        {
            return Err(invalid(
                "general_sem_conditional_process_v2.raw_probe_parameters",
                path,
                "raw-unit probe center/scale must be finite, scale positive, and orientation sign plus or minus one",
            ));
        }
        Ok(())
    }

    pub fn standardize(&self, raw_value: f64) -> Result<f64, MultiModConfigErrorV1> {
        let standardized = f64::from(self.orientation_sign) * (raw_value - self.center)
            / self.sample_standard_deviation;
        if standardized.is_finite() {
            Ok(standardized)
        } else {
            Err(invalid(
                "general_sem_conditional_process_v2.raw_probe_nonfinite",
                "conditional_process.probes.values",
                "a raw-unit probe did not transform to a finite standardized score",
            ))
        }
    }
}

/// Identity of the original-sample score metric to which a raw-unit probe is
/// anchored. Grouped conditional-process analyses have one independent metric
/// per selected group; every other admitted profile has one analysis-fit
/// metric.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionalRawProbeFitScopeV2 {
    AnalysisFit,
    GroupFit { group_id: String },
}

impl ConditionalRawProbeFitScopeV2 {
    fn ensure_valid(&self, path: &str) -> Result<(), MultiModConfigErrorV1> {
        if let Self::GroupFit { group_id } = self {
            stable_id(group_id, &format!("{path}.group_id"))?;
        }
        Ok(())
    }
}

/// Denominator semantics used by the frozen raw-to-score transformation.
/// Frequency counts intentionally use the expanded-sample denominator so the
/// compact count-space calculation is exactly equal to physical row expansion.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalRawProbeMetricBasisV2 {
    UnweightedSample,
    CaseWeightedEffectiveDf,
    FrequencyExpandedSample,
}

/// Frozen original-sample receipt for a raw observed moderator. It binds the
/// transformation to the exact retained rows, per-row masses, fit/group scope,
/// and score orientation. Resamples reuse this anchor rather than recomputing
/// probe locations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalRawProbeFitMetricReceiptV2 {
    pub contract: String,
    pub moderator_id: String,
    pub source_column: String,
    pub dataset_fingerprint: String,
    pub analysis_row_mask_sha256: String,
    pub fit_scope: ConditionalRawProbeFitScopeV2,
    pub metric_basis: ConditionalRawProbeMetricBasisV2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_column: Option<String>,
    pub row_mass_sha256: String,
    pub compact_row_count: u32,
    pub mass_sum: f64,
    pub mass_squared_sum: f64,
    pub effective_degrees_of_freedom: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_total: Option<u64>,
    pub center: f64,
    pub standard_deviation: f64,
    pub orientation_sign: i8,
}

impl ConditionalRawProbeFitMetricReceiptV2 {
    pub fn ensure_valid(&self, path: &str) -> Result<(), MultiModConfigErrorV1> {
        if self.contract != CONDITIONAL_RAW_PROBE_FIT_METRIC_CONTRACT_V2 {
            return Err(invalid(
                "general_sem_conditional_process_v2.raw_probe_fit_metric_contract",
                format!("{path}.contract"),
                "fit-metric raw probes require the versioned fit-metric contract",
            ));
        }
        stable_id(&self.moderator_id, &format!("{path}.moderator_id"))?;
        stable_id(&self.source_column, &format!("{path}.source_column"))?;
        stable_id(
            &self.dataset_fingerprint,
            &format!("{path}.dataset_fingerprint"),
        )?;
        self.fit_scope.ensure_valid(&format!("{path}.fit_scope"))?;
        if let Some(column) = &self.weight_column {
            stable_id(column, &format!("{path}.weight_column"))?;
        }
        if !lowercase_sha256(&self.analysis_row_mask_sha256)
            || !lowercase_sha256(&self.row_mass_sha256)
        {
            return Err(invalid(
                "general_sem_conditional_process_v2.raw_probe_fit_metric_digest",
                path,
                "fit-metric row-mask and row-mass identities require lowercase SHA-256 values",
            ));
        }
        if self.compact_row_count < 2
            || !self.mass_sum.is_finite()
            || self.mass_sum <= 0.0
            || !self.mass_squared_sum.is_finite()
            || self.mass_squared_sum <= 0.0
            || !self.effective_degrees_of_freedom.is_finite()
            || self.effective_degrees_of_freedom <= 0.0
            || !self.center.is_finite()
            || !self.standard_deviation.is_finite()
            || self.standard_deviation <= 0.0
            || !matches!(self.orientation_sign, -1 | 1)
        {
            return Err(invalid(
                "general_sem_conditional_process_v2.raw_probe_fit_metric_parameters",
                path,
                "fit-metric mass, effective degrees of freedom, center, scale, and orientation are invalid",
            ));
        }
        let approximately_equal = |left: f64, right: f64| {
            let scale = left.abs().max(right.abs()).max(1.0);
            (left - right).abs() <= 1.0e-12 * scale
        };
        match self.metric_basis {
            ConditionalRawProbeMetricBasisV2::UnweightedSample => {
                let rows = f64::from(self.compact_row_count);
                if self.weight_column.is_some()
                    || self.frequency_total.is_some()
                    || !approximately_equal(self.mass_sum, rows)
                    || !approximately_equal(self.mass_squared_sum, rows)
                    || !approximately_equal(self.effective_degrees_of_freedom, rows - 1.0)
                {
                    return Err(invalid(
                        "general_sem_conditional_process_v2.raw_probe_unweighted_metric",
                        path,
                        "unweighted fit metrics require unit row masses and n minus one degrees of freedom",
                    ));
                }
            }
            ConditionalRawProbeMetricBasisV2::CaseWeightedEffectiveDf => {
                let expected_df = self.mass_sum - self.mass_squared_sum / self.mass_sum;
                if self.weight_column.is_none()
                    || self.frequency_total.is_some()
                    || !approximately_equal(self.effective_degrees_of_freedom, expected_df)
                {
                    return Err(invalid(
                        "general_sem_conditional_process_v2.raw_probe_case_weight_metric",
                        path,
                        "case-weighted fit metrics require the reliability-weight effective denominator",
                    ));
                }
            }
            ConditionalRawProbeMetricBasisV2::FrequencyExpandedSample => {
                let Some(total) = self.frequency_total else {
                    return Err(invalid(
                        "general_sem_conditional_process_v2.raw_probe_frequency_metric",
                        path,
                        "frequency fit metrics require the represented expanded-row total",
                    ));
                };
                if self.weight_column.is_none()
                    || total < 2
                    || !approximately_equal(self.mass_sum, total as f64)
                    || !approximately_equal(self.effective_degrees_of_freedom, total as f64 - 1.0)
                {
                    return Err(invalid(
                        "general_sem_conditional_process_v2.raw_probe_frequency_metric",
                        path,
                        "frequency fit metrics require exact expanded count mass and N minus one degrees of freedom",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn standardize(&self, raw_value: f64) -> Result<f64, MultiModConfigErrorV1> {
        let standardized =
            f64::from(self.orientation_sign) * (raw_value - self.center) / self.standard_deviation;
        if standardized.is_finite() {
            Ok(standardized)
        } else {
            Err(invalid(
                "general_sem_conditional_process_v2.raw_probe_nonfinite",
                "conditional_process.probes.values",
                "a raw-unit probe did not transform to a finite standardized score",
            ))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalModeratorProbeV2 {
    pub probe_id: String,
    pub moderator_id: String,
    pub scale: ConditionalProbeScaleV2,
    pub values: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_transformation_receipt: Option<ConditionalRawProbeTransformationReceiptV2>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_fit_metric_receipts: Vec<ConditionalRawProbeFitMetricReceiptV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalJointProbeTupleV2 {
    pub tuple_id: String,
    pub values_by_moderator: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProbeContrastV2 {
    pub contrast_id: String,
    pub left_tuple_id: String,
    pub right_tuple_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalGroupContrastV2 {
    pub contrast_id: String,
    pub left_group_id: String,
    pub right_group_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessEstimandsV2 {
    pub conditional_specific_indirect: bool,
    pub conditional_total_indirect: bool,
    pub conditional_total_effect: bool,
    pub scalar_index_when_affine: bool,
    pub local_first_derivatives: bool,
    pub local_second_and_cross_derivatives: bool,
    pub finite_probe_contrasts: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalProcessIntervalV2 {
    Percentile,
    Bca,
    Studentized,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessInferenceV2 {
    pub interval: ConditionalProcessIntervalV2,
    pub alternative: InferenceAlternativeV1,
    pub outer_resamples: u32,
    pub inner_resamples: u32,
    pub seed: u64,
    pub confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemConditionalProcessConfigV2 {
    pub schema_version: u32,
    pub profile: ConditionalProcessProfileV2,
    pub paths: Vec<ConditionalProcessPathV2>,
    pub declared_interaction_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub three_way_interaction_id: Option<String>,
    #[serde(default)]
    pub hoc_ids: Vec<String>,
    pub moderator_ids: Vec<String>,
    pub probes: Vec<ConditionalModeratorProbeV2>,
    #[serde(default)]
    pub explicit_joint_tuples: Vec<ConditionalJointProbeTupleV2>,
    #[serde(default)]
    pub probe_contrasts: Vec<ConditionalProbeContrastV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grouping_column: Option<String>,
    #[serde(default)]
    pub groups: Vec<SelectedGroupV1>,
    #[serde(default)]
    pub group_contrasts: Vec<ConditionalGroupContrastV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<AnalysisWeightBindingV1>,
    pub estimands: ConditionalProcessEstimandsV2,
    pub inference: ConditionalProcessInferenceV2,
}

pub(crate) fn conditional_process_target_upper_bound_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    probe_points: usize,
    group_count: usize,
) -> usize {
    let moderator_count = config.moderator_ids.len();
    let requested_cell_effects = usize::from(config.estimands.conditional_specific_indirect)
        + usize::from(config.estimands.conditional_total_indirect)
        + usize::from(config.estimands.conditional_total_effect)
        + usize::from(config.estimands.local_first_derivatives) * moderator_count
        + usize::from(config.estimands.local_second_and_cross_derivatives)
            * moderator_count
            * (moderator_count + 1)
            / 2;
    let path_count = config.paths.len();
    let local_targets = path_count
        .saturating_mul(probe_points)
        .saturating_mul(group_count)
        .saturating_mul(requested_cell_effects);
    let scalar_index_targets = usize::from(config.estimands.scalar_index_when_affine)
        .saturating_mul(path_count)
        .saturating_mul(group_count);
    let probe_contrast_targets = usize::from(config.estimands.finite_probe_contrasts)
        .saturating_mul(path_count)
        .saturating_mul(group_count)
        .saturating_mul(config.probe_contrasts.len());
    let group_contrast_targets = config
        .group_contrasts
        .len()
        .saturating_mul(path_count)
        .saturating_mul(probe_points)
        .saturating_mul(
            requested_cell_effects + usize::from(config.estimands.scalar_index_when_affine),
        );
    local_targets
        .saturating_add(scalar_index_targets)
        .saturating_add(probe_contrast_targets)
        .saturating_add(group_contrast_targets)
}

impl GeneralSemConditionalProcessConfigV2 {
    pub fn ensure_valid(&self) -> Result<(), MultiModConfigErrorV1> {
        if self.schema_version != GENERAL_SEM_CONDITIONAL_PROCESS_V2_SCHEMA_VERSION {
            return Err(invalid(
                "general_sem_conditional_process_v2.schema",
                "conditional_process.schema_version",
                "conditional-process configuration requires schema version 2",
            ));
        }
        let (max_interactions, max_paths, min_edges, max_edges, max_moderators) = match self.profile
        {
            ConditionalProcessProfileV2::MultiTwoWayPercentile
            | ConditionalProcessProfileV2::MultiTwoWayBca => (8, 8, 2, 6, 4),
            ConditionalProcessProfileV2::MultiTwoWayStudentized => (4, 2, 2, 4, 3),
            ConditionalProcessProfileV2::BoundedThreeWayPercentile => (8, 4, 2, 5, 4),
            ConditionalProcessProfileV2::MultipleHocPercentile => (2, 8, 2, 6, 4),
            ConditionalProcessProfileV2::GroupedPercentile
            | ConditionalProcessProfileV2::CaseWeightedPercentile
            | ConditionalProcessProfileV2::FrequencyWeightedPercentile => (4, 4, 2, 4, 4),
        };
        if self.paths.is_empty() || self.paths.len() > max_paths {
            return Err(invalid(
                "general_sem_conditional_process_v2.path_count",
                "conditional_process.paths",
                format!("this profile requires 1 through {max_paths} explicit paths"),
            ));
        }
        unique_stable_ids(
            self.paths.iter().map(|path| path.path_id.as_str()),
            "conditional_process.paths.path_id",
        )?;
        for (index, path) in self.paths.iter().enumerate() {
            if !(min_edges..=max_edges).contains(&path.ordered_relation_ids.len()) {
                return Err(invalid(
                    "general_sem_conditional_process_v2.path_length",
                    format!("conditional_process.paths[{index}].ordered_relation_ids"),
                    format!("path length must be between {min_edges} and {max_edges}"),
                ));
            }
            unique_stable_ids(
                path.ordered_relation_ids.iter().map(String::as_str),
                &format!("conditional_process.paths[{index}].ordered_relation_ids"),
            )?;
        }
        if self.declared_interaction_ids.is_empty()
            || self.declared_interaction_ids.len() > max_interactions
        {
            return Err(invalid(
                "general_sem_conditional_process_v2.interaction_count",
                "conditional_process.declared_interaction_ids",
                format!("this profile requires 1 through {max_interactions} interactions"),
            ));
        }
        unique_stable_ids(
            self.declared_interaction_ids.iter().map(String::as_str),
            "conditional_process.declared_interaction_ids",
        )?;
        if self.moderator_ids.is_empty() || self.moderator_ids.len() > max_moderators {
            return Err(invalid(
                "general_sem_conditional_process_v2.moderator_count",
                "conditional_process.moderator_ids",
                format!("this profile requires 1 through {max_moderators} moderators"),
            ));
        }
        unique_stable_ids(
            self.moderator_ids.iter().map(String::as_str),
            "conditional_process.moderator_ids",
        )?;

        match self.profile {
            ConditionalProcessProfileV2::BoundedThreeWayPercentile => {
                let id = self.three_way_interaction_id.as_deref().ok_or_else(|| {
                    invalid(
                        "general_sem_conditional_process_v2.three_way_required",
                        "conditional_process.three_way_interaction_id",
                        "the bounded three-way profile requires exactly one top interaction",
                    )
                })?;
                stable_id(id, "conditional_process.three_way_interaction_id")?;
            }
            _ if self.three_way_interaction_id.is_some() => {
                return Err(invalid(
                    "general_sem_conditional_process_v2.three_way_profile",
                    "conditional_process.three_way_interaction_id",
                    "three-way terms are admitted only by the bounded three-way profile",
                ));
            }
            _ => {}
        }
        if self.profile == ConditionalProcessProfileV2::MultipleHocPercentile {
            if self.hoc_ids.is_empty() || self.hoc_ids.len() > 4 {
                return Err(invalid(
                    "general_sem_conditional_process_v2.hoc_count",
                    "conditional_process.hoc_ids",
                    "the multiple-HOC profile requires one through four disjoint HOCs",
                ));
            }
            unique_stable_ids(
                self.hoc_ids.iter().map(String::as_str),
                "conditional_process.hoc_ids",
            )?;
        } else if !self.hoc_ids.is_empty() {
            return Err(invalid(
                "general_sem_conditional_process_v2.hoc_profile",
                "conditional_process.hoc_ids",
                "HOCs are admitted only by the multiple-HOC profile",
            ));
        }

        if self.probes.len() != self.moderator_ids.len() {
            return Err(invalid(
                "general_sem_conditional_process_v2.probe_coverage",
                "conditional_process.probes",
                "each moderator requires exactly one probe declaration",
            ));
        }
        let moderators = self
            .moderator_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut probe_moderators = BTreeSet::new();
        let mut cartesian_points = 1usize;
        for (index, probe) in self.probes.iter().enumerate() {
            stable_id(
                &probe.probe_id,
                &format!("conditional_process.probes[{index}].probe_id"),
            )?;
            if !moderators.contains(probe.moderator_id.as_str())
                || !probe_moderators.insert(probe.moderator_id.as_str())
                || probe.values.is_empty()
                || probe.values.len() > 5
                || probe.values.iter().any(|value| !value.is_finite())
            {
                return Err(invalid(
                    "general_sem_conditional_process_v2.probe_invalid",
                    format!("conditional_process.probes[{index}]"),
                    "probe moderators must be unique and declared, with one through five finite values",
                ));
            }
            if probe
                .values
                .windows(2)
                .any(|pair| pair[0].total_cmp(&pair[1]).is_ge())
            {
                return Err(invalid(
                    "general_sem_conditional_process_v2.probe_order",
                    format!("conditional_process.probes[{index}].values"),
                    "probe values must be unique and ascending",
                ));
            }
            match probe.scale {
                ConditionalProbeScaleV2::StandardizedScore => {
                    if probe.raw_transformation_receipt.is_some()
                        || !probe.raw_fit_metric_receipts.is_empty()
                    {
                        return Err(invalid(
                            "general_sem_conditional_process_v2.raw_probe_receipt_unexpected",
                            format!("conditional_process.probes[{index}]"),
                            "standardized-score probes cannot carry raw-unit transformation receipts",
                        ));
                    }
                }
                ConditionalProbeScaleV2::RawObservedWithTransformationReceipt => {
                    if probe.raw_transformation_receipt.is_some()
                        && !probe.raw_fit_metric_receipts.is_empty()
                    {
                        return Err(invalid(
                            "general_sem_conditional_process_v2.raw_probe_receipt_ambiguous",
                            format!("conditional_process.probes[{index}]"),
                            "a raw-unit probe must use either the legacy sample receipt or the fit-metric receipt inventory, never both",
                        ));
                    }
                    if let Some(receipt) = &probe.raw_transformation_receipt {
                        if matches!(
                            self.profile,
                            ConditionalProcessProfileV2::GroupedPercentile
                                | ConditionalProcessProfileV2::CaseWeightedPercentile
                                | ConditionalProcessProfileV2::FrequencyWeightedPercentile
                        ) {
                            return Err(invalid(
                                "general_sem_conditional_process_v2.raw_probe_fit_metric_required",
                                format!("conditional_process.probes[{index}]"),
                                "grouped and weighted raw probes require scoped fit-metric receipts",
                            ));
                        }
                        receipt.ensure_valid(&format!(
                            "conditional_process.probes[{index}].raw_transformation_receipt"
                        ))?;
                        if receipt.moderator_id != probe.moderator_id {
                            return Err(invalid(
                                "general_sem_conditional_process_v2.raw_probe_moderator",
                                format!(
                                    "conditional_process.probes[{index}].raw_transformation_receipt.moderator_id"
                                ),
                                "the raw-unit transformation receipt must identify its probe moderator",
                            ));
                        }
                        for value in &probe.values {
                            receipt.standardize(*value)?;
                        }
                    } else if probe.raw_fit_metric_receipts.is_empty() {
                        return Err(invalid(
                            "general_sem_conditional_process_v2.raw_probe_receipt_missing",
                            format!("conditional_process.probes[{index}]"),
                            "raw-unit probes require a frozen numerical transformation receipt",
                        ));
                    } else {
                        let mut scopes = BTreeSet::new();
                        for (receipt_index, receipt) in
                            probe.raw_fit_metric_receipts.iter().enumerate()
                        {
                            let receipt_path = format!(
                                "conditional_process.probes[{index}].raw_fit_metric_receipts[{receipt_index}]"
                            );
                            receipt.ensure_valid(&receipt_path)?;
                            if receipt.moderator_id != probe.moderator_id {
                                return Err(invalid(
                                    "general_sem_conditional_process_v2.raw_probe_moderator",
                                    format!("{receipt_path}.moderator_id"),
                                    "the fit-metric receipt must identify its probe moderator",
                                ));
                            }
                            if !scopes.insert(receipt.fit_scope.clone()) {
                                return Err(invalid(
                                    "general_sem_conditional_process_v2.raw_probe_scope_duplicate",
                                    format!("{receipt_path}.fit_scope"),
                                    "each original-sample fit scope requires exactly one receipt",
                                ));
                            }
                            for value in &probe.values {
                                receipt.standardize(*value)?;
                            }
                        }
                        let (expected_scopes, expected_basis, expected_weight_column) =
                            match self.profile {
                                ConditionalProcessProfileV2::GroupedPercentile => (
                                    self.groups
                                        .iter()
                                        .map(|group| ConditionalRawProbeFitScopeV2::GroupFit {
                                            group_id: group.group_id.clone(),
                                        })
                                        .collect::<BTreeSet<_>>(),
                                    ConditionalRawProbeMetricBasisV2::UnweightedSample,
                                    None,
                                ),
                                ConditionalProcessProfileV2::CaseWeightedPercentile => (
                                    BTreeSet::from([ConditionalRawProbeFitScopeV2::AnalysisFit]),
                                    ConditionalRawProbeMetricBasisV2::CaseWeightedEffectiveDf,
                                    self.weight.as_ref().and_then(|weight| match weight {
                                        AnalysisWeightBindingV1::Case { column } => {
                                            Some(column.as_str())
                                        }
                                        _ => None,
                                    }),
                                ),
                                ConditionalProcessProfileV2::FrequencyWeightedPercentile => (
                                    BTreeSet::from([ConditionalRawProbeFitScopeV2::AnalysisFit]),
                                    ConditionalRawProbeMetricBasisV2::FrequencyExpandedSample,
                                    self.weight.as_ref().and_then(|weight| match weight {
                                        AnalysisWeightBindingV1::Frequency { column } => {
                                            Some(column.as_str())
                                        }
                                        _ => None,
                                    }),
                                ),
                                _ => (
                                    BTreeSet::from([ConditionalRawProbeFitScopeV2::AnalysisFit]),
                                    ConditionalRawProbeMetricBasisV2::UnweightedSample,
                                    None,
                                ),
                            };
                        if scopes != expected_scopes
                            || probe.raw_fit_metric_receipts.iter().any(|receipt| {
                                receipt.metric_basis != expected_basis
                                    || receipt.weight_column.as_deref() != expected_weight_column
                            })
                        {
                            return Err(invalid(
                                "general_sem_conditional_process_v2.raw_probe_scope_profile",
                                format!(
                                    "conditional_process.probes[{index}].raw_fit_metric_receipts"
                                ),
                                "fit-metric receipt scopes, basis, or weight binding differ from the selected profile",
                            ));
                        }
                        let first = &probe.raw_fit_metric_receipts[0];
                        if probe.raw_fit_metric_receipts.iter().skip(1).any(|receipt| {
                            receipt.source_column != first.source_column
                                || receipt.dataset_fingerprint != first.dataset_fingerprint
                                || receipt.analysis_row_mask_sha256
                                    != first.analysis_row_mask_sha256
                        }) {
                            return Err(invalid(
                                "general_sem_conditional_process_v2.raw_probe_fit_authority",
                                format!(
                                    "conditional_process.probes[{index}].raw_fit_metric_receipts"
                                ),
                                "all scoped fit-metric receipts for a moderator must share one source, dataset, and analysis-row mask",
                            ));
                        }
                    }
                }
            }
            cartesian_points = cartesian_points.saturating_mul(probe.values.len());
        }
        if cartesian_points > 81 {
            return Err(invalid(
                "general_sem_conditional_process_v2.cartesian_probe_cap",
                "conditional_process.probes",
                "the Cartesian probe grid cannot exceed 81 points",
            ));
        }
        if self.explicit_joint_tuples.len() > 100 || self.probe_contrasts.len() > 16 {
            return Err(invalid(
                "general_sem_conditional_process_v2.probe_cap",
                "conditional_process.explicit_joint_tuples",
                "explicit tuples are capped at 100 and probe contrasts at 16",
            ));
        }
        unique_stable_ids(
            self.explicit_joint_tuples
                .iter()
                .map(|tuple| tuple.tuple_id.as_str()),
            "conditional_process.explicit_joint_tuples.tuple_id",
        )?;
        for (index, tuple) in self.explicit_joint_tuples.iter().enumerate() {
            if tuple.values_by_moderator.len() != moderators.len()
                || tuple
                    .values_by_moderator
                    .iter()
                    .any(|(id, value)| !moderators.contains(id.as_str()) || !value.is_finite())
            {
                return Err(invalid(
                    "general_sem_conditional_process_v2.joint_tuple",
                    format!("conditional_process.explicit_joint_tuples[{index}]"),
                    "joint tuples require one finite value for every declared moderator",
                ));
            }
        }
        unique_stable_ids(
            self.probe_contrasts
                .iter()
                .map(|contrast| contrast.contrast_id.as_str()),
            "conditional_process.probe_contrasts.contrast_id",
        )?;
        let tuple_ids = self
            .explicit_joint_tuples
            .iter()
            .map(|tuple| tuple.tuple_id.as_str())
            .collect::<BTreeSet<_>>();
        for (index, contrast) in self.probe_contrasts.iter().enumerate() {
            if contrast.left_tuple_id == contrast.right_tuple_id
                || !tuple_ids.contains(contrast.left_tuple_id.as_str())
                || !tuple_ids.contains(contrast.right_tuple_id.as_str())
            {
                return Err(invalid(
                    "general_sem_conditional_process_v2.probe_contrast",
                    format!("conditional_process.probe_contrasts[{index}]"),
                    "probe contrasts require two distinct declared explicit joint tuples",
                ));
            }
        }
        if self.estimands.finite_probe_contrasts != !self.probe_contrasts.is_empty() {
            return Err(invalid(
                "general_sem_conditional_process_v2.probe_contrast_estimand",
                "conditional_process.estimands.finite_probe_contrasts",
                "finite probe contrasts require one or more declared contrasts, and declared contrasts must be selected as an estimand",
            ));
        }

        let (groups, group_factor) =
            if self.profile == ConditionalProcessProfileV2::GroupedPercentile {
                let column = self.grouping_column.as_deref().ok_or_else(|| {
                    invalid(
                        "general_sem_conditional_process_v2.group_column",
                        "conditional_process.grouping_column",
                        "the grouped profile requires an explicit grouping column",
                    )
                })?;
                stable_id(column, "conditional_process.grouping_column")?;
                if !(2..=MULTIMOD_MAX_GROUPS_V1).contains(&self.groups.len()) {
                    return Err(invalid(
                        "general_sem_conditional_process_v2.group_count",
                        "conditional_process.groups",
                        "the grouped profile requires 2 through 20 groups",
                    ));
                }
                unique_stable_ids(
                    self.groups.iter().map(|group| group.group_id.as_str()),
                    "conditional_process.groups.group_id",
                )?;
                let mut group_values = BTreeSet::new();
                for (index, group) in self.groups.iter().enumerate() {
                    if group.label.trim().is_empty() {
                        return Err(invalid(
                            "general_sem_conditional_process_v2.group_label",
                            format!("conditional_process.groups[{index}].label"),
                            "group labels cannot be empty",
                        ));
                    }
                    group
                        .value
                        .ensure_valid(&format!("conditional_process.groups[{index}].value"))?;
                    if !group_values.insert(group.value.canonical_key()) {
                        return Err(invalid(
                            "general_sem_conditional_process_v2.group_value_duplicate",
                            format!("conditional_process.groups[{index}].value"),
                            "selected typed group values must be distinct",
                        ));
                    }
                }
                let maximum_pairs = self.groups.len() * (self.groups.len() - 1) / 2;
                if self.group_contrasts.len() > maximum_pairs {
                    return Err(invalid(
                        "general_sem_conditional_process_v2.group_contrast_cap",
                        "conditional_process.group_contrasts",
                        "group contrasts cannot exceed the number of unique selected-group pairs",
                    ));
                }
                unique_stable_ids(
                    self.group_contrasts
                        .iter()
                        .map(|contrast| contrast.contrast_id.as_str()),
                    "conditional_process.group_contrasts.contrast_id",
                )?;
                let group_ids = self
                    .groups
                    .iter()
                    .map(|group| group.group_id.as_str())
                    .collect::<BTreeSet<_>>();
                let mut group_pairs = BTreeSet::new();
                for (index, contrast) in self.group_contrasts.iter().enumerate() {
                    if contrast.left_group_id == contrast.right_group_id
                        || !group_ids.contains(contrast.left_group_id.as_str())
                        || !group_ids.contains(contrast.right_group_id.as_str())
                    {
                        return Err(invalid(
                            "general_sem_conditional_process_v2.group_contrast",
                            format!("conditional_process.group_contrasts[{index}]"),
                            "group contrasts require two distinct selected groups",
                        ));
                    }
                    let pair = if contrast.left_group_id <= contrast.right_group_id {
                        (&contrast.left_group_id, &contrast.right_group_id)
                    } else {
                        (&contrast.right_group_id, &contrast.left_group_id)
                    };
                    if !group_pairs.insert(pair) {
                        return Err(invalid(
                            "general_sem_conditional_process_v2.group_contrast_duplicate",
                            format!("conditional_process.group_contrasts[{index}]"),
                            "each selected-group pair can appear only once",
                        ));
                    }
                }
                (self.groups.len(), self.groups.len())
            } else {
                if self.grouping_column.is_some()
                    || !self.groups.is_empty()
                    || !self.group_contrasts.is_empty()
                {
                    return Err(invalid(
                        "general_sem_conditional_process_v2.group_profile",
                        "conditional_process.groups",
                        "groups can be combined only with the grouped profile",
                    ));
                }
                (1, 1)
            };
        let _ = groups;

        match (&self.profile, &self.weight) {
            (
                ConditionalProcessProfileV2::CaseWeightedPercentile,
                Some(AnalysisWeightBindingV1::Case { column }),
            )
            | (
                ConditionalProcessProfileV2::FrequencyWeightedPercentile,
                Some(AnalysisWeightBindingV1::Frequency { column }),
            ) => stable_id(column, "conditional_process.weight.column")?,
            (
                ConditionalProcessProfileV2::CaseWeightedPercentile
                | ConditionalProcessProfileV2::FrequencyWeightedPercentile,
                _,
            ) => {
                return Err(invalid(
                    "general_sem_conditional_process_v2.weight_profile",
                    "conditional_process.weight",
                    "weighted profiles require their matching weight binding",
                ));
            }
            (_, Some(_)) => {
                return Err(invalid(
                    "general_sem_conditional_process_v2.weight_profile",
                    "conditional_process.weight",
                    "weights cannot be combined with this profile",
                ));
            }
            (_, None) => {}
        }

        let expected_interval = match self.profile {
            ConditionalProcessProfileV2::MultiTwoWayBca => ConditionalProcessIntervalV2::Bca,
            ConditionalProcessProfileV2::MultiTwoWayStudentized => {
                ConditionalProcessIntervalV2::Studentized
            }
            _ => ConditionalProcessIntervalV2::Percentile,
        };
        if self.inference.interval != expected_interval {
            return Err(invalid(
                "general_sem_conditional_process_v2.interval_profile",
                "conditional_process.inference.interval",
                "the selected interval family is not admitted by this profile",
            ));
        }
        if matches!(
            self.profile,
            ConditionalProcessProfileV2::MultipleHocPercentile
                | ConditionalProcessProfileV2::GroupedPercentile
                | ConditionalProcessProfileV2::CaseWeightedPercentile
                | ConditionalProcessProfileV2::FrequencyWeightedPercentile
        ) && self.inference.alternative != InferenceAlternativeV1::TwoSided
        {
            return Err(invalid(
                "general_sem_conditional_process_v2.alternative_profile",
                "conditional_process.inference.alternative",
                "this profile is qualified for two-sided inference only",
            ));
        }
        if !(CONDITIONAL_PROCESS_MIN_BOOTSTRAP_V2..=CONDITIONAL_PROCESS_MAX_BOOTSTRAP_V2)
            .contains(&self.inference.outer_resamples)
        {
            return Err(invalid(
                "general_sem_conditional_process_v2.outer_resamples",
                "conditional_process.inference.outer_resamples",
                "outer resamples must be between 500 and 10000",
            ));
        }
        match self.inference.interval {
            ConditionalProcessIntervalV2::Studentized => {
                if !(100..=1_000).contains(&self.inference.inner_resamples)
                    || self.inference.outer_resamples > 5_000
                    || u64::from(self.inference.outer_resamples + 1)
                        * u64::from(self.inference.inner_resamples)
                        > 1_000_000
                {
                    return Err(invalid(
                        "general_sem_conditional_process_v2.studentized_budget",
                        "conditional_process.inference",
                        "studentized inference exceeds its outer/inner refit envelope",
                    ));
                }
            }
            _ if self.inference.inner_resamples != 0 => {
                return Err(invalid(
                    "general_sem_conditional_process_v2.inner_resamples",
                    "conditional_process.inference.inner_resamples",
                    "inner resamples are reserved for studentized inference",
                ));
            }
            _ => {}
        }
        finite_probability(
            self.inference.confidence_level,
            "conditional_process.inference.confidence_level",
        )?;

        if !self.estimands.conditional_specific_indirect
            && !self.estimands.conditional_total_indirect
            && !self.estimands.conditional_total_effect
            && !self.estimands.scalar_index_when_affine
            && !self.estimands.local_first_derivatives
            && !self.estimands.local_second_and_cross_derivatives
            && !self.estimands.finite_probe_contrasts
        {
            return Err(invalid(
                "general_sem_conditional_process_v2.estimand_required",
                "conditional_process.estimands",
                "select at least one explicit conditional-process estimand",
            ));
        }

        let points = if self.explicit_joint_tuples.is_empty() {
            cartesian_points
        } else {
            self.explicit_joint_tuples.len()
        };
        if self.profile == ConditionalProcessProfileV2::MultiTwoWayStudentized && points > 27 {
            return Err(invalid(
                "general_sem_conditional_process_v2.studentized_probe_cap",
                "conditional_process.probes",
                "studentized inference supports at most 27 joint probe points",
            ));
        }
        let cells = self
            .paths
            .len()
            .saturating_mul(points)
            .saturating_mul(group_factor);
        if cells > CONDITIONAL_PROCESS_MAX_CELLS_V2 {
            return Err(invalid(
                "general_sem_conditional_process_v2.cell_cap",
                "conditional_process",
                "path × group × probe cells exceed the qualified cap of 512",
            ));
        }
        let target_estimate = conditional_process_target_upper_bound_v2(self, points, group_factor);
        let target_limit = if self.profile == ConditionalProcessProfileV2::MultiTwoWayStudentized {
            256
        } else {
            CONDITIONAL_PROCESS_MAX_TARGETS_V2
        };
        if target_estimate == 0 || target_estimate > target_limit {
            return Err(invalid(
                "general_sem_conditional_process_v2.target_cap",
                "conditional_process",
                format!(
                    "the compiled inferential target estimate must be between 1 and {target_limit}"
                ),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedTreatmentContrastV1 {
    Binary { control: f64, treated: f64 },
    Continuous { x0: f64, x1: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CausalIdentificationChecklistV1 {
    pub temporal_order_declared: bool,
    pub adjustment_set_justified: bool,
    pub consistency_assumption_acknowledged: bool,
    pub no_unmeasured_treatment_outcome_confounding_acknowledged: bool,
    pub no_unmeasured_treatment_mediator_confounding_acknowledged: bool,
    pub no_unmeasured_mediator_outcome_confounding_acknowledged: bool,
    pub no_exposure_induced_mediator_outcome_confounder_confirmed: bool,
    pub no_recanting_witness_confirmed: bool,
    pub linear_model_specification_reviewed: bool,
    pub positivity_reviewed: bool,
}

impl CausalIdentificationChecklistV1 {
    pub fn complete(&self) -> bool {
        self.temporal_order_declared
            && self.adjustment_set_justified
            && self.consistency_assumption_acknowledged
            && self.no_unmeasured_treatment_outcome_confounding_acknowledged
            && self.no_unmeasured_treatment_mediator_confounding_acknowledged
            && self.no_unmeasured_mediator_outcome_confounding_acknowledged
            && self.no_exposure_induced_mediator_outcome_confounder_confirmed
            && self.no_recanting_witness_confirmed
            && self.linear_model_specification_reviewed
            && self.positivity_reviewed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CausalLinearTermV1 {
    pub term_id: String,
    pub factor_variable_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CausalLinearEquationV1 {
    pub equation_id: String,
    pub outcome_variable_id: String,
    pub terms: Vec<CausalLinearTermV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CausalPositivityPolicyV1 {
    pub minimum_binary_arm_count: u32,
    pub maximum_binary_arm_ratio: f64,
    pub positivity_strata_variable_ids: Vec<String>,
    pub minimum_count_per_binary_stratum_arm: u32,
    pub continuous_neighborhood_fraction_of_range: f64,
    pub minimum_continuous_neighborhood_count: u32,
}

impl Default for CausalPositivityPolicyV1 {
    fn default() -> Self {
        Self {
            minimum_binary_arm_count: 10,
            maximum_binary_arm_ratio: 10.0,
            positivity_strata_variable_ids: Vec::new(),
            minimum_count_per_binary_stratum_arm: 1,
            continuous_neighborhood_fraction_of_range: 0.05,
            minimum_continuous_neighborhood_count: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservedCausalPathV1 {
    pub path_id: String,
    pub ordered_variable_ids: Vec<String>,
    /// Ordered mediator equations followed by the outcome equation. This is
    /// part of scientific authority: the runner must never infer or replace
    /// an observed-data causal model.
    pub equations: Vec<CausalLinearEquationV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterventionalCausalMediationConfigV1 {
    pub schema_version: u32,
    pub treatment: String,
    pub treatment_contrast: ObservedTreatmentContrastV1,
    pub outcome: String,
    pub mediators: Vec<String>,
    #[serde(default)]
    pub baseline_moderators: Vec<String>,
    #[serde(default)]
    pub adjustment_covariates: Vec<String>,
    pub paths: Vec<ObservedCausalPathV1>,
    pub positivity_policy: CausalPositivityPolicyV1,
    pub identification: CausalIdentificationChecklistV1,
    pub bootstrap_resamples: u32,
    pub seed: u64,
    pub confidence_level: f64,
}

impl InterventionalCausalMediationConfigV1 {
    pub fn ensure_valid(&self) -> Result<(), MultiModConfigErrorV1> {
        if self.schema_version != INTERVENTIONAL_CAUSAL_MEDIATION_V1_SCHEMA_VERSION {
            return Err(invalid(
                "interventional_causal_mediation_v1.schema",
                "interventional_mediation.schema_version",
                "interventional causal mediation requires schema version 1",
            ));
        }
        stable_id(&self.treatment, "interventional_mediation.treatment")?;
        stable_id(&self.outcome, "interventional_mediation.outcome")?;
        if self.treatment == self.outcome {
            return Err(invalid(
                "interventional_causal_mediation_v1.endpoint",
                "interventional_mediation",
                "treatment and outcome must differ",
            ));
        }
        match self.treatment_contrast {
            ObservedTreatmentContrastV1::Binary { control, treated }
            | ObservedTreatmentContrastV1::Continuous {
                x0: control,
                x1: treated,
            } if control.is_finite() && treated.is_finite() && control != treated => {}
            _ => {
                return Err(invalid(
                    "interventional_causal_mediation_v1.treatment_contrast",
                    "interventional_mediation.treatment_contrast",
                    "treatment endpoints must be distinct finite values",
                ));
            }
        }
        if self.mediators.is_empty() {
            return Err(invalid(
                "interventional_causal_mediation_v1.mediators",
                "interventional_mediation.mediators",
                "declare at least one observed continuous mediator",
            ));
        }
        unique_stable_ids(
            self.mediators.iter().map(String::as_str),
            "interventional_mediation.mediators",
        )?;
        unique_stable_ids(
            self.baseline_moderators.iter().map(String::as_str),
            "interventional_mediation.baseline_moderators",
        )?;
        if self.adjustment_covariates.is_empty() {
            return Err(invalid(
                "interventional_causal_mediation_v1.adjustment_set_missing",
                "interventional_mediation.adjustment_covariates",
                "the first qualified causal profile requires an explicit nonempty adjustment set",
            ));
        }
        unique_stable_ids(
            self.adjustment_covariates.iter().map(String::as_str),
            "interventional_mediation.adjustment_covariates",
        )?;
        let mut roles = BTreeSet::from([self.treatment.as_str(), self.outcome.as_str()]);
        for (path, values) in [
            ("mediators", &self.mediators),
            ("baseline_moderators", &self.baseline_moderators),
            ("adjustment_covariates", &self.adjustment_covariates),
        ] {
            for value in values {
                if !roles.insert(value.as_str()) {
                    return Err(invalid(
                        "interventional_causal_mediation_v1.role_overlap",
                        format!("interventional_mediation.{path}"),
                        format!("observed variable {value} has more than one causal role"),
                    ));
                }
            }
        }
        if self.paths.is_empty() || self.paths.len() > 8 {
            return Err(invalid(
                "interventional_causal_mediation_v1.path_count",
                "interventional_mediation.paths",
                "declare between one and eight causal paths",
            ));
        }
        unique_stable_ids(
            self.paths.iter().map(|path| path.path_id.as_str()),
            "interventional_mediation.paths.path_id",
        )?;
        for (index, path) in self.paths.iter().enumerate() {
            let distinct_variables = path
                .ordered_variable_ids
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if !(3..=5).contains(&path.ordered_variable_ids.len())
                || path.ordered_variable_ids.first() != Some(&self.treatment)
                || path.ordered_variable_ids.last() != Some(&self.outcome)
                || distinct_variables.len() != path.ordered_variable_ids.len()
                || path
                    .ordered_variable_ids
                    .windows(2)
                    .any(|pair| pair[0] == pair[1])
                || path.ordered_variable_ids[1..path.ordered_variable_ids.len() - 1]
                    .iter()
                    .any(|value| !self.mediators.contains(value))
            {
                return Err(invalid(
                    "interventional_causal_mediation_v1.path",
                    format!("interventional_mediation.paths[{index}]"),
                    "paths must contain 2 through 4 directed edges from treatment through declared mediators to outcome",
                ));
            }
            let expected_outcomes = path.ordered_variable_ids[1..].to_vec();
            if path
                .equations
                .iter()
                .map(|equation| equation.outcome_variable_id.clone())
                .collect::<Vec<_>>()
                != expected_outcomes
            {
                return Err(invalid(
                    "interventional_causal_mediation_v1.equation_order",
                    format!("interventional_mediation.paths[{index}].equations"),
                    "each path requires one ordered equation per mediator followed by the outcome equation",
                ));
            }
            unique_stable_ids(
                path.equations
                    .iter()
                    .map(|equation| equation.equation_id.as_str()),
                &format!("interventional_mediation.paths[{index}].equations.equation_id"),
            )?;
            let baseline = self
                .adjustment_covariates
                .iter()
                .chain(&self.baseline_moderators)
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            let mediator_ids = self
                .mediators
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            for (equation_index, equation) in path.equations.iter().enumerate() {
                if equation.terms.is_empty() {
                    return Err(invalid(
                        "interventional_causal_mediation_v1.equation_terms",
                        format!(
                            "interventional_mediation.paths[{index}].equations[{equation_index}].terms"
                        ),
                        "each observed equation requires explicit non-intercept terms",
                    ));
                }
                unique_stable_ids(
                    equation.terms.iter().map(|term| term.term_id.as_str()),
                    &format!(
                        "interventional_mediation.paths[{index}].equations[{equation_index}].terms.term_id"
                    ),
                )?;
                let allowed_factors = baseline
                    .iter()
                    .copied()
                    .chain(
                        path.ordered_variable_ids[..=equation_index]
                            .iter()
                            .map(String::as_str),
                    )
                    .collect::<BTreeSet<_>>();
                let main_factors = equation
                    .terms
                    .iter()
                    .filter(|term| term.factor_variable_ids.len() == 1)
                    .map(|term| term.factor_variable_ids[0].as_str())
                    .collect::<BTreeSet<_>>();
                let mut factor_sets = BTreeSet::new();
                for (term_index, term) in equation.terms.iter().enumerate() {
                    let factors = term
                        .factor_variable_ids
                        .iter()
                        .map(String::as_str)
                        .collect::<BTreeSet<_>>();
                    let post_treatment_factors = factors
                        .iter()
                        .filter(|factor| mediator_ids.contains(*factor))
                        .count();
                    if term.factor_variable_ids.is_empty()
                        || term.factor_variable_ids.len() > 3
                        || factors.len() != term.factor_variable_ids.len()
                        || factors
                            .iter()
                            .any(|factor| !allowed_factors.contains(*factor))
                        || post_treatment_factors > 1
                        || !factor_sets.insert(factors.clone())
                        || (factors.len() > 1
                            && factors.iter().any(|factor| !main_factors.contains(*factor)))
                    {
                        return Err(invalid(
                            "interventional_causal_mediation_v1.equation_term",
                            format!(
                                "interventional_mediation.paths[{index}].equations[{equation_index}].terms[{term_index}]"
                            ),
                            "equation terms must be unique recursive products of one through three declared factors with strong-hierarchy main effects and at most one post-treatment mediator",
                        ));
                    }
                }
                let required_predecessor = path.ordered_variable_ids[equation_index].as_str();
                if !main_factors.contains(required_predecessor)
                    || self
                        .adjustment_covariates
                        .iter()
                        .any(|covariate| !main_factors.contains(covariate.as_str()))
                {
                    return Err(invalid(
                        "interventional_causal_mediation_v1.required_main_effect",
                        format!(
                            "interventional_mediation.paths[{index}].equations[{equation_index}].terms"
                        ),
                        "each equation requires its selected-path predecessor and every adjustment covariate as main effects",
                    ));
                }
            }
        }
        if !self.identification.complete() {
            return Err(invalid(
                "interventional_causal_mediation_v1.identification",
                "interventional_mediation.identification",
                "all identification and positivity declarations require explicit acknowledgement",
            ));
        }
        let positivity_baseline = self
            .adjustment_covariates
            .iter()
            .chain(&self.baseline_moderators)
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        unique_stable_ids(
            self.positivity_policy
                .positivity_strata_variable_ids
                .iter()
                .map(String::as_str),
            "interventional_mediation.positivity_policy.positivity_strata_variable_ids",
        )?;
        if self.positivity_policy.minimum_binary_arm_count == 0
            || !self.positivity_policy.maximum_binary_arm_ratio.is_finite()
            || !(1.0..=10.0).contains(&self.positivity_policy.maximum_binary_arm_ratio)
            || self
                .positivity_policy
                .positivity_strata_variable_ids
                .iter()
                .any(|variable| !positivity_baseline.contains(variable.as_str()))
            || self.positivity_policy.minimum_count_per_binary_stratum_arm == 0
            || !self
                .positivity_policy
                .continuous_neighborhood_fraction_of_range
                .is_finite()
            || self
                .positivity_policy
                .continuous_neighborhood_fraction_of_range
                <= 0.0
            || self
                .positivity_policy
                .continuous_neighborhood_fraction_of_range
                > 0.5
            || self.positivity_policy.minimum_continuous_neighborhood_count == 0
        {
            return Err(invalid(
                "interventional_causal_mediation_v1.positivity_policy",
                "interventional_mediation.positivity_policy",
                "the positivity screen requires positive finite bounded thresholds and baseline-only unique strata",
            ));
        }
        if !(HETEROGENEITY_MIN_BOOTSTRAP_V2..=HETEROGENEITY_MAX_BOOTSTRAP_V2)
            .contains(&self.bootstrap_resamples)
        {
            return Err(invalid(
                "interventional_causal_mediation_v1.bootstrap_resamples",
                "interventional_mediation.bootstrap_resamples",
                "bootstrap resamples must be between 500 and 10000",
            ));
        }
        finite_probability(
            self.confidence_level,
            "interventional_mediation.confidence_level",
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checklist() -> MicomConfiguralChecklistV1 {
        MicomConfiguralChecklistV1 {
            identical_indicators_and_coding: true,
            identical_data_treatment: true,
            identical_algorithm_settings: true,
            identical_model_specification: true,
            deterministic_sign_orientation_reviewed: true,
            analyst_review_confirmed: true,
        }
    }

    fn groups() -> Vec<SelectedGroupV1> {
        vec![
            SelectedGroupV1 {
                group_id: "a".into(),
                label: "A".into(),
                value: TypedGroupValueV1::Number { value: 1.0 },
            },
            SelectedGroupV1 {
                group_id: "b".into(),
                label: "B".into(),
                value: TypedGroupValueV1::Text { value: "1".into() },
            },
        ]
    }

    #[test]
    fn typed_group_values_do_not_collapse_across_types() {
        let config = MgaMultigroupV1 {
            schema_version: 1,
            profile: MgaModelProfileV1::GeneralSemPls,
            grouping_column: "group".into(),
            groups: groups(),
            comparison_plan: MgaComparisonPlanV1::AllPairs {
                heavy_run_confirmed: false,
            },
            procedures: vec![MgaProcedureV1::MicomPairwise],
            permutation_samples: 5_000,
            bootstrap_samples: 5_000,
            seed: 42,
            confidence_level: 0.95,
            alpha: 0.05,
            alternative: InferenceAlternativeV1::TwoSided,
            multiplicity: MultiplicityAdjustmentV1::Holm,
            configural_checklist: checklist(),
            weight: None,
            selected_parameter_ids: vec!["path:x:y".into()],
        };
        config.ensure_valid().unwrap();
        let round_trip: MgaMultigroupV1 =
            serde_json::from_value(serde_json::to_value(&config).unwrap()).unwrap();
        assert_eq!(round_trip, config);
    }

    #[test]
    fn published_pos_rejects_interactions() {
        let config = PlsUnobservedHeterogeneityConfigV2 {
            schema_version: 2,
            profile: HeterogeneityInteractionProfileV2::P2MultiTwoWay,
            phase: HeterogeneityPhaseV2::Discovery {
                candidate_k: vec![2, 3],
                algorithms: vec![HeterogeneityAlgorithmV2::PlsPosPublishedV2],
            },
            seed: 42,
            fimix: FimixSettingsV2::default(),
            pls_pos: PlsPosSettingsV2::default(),
            pos_common_metric: None,
            bootstrap: None,
        };
        assert_eq!(
            config.ensure_valid().unwrap_err().code,
            "pls_heterogeneity_v2.published_pos_interaction"
        );
    }

    #[test]
    fn fimix_likelihood_decrease_tolerance_is_frozen() {
        let mut config = PlsUnobservedHeterogeneityConfigV2 {
            schema_version: 2,
            profile: HeterogeneityInteractionProfileV2::P0Structural,
            phase: HeterogeneityPhaseV2::Discovery {
                candidate_k: vec![2],
                algorithms: vec![HeterogeneityAlgorithmV2::FimixPlsV2],
            },
            seed: 42,
            fimix: FimixSettingsV2::default(),
            pls_pos: PlsPosSettingsV2::default(),
            pos_common_metric: None,
            bootstrap: None,
        };
        config.ensure_valid().unwrap();
        assert_eq!(
            config.fimix.likelihood_decrease_tolerance.to_bits(),
            FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2.to_bits()
        );

        config.fimix.likelihood_decrease_tolerance = 2.0e-9;
        assert_eq!(
            config.ensure_valid().unwrap_err().code,
            "pls_heterogeneity_v2.fimix_settings"
        );
    }

    fn studentized_config() -> GeneralSemConditionalProcessConfigV2 {
        GeneralSemConditionalProcessConfigV2 {
            schema_version: 2,
            profile: ConditionalProcessProfileV2::MultiTwoWayStudentized,
            paths: vec![ConditionalProcessPathV2 {
                path_id: "p".into(),
                ordered_relation_ids: vec!["x_m".into(), "m_y".into()],
            }],
            declared_interaction_ids: vec!["xz_m".into()],
            three_way_interaction_id: None,
            hoc_ids: vec![],
            moderator_ids: vec!["z".into()],
            probes: vec![ConditionalModeratorProbeV2 {
                probe_id: "z-probe".into(),
                moderator_id: "z".into(),
                scale: ConditionalProbeScaleV2::StandardizedScore,
                values: vec![-1.0, 0.0, 1.0],
                raw_transformation_receipt: None,
                raw_fit_metric_receipts: vec![],
            }],
            explicit_joint_tuples: vec![],
            probe_contrasts: vec![],
            grouping_column: None,
            groups: vec![],
            group_contrasts: vec![],
            weight: None,
            estimands: ConditionalProcessEstimandsV2 {
                conditional_specific_indirect: true,
                conditional_total_indirect: false,
                conditional_total_effect: false,
                scalar_index_when_affine: true,
                local_first_derivatives: true,
                local_second_and_cross_derivatives: true,
                finite_probe_contrasts: false,
            },
            inference: ConditionalProcessInferenceV2 {
                interval: ConditionalProcessIntervalV2::Studentized,
                alternative: InferenceAlternativeV1::TwoSided,
                outer_resamples: 1_000,
                inner_resamples: 200,
                seed: 42,
                confidence_level: 0.95,
            },
        }
    }

    #[test]
    fn studentized_budget_fails_closed() {
        let mut config = studentized_config();
        config.inference.outer_resamples = 5_000;
        config.inference.inner_resamples = 1_000;
        assert_eq!(
            config.ensure_valid().unwrap_err().code,
            "general_sem_conditional_process_v2.studentized_budget"
        );
    }

    #[test]
    fn studentized_probe_and_target_caps_fail_closed() {
        let mut config = studentized_config();
        config.declared_interaction_ids = vec!["xz_m".into(), "xw_m".into(), "xq_m".into()];
        config.moderator_ids = vec!["z".into(), "w".into(), "q".into()];
        config.probes = config
            .moderator_ids
            .iter()
            .map(|moderator| ConditionalModeratorProbeV2 {
                probe_id: format!("{moderator}-probe"),
                moderator_id: moderator.clone(),
                scale: ConditionalProbeScaleV2::StandardizedScore,
                values: vec![-1.0, 0.0, 1.0, 2.0],
                raw_transformation_receipt: None,
                raw_fit_metric_receipts: vec![],
            })
            .collect();
        assert_eq!(
            config.ensure_valid().unwrap_err().code,
            "general_sem_conditional_process_v2.studentized_probe_cap"
        );

        for probe in &mut config.probes {
            probe.values.pop();
        }
        config.estimands.conditional_total_indirect = true;
        config.estimands.conditional_total_effect = true;
        assert_eq!(
            config.ensure_valid().unwrap_err().code,
            "general_sem_conditional_process_v2.target_cap"
        );
    }

    #[test]
    fn multiple_hoc_profile_keeps_the_general_long_path_envelope() {
        let mut config = studentized_config();
        config.profile = ConditionalProcessProfileV2::MultipleHocPercentile;
        config.inference.interval = ConditionalProcessIntervalV2::Percentile;
        config.inference.inner_resamples = 0;
        config.hoc_ids = vec!["hoc-a".into(), "hoc-b".into()];
        config.declared_interaction_ids = vec!["xz-m".into(), "wz-y".into()];
        config.paths = (0..8)
            .map(|index| ConditionalProcessPathV2 {
                path_id: format!("path-{index}"),
                ordered_relation_ids: (0..6)
                    .map(|edge| format!("path-{index}-edge-{edge}"))
                    .collect(),
            })
            .collect();

        config
            .ensure_valid()
            .expect("multiple-HOC inherits the 1-8 path, 2-6 edge envelope");

        config.paths.push(ConditionalProcessPathV2 {
            path_id: "path-8".into(),
            ordered_relation_ids: vec!["path-8-edge-0".into(), "path-8-edge-1".into()],
        });
        assert_eq!(
            config.ensure_valid().unwrap_err().code,
            "general_sem_conditional_process_v2.path_count"
        );
    }

    #[test]
    fn raw_probe_requires_a_bound_numerical_receipt_and_transforms_deterministically() {
        let mut config = studentized_config();
        config.profile = ConditionalProcessProfileV2::MultiTwoWayPercentile;
        config.inference.interval = ConditionalProcessIntervalV2::Percentile;
        config.inference.inner_resamples = 0;
        config.probes[0].scale = ConditionalProbeScaleV2::RawObservedWithTransformationReceipt;
        assert_eq!(
            config.ensure_valid().unwrap_err().code,
            "general_sem_conditional_process_v2.raw_probe_receipt_missing"
        );

        let receipt = ConditionalRawProbeTransformationReceiptV2 {
            contract: CONDITIONAL_RAW_PROBE_TRANSFORMATION_CONTRACT_V2.into(),
            moderator_id: "z".into(),
            source_column: "z_raw".into(),
            dataset_fingerprint: "dataset:fingerprint".into(),
            analysis_row_mask_sha256: "a".repeat(64),
            center: 10.0,
            sample_standard_deviation: 2.0,
            orientation_sign: -1,
        };
        assert_eq!(receipt.standardize(12.0).unwrap(), -1.0);
        config.probes[0].raw_transformation_receipt = Some(receipt);
        config.ensure_valid().unwrap();

        config.probes[0]
            .raw_transformation_receipt
            .as_mut()
            .unwrap()
            .analysis_row_mask_sha256 = "A".repeat(64);
        assert_eq!(
            config.ensure_valid().unwrap_err().code,
            "general_sem_conditional_process_v2.raw_probe_row_mask"
        );
    }

    fn fit_metric_receipt(
        scope: ConditionalRawProbeFitScopeV2,
        basis: ConditionalRawProbeMetricBasisV2,
        weight_column: Option<&str>,
    ) -> ConditionalRawProbeFitMetricReceiptV2 {
        let (mass_sum, mass_squared_sum, effective_df, frequency_total) = match basis {
            ConditionalRawProbeMetricBasisV2::UnweightedSample => (4.0, 4.0, 3.0, None),
            ConditionalRawProbeMetricBasisV2::CaseWeightedEffectiveDf => (10.0, 30.0, 7.0, None),
            ConditionalRawProbeMetricBasisV2::FrequencyExpandedSample => {
                (10.0, 30.0, 9.0, Some(10))
            }
        };
        ConditionalRawProbeFitMetricReceiptV2 {
            contract: CONDITIONAL_RAW_PROBE_FIT_METRIC_CONTRACT_V2.into(),
            moderator_id: "z".into(),
            source_column: "z_raw".into(),
            dataset_fingerprint: "dataset:fingerprint".into(),
            analysis_row_mask_sha256: "a".repeat(64),
            fit_scope: scope,
            metric_basis: basis,
            weight_column: weight_column.map(str::to_owned),
            row_mass_sha256: "b".repeat(64),
            compact_row_count: 4,
            mass_sum,
            mass_squared_sum,
            effective_degrees_of_freedom: effective_df,
            frequency_total,
            center: 10.0,
            standard_deviation: 2.0,
            orientation_sign: 1,
        }
    }

    #[test]
    fn grouped_raw_probe_requires_one_unweighted_fit_metric_per_group() {
        let mut config = studentized_config();
        config.profile = ConditionalProcessProfileV2::GroupedPercentile;
        config.inference.interval = ConditionalProcessIntervalV2::Percentile;
        config.inference.inner_resamples = 0;
        config.grouping_column = Some("group".into());
        config.groups = groups();
        config.probes[0].scale = ConditionalProbeScaleV2::RawObservedWithTransformationReceipt;
        config.probes[0].raw_fit_metric_receipts = config
            .groups
            .iter()
            .map(|group| {
                fit_metric_receipt(
                    ConditionalRawProbeFitScopeV2::GroupFit {
                        group_id: group.group_id.clone(),
                    },
                    ConditionalRawProbeMetricBasisV2::UnweightedSample,
                    None,
                )
            })
            .collect();
        config.ensure_valid().unwrap();

        config.probes[0].raw_fit_metric_receipts.pop();
        assert_eq!(
            config.ensure_valid().unwrap_err().code,
            "general_sem_conditional_process_v2.raw_probe_scope_profile"
        );
    }

    #[test]
    fn fit_metric_denominators_distinguish_case_weights_from_frequency_expansion() {
        let case = fit_metric_receipt(
            ConditionalRawProbeFitScopeV2::AnalysisFit,
            ConditionalRawProbeMetricBasisV2::CaseWeightedEffectiveDf,
            Some("w"),
        );
        case.ensure_valid("case").unwrap();
        let frequency = fit_metric_receipt(
            ConditionalRawProbeFitScopeV2::AnalysisFit,
            ConditionalRawProbeMetricBasisV2::FrequencyExpandedSample,
            Some("f"),
        );
        frequency.ensure_valid("frequency").unwrap();
        assert_eq!(frequency.standardize(12.0).unwrap(), 1.0);

        let mut invalid = case;
        invalid.effective_degrees_of_freedom = 9.0;
        assert_eq!(
            invalid.ensure_valid("case").unwrap_err().code,
            "general_sem_conditional_process_v2.raw_probe_case_weight_metric"
        );
    }
}
