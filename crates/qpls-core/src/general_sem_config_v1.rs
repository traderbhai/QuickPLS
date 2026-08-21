use serde::{Deserialize, Deserializer, Serialize, de};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;

pub const GENERAL_SEM_CONFIG_V1_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_MAX_MATERIALIZED_SPECIFIC_PATHS_V1: u32 = 10_000;
pub const GENERAL_SEM_CASE_BOOTSTRAP_MIN_RESAMPLES_V1: u32 = 2;
pub const GENERAL_SEM_CASE_BOOTSTRAP_MAX_RESAMPLES_V1: u32 = 10_000;
/// Largest integer that round-trips losslessly through the JavaScript JSON wire.
pub const GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1: u64 = 9_007_199_254_740_991;

/// Scientific requests layered over a validated SEM model and its compiled topology.
///
/// Collection order is canonical: estimands and probes must be strictly ordered by
/// their stable request ids. Relation order inside a specific path is scientific
/// content and is therefore preserved exactly rather than sorted here.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemConfigV1 {
    pub schema_version: u32,
    pub requested_effect_estimands: Vec<GeneralSemEffectEstimandV1>,
    pub conditional_effect_probes: Vec<GeneralSemConditionalEffectProbeV1>,
    pub inference: GeneralSemInferenceV1,
    pub output_policy: GeneralSemOutputPolicyV1,
}

impl Default for GeneralSemConfigV1 {
    fn default() -> Self {
        Self {
            schema_version: GENERAL_SEM_CONFIG_V1_SCHEMA_VERSION,
            requested_effect_estimands: Vec::new(),
            conditional_effect_probes: Vec::new(),
            inference: GeneralSemInferenceV1::None,
            output_policy: GeneralSemOutputPolicyV1::default(),
        }
    }
}

impl GeneralSemConfigV1 {
    pub fn ensure_valid(&self) -> Result<(), GeneralSemConfigV1ValidationError> {
        if self.schema_version != GENERAL_SEM_CONFIG_V1_SCHEMA_VERSION {
            return Err(GeneralSemConfigV1ValidationError::SchemaVersion {
                found: self.schema_version,
            });
        }

        let mut request_ids = BTreeSet::new();
        let mut estimand_signatures = BTreeSet::new();
        let mut previous_estimand_id: Option<&str> = None;
        for (index, estimand) in self.requested_effect_estimands.iter().enumerate() {
            let estimand_id = estimand.estimand_id();
            validate_stable_id(
                format!("requested_effect_estimands[{index}].estimand_id"),
                estimand_id,
            )?;
            validate_unique_request_id(&mut request_ids, estimand_id)?;
            validate_canonical_request_order(
                "requested_effect_estimands",
                previous_estimand_id,
                estimand_id,
            )?;
            previous_estimand_id = Some(estimand_id);

            match estimand {
                GeneralSemEffectEstimandV1::SpecificPath {
                    ordered_relation_ids,
                    ..
                } => {
                    if ordered_relation_ids.len() < 2 {
                        return Err(
                            GeneralSemConfigV1ValidationError::SpecificIndirectPathTooShort {
                                estimand_id: estimand_id.to_string(),
                            },
                        );
                    }
                    let mut relation_ids = BTreeSet::new();
                    for (relation_index, relation_id) in ordered_relation_ids.iter().enumerate() {
                        validate_stable_id(
                            format!(
                                "requested_effect_estimands[{index}].ordered_relation_ids[{relation_index}]"
                            ),
                            relation_id,
                        )?;
                        if !relation_ids.insert(relation_id.as_str()) {
                            return Err(
                                GeneralSemConfigV1ValidationError::DuplicateSpecificPathRelation {
                                    estimand_id: estimand_id.to_string(),
                                    relation_id: relation_id.clone(),
                                },
                            );
                        }
                    }
                    let signature = format!("specific_path\0{}", ordered_relation_ids.join("\0"));
                    if !estimand_signatures.insert(signature) {
                        return Err(GeneralSemConfigV1ValidationError::DuplicateEffectEstimand {
                            estimand_id: estimand_id.to_string(),
                        });
                    }
                }
                GeneralSemEffectEstimandV1::TotalIndirect {
                    source_id,
                    target_id,
                    ..
                }
                | GeneralSemEffectEstimandV1::TotalEffect {
                    source_id,
                    target_id,
                    ..
                } => {
                    validate_stable_id(
                        format!("requested_effect_estimands[{index}].source_id"),
                        source_id,
                    )?;
                    validate_stable_id(
                        format!("requested_effect_estimands[{index}].target_id"),
                        target_id,
                    )?;
                    if source_id == target_id {
                        return Err(GeneralSemConfigV1ValidationError::EffectEndpointsEqual {
                            estimand_id: estimand_id.to_string(),
                        });
                    }
                    let kind = match estimand {
                        GeneralSemEffectEstimandV1::TotalIndirect { .. } => "total_indirect",
                        GeneralSemEffectEstimandV1::TotalEffect { .. } => "total_effect",
                        GeneralSemEffectEstimandV1::SpecificPath { .. } => unreachable!(),
                    };
                    if !estimand_signatures.insert(format!("{kind}\0{source_id}\0{target_id}")) {
                        return Err(GeneralSemConfigV1ValidationError::DuplicateEffectEstimand {
                            estimand_id: estimand_id.to_string(),
                        });
                    }
                }
            }
        }

        let mut previous_probe_id: Option<&str> = None;
        for (index, probe) in self.conditional_effect_probes.iter().enumerate() {
            validate_stable_id(
                format!("conditional_effect_probes[{index}].probe_id"),
                &probe.probe_id,
            )?;
            validate_unique_request_id(&mut request_ids, &probe.probe_id)?;
            validate_canonical_request_order(
                "conditional_effect_probes",
                previous_probe_id,
                &probe.probe_id,
            )?;
            previous_probe_id = Some(&probe.probe_id);
            validate_stable_id(
                format!("conditional_effect_probes[{index}].moderator_id"),
                &probe.moderator_id,
            )?;

            if let GeneralSemConditionalProbeValuesV1::Explicit { values } = &probe.values {
                if values.is_empty() {
                    return Err(
                        GeneralSemConfigV1ValidationError::EmptyExplicitProbeValues {
                            probe_id: probe.probe_id.clone(),
                        },
                    );
                }
                for (value_index, value) in values.iter().enumerate() {
                    if !value.is_finite() {
                        return Err(
                            GeneralSemConfigV1ValidationError::NonFiniteExplicitProbeValue {
                                probe_id: probe.probe_id.clone(),
                                value_index,
                            },
                        );
                    }
                }
                for (value_index, pair) in values.windows(2).enumerate() {
                    if pair[0].partial_cmp(&pair[1]) != Some(Ordering::Less) {
                        return Err(
                            GeneralSemConfigV1ValidationError::NonCanonicalExplicitProbeValueOrder {
                                probe_id: probe.probe_id.clone(),
                                left_index: value_index,
                                right_index: value_index + 1,
                            },
                        );
                    }
                }
            }
        }

        if let GeneralSemInferenceV1::CaseBootstrap {
            resamples,
            seed,
            confidence_level,
            ..
        } = self.inference
        {
            if !(GENERAL_SEM_CASE_BOOTSTRAP_MIN_RESAMPLES_V1
                ..=GENERAL_SEM_CASE_BOOTSTRAP_MAX_RESAMPLES_V1)
                .contains(&resamples)
            {
                return Err(
                    GeneralSemConfigV1ValidationError::BootstrapResamplesOutOfRange {
                        found: resamples,
                    },
                );
            }
            if seed > GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1 {
                return Err(GeneralSemConfigV1ValidationError::BootstrapSeedOutOfRange {
                    found: seed,
                });
            }
            if !confidence_level.is_finite() || confidence_level <= 0.0 || confidence_level >= 1.0 {
                return Err(GeneralSemConfigV1ValidationError::InvalidConfidenceLevel);
            }
        }

        self.output_policy.ensure_valid()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneralSemConfigWireV1 {
    schema_version: u32,
    requested_effect_estimands: Vec<GeneralSemEffectEstimandV1>,
    conditional_effect_probes: Vec<GeneralSemConditionalEffectProbeV1>,
    inference: GeneralSemInferenceV1,
    output_policy: GeneralSemOutputPolicyV1,
}

impl<'de> Deserialize<'de> for GeneralSemConfigV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = GeneralSemConfigWireV1::deserialize(deserializer)?;
        let config = Self {
            schema_version: wire.schema_version,
            requested_effect_estimands: wire.requested_effect_estimands,
            conditional_effect_probes: wire.conditional_effect_probes,
            inference: wire.inference,
            output_policy: wire.output_policy,
        };
        config.ensure_valid().map_err(de::Error::custom)?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemEffectEstimandV1 {
    /// Product of the coefficients identified by these ordered stable relation ids.
    ///
    /// This request layer can validate identity and order, but it cannot prove that
    /// adjacent relations form a continuous directed path. The topology compiler
    /// must match the complete ordered sequence against the validated model before
    /// estimation; it must reject rather than reinterpret a discontinuous path.
    SpecificPath {
        estimand_id: String,
        ordered_relation_ids: Vec<String>,
    },
    TotalIndirect {
        estimand_id: String,
        source_id: String,
        target_id: String,
    },
    TotalEffect {
        estimand_id: String,
        source_id: String,
        target_id: String,
    },
}

impl GeneralSemEffectEstimandV1 {
    pub fn estimand_id(&self) -> &str {
        match self {
            Self::SpecificPath { estimand_id, .. }
            | Self::TotalIndirect { estimand_id, .. }
            | Self::TotalEffect { estimand_id, .. } => estimand_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemConditionalEffectProbeV1 {
    pub probe_id: String,
    pub moderator_id: String,
    pub values: GeneralSemConditionalProbeValuesV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemConditionalProbeValuesV1 {
    /// Resolve three values from the exact estimation sample: mean - one SD,
    /// mean, and mean + one SD. The engine records its SD convention in result
    /// provenance; this request never substitutes authored constants.
    DataDerivedMeanPlusMinusOneSd,
    /// Evaluate the moderator at every authored value, in strictly increasing
    /// numeric order. All values must be finite.
    Explicit { values: Vec<f64> },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemInferenceV1 {
    None,
    CaseBootstrap {
        resamples: u32,
        seed: u64,
        confidence_level: f64,
        interval: GeneralSemBootstrapIntervalV1,
        tail: GeneralSemInferenceTailV1,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemBootstrapIntervalV1 {
    Percentile,
    Bca,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemInferenceTailV1 {
    TwoSided,
    OneSidedLower,
    OneSidedUpper,
}

/// Bounded output behavior for potentially exponential specific-path sets.
///
/// There is deliberately no truncation variant. Exceeding the materialization
/// limit either returns a typed error or a lazy result that retains the complete
/// requested set.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemOutputPolicyV1 {
    pub max_materialized_specific_paths: u32,
    pub lazy_specific_path_materialization: bool,
    pub when_specific_path_limit_exceeded: GeneralSemSpecificPathLimitBehaviorV1,
}

impl Default for GeneralSemOutputPolicyV1 {
    fn default() -> Self {
        Self {
            max_materialized_specific_paths: DEFAULT_MAX_MATERIALIZED_SPECIFIC_PATHS_V1,
            lazy_specific_path_materialization: false,
            when_specific_path_limit_exceeded: GeneralSemSpecificPathLimitBehaviorV1::Error,
        }
    }
}

impl GeneralSemOutputPolicyV1 {
    fn ensure_valid(&self) -> Result<(), GeneralSemConfigV1ValidationError> {
        if self.max_materialized_specific_paths == 0 {
            return Err(GeneralSemConfigV1ValidationError::ZeroMaxMaterializedSpecificPaths);
        }
        if self.when_specific_path_limit_exceeded
            == GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy
            && !self.lazy_specific_path_materialization
        {
            return Err(
                GeneralSemConfigV1ValidationError::LazyLimitBehaviorRequiresLazyMaterialization,
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemSpecificPathLimitBehaviorV1 {
    Error,
    ReturnLazy,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum GeneralSemConfigV1ValidationError {
    #[error("general SEM config v1 requires schema_version 1 (found {found})")]
    SchemaVersion { found: u32 },
    #[error("stable id at {context} cannot be empty")]
    EmptyStableId { context: String },
    #[error("stable id at {context} cannot contain surrounding whitespace")]
    StableIdHasSurroundingWhitespace { context: String },
    #[error("stable id at {context} cannot contain control characters")]
    StableIdContainsControlCharacter { context: String },
    #[error("stable id at {context} must use Unicode NFC normalization")]
    StableIdNotNfc { context: String },
    #[error("request id {request_id} is duplicated")]
    DuplicateRequestId { request_id: String },
    #[error("{collection} must be in strict stable-id order ({current_id} follows {previous_id})")]
    NonCanonicalRequestOrder {
        collection: &'static str,
        previous_id: String,
        current_id: String,
    },
    #[error("specific indirect-path estimand {estimand_id} requires at least two relation ids")]
    SpecificIndirectPathTooShort { estimand_id: String },
    #[error("specific indirect-path estimand {estimand_id} repeats relation id {relation_id}")]
    DuplicateSpecificPathRelation {
        estimand_id: String,
        relation_id: String,
    },
    #[error("effect estimand {estimand_id} duplicates another scientific request")]
    DuplicateEffectEstimand { estimand_id: String },
    #[error("effect estimand {estimand_id} requires distinct source and target ids")]
    EffectEndpointsEqual { estimand_id: String },
    #[error("conditional-effect probe {probe_id} requires at least one explicit value")]
    EmptyExplicitProbeValues { probe_id: String },
    #[error(
        "conditional-effect probe {probe_id} contains a non-finite value at index {value_index}"
    )]
    NonFiniteExplicitProbeValue {
        probe_id: String,
        value_index: usize,
    },
    #[error(
        "conditional-effect probe {probe_id} values must be strictly increasing (indices {left_index} and {right_index})"
    )]
    NonCanonicalExplicitProbeValueOrder {
        probe_id: String,
        left_index: usize,
        right_index: usize,
    },
    #[error(
        "case-bootstrap inference requires {GENERAL_SEM_CASE_BOOTSTRAP_MIN_RESAMPLES_V1}..={GENERAL_SEM_CASE_BOOTSTRAP_MAX_RESAMPLES_V1} resamples (found {found})"
    )]
    BootstrapResamplesOutOfRange { found: u32 },
    #[error(
        "case-bootstrap seed must be at most {GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1} so the JSON wire preserves it exactly (found {found})"
    )]
    BootstrapSeedOutOfRange { found: u64 },
    #[error("case-bootstrap confidence_level must be finite and strictly between 0 and 1")]
    InvalidConfidenceLevel,
    #[error("max_materialized_specific_paths must be greater than zero")]
    ZeroMaxMaterializedSpecificPaths,
    #[error("return_lazy limit behavior requires lazy_specific_path_materialization=true")]
    LazyLimitBehaviorRequiresLazyMaterialization,
}

fn validate_unique_request_id(
    request_ids: &mut BTreeSet<String>,
    request_id: &str,
) -> Result<(), GeneralSemConfigV1ValidationError> {
    if request_ids.insert(request_id.to_string()) {
        Ok(())
    } else {
        Err(GeneralSemConfigV1ValidationError::DuplicateRequestId {
            request_id: request_id.to_string(),
        })
    }
}

fn validate_canonical_request_order(
    collection: &'static str,
    previous_id: Option<&str>,
    current_id: &str,
) -> Result<(), GeneralSemConfigV1ValidationError> {
    if let Some(previous_id) = previous_id
        && previous_id >= current_id
    {
        return Err(
            GeneralSemConfigV1ValidationError::NonCanonicalRequestOrder {
                collection,
                previous_id: previous_id.to_string(),
                current_id: current_id.to_string(),
            },
        );
    }
    Ok(())
}

fn validate_stable_id(
    context: String,
    value: &str,
) -> Result<(), GeneralSemConfigV1ValidationError> {
    if value.trim().is_empty() {
        return Err(GeneralSemConfigV1ValidationError::EmptyStableId { context });
    }
    if value.trim() != value {
        return Err(
            GeneralSemConfigV1ValidationError::StableIdHasSurroundingWhitespace { context },
        );
    }
    if value.chars().any(char::is_control) {
        return Err(
            GeneralSemConfigV1ValidationError::StableIdContainsControlCharacter { context },
        );
    }
    if value.nfc().collect::<String>() != value {
        return Err(GeneralSemConfigV1ValidationError::StableIdNotNfc { context });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn comprehensive_config() -> GeneralSemConfigV1 {
        GeneralSemConfigV1 {
            schema_version: GENERAL_SEM_CONFIG_V1_SCHEMA_VERSION,
            requested_effect_estimands: vec![
                GeneralSemEffectEstimandV1::SpecificPath {
                    estimand_id: "effect:01:specific".into(),
                    // Deliberately not lexical: relation order is path semantics.
                    ordered_relation_ids: vec!["relation:z".into(), "relation:a".into()],
                },
                GeneralSemEffectEstimandV1::TotalIndirect {
                    estimand_id: "effect:02:total_indirect".into(),
                    source_id: "construct:x".into(),
                    target_id: "construct:y".into(),
                },
                GeneralSemEffectEstimandV1::TotalEffect {
                    estimand_id: "effect:03:total".into(),
                    source_id: "construct:x".into(),
                    target_id: "construct:y".into(),
                },
            ],
            conditional_effect_probes: vec![
                GeneralSemConditionalEffectProbeV1 {
                    probe_id: "probe:01:explicit".into(),
                    moderator_id: "construct:moderator_a".into(),
                    values: GeneralSemConditionalProbeValuesV1::Explicit {
                        values: vec![-1.5, 0.0, 2.25],
                    },
                },
                GeneralSemConditionalEffectProbeV1 {
                    probe_id: "probe:02:data".into(),
                    moderator_id: "construct:moderator_b".into(),
                    values: GeneralSemConditionalProbeValuesV1::DataDerivedMeanPlusMinusOneSd,
                },
            ],
            inference: GeneralSemInferenceV1::CaseBootstrap {
                resamples: 5_000,
                seed: 2_026_081_800,
                confidence_level: 0.95,
                interval: GeneralSemBootstrapIntervalV1::Bca,
                tail: GeneralSemInferenceTailV1::OneSidedUpper,
            },
            output_policy: GeneralSemOutputPolicyV1 {
                max_materialized_specific_paths: 2_048,
                lazy_specific_path_materialization: true,
                when_specific_path_limit_exceeded:
                    GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy,
            },
        }
    }

    #[test]
    fn default_is_valid_fail_closed_and_round_trips() {
        let config = GeneralSemConfigV1::default();
        config.ensure_valid().unwrap();
        assert_eq!(config.schema_version, 1);
        assert!(config.requested_effect_estimands.is_empty());
        assert!(config.conditional_effect_probes.is_empty());
        assert_eq!(config.inference, GeneralSemInferenceV1::None);
        assert_eq!(
            config.output_policy,
            GeneralSemOutputPolicyV1 {
                max_materialized_specific_paths: 10_000,
                lazy_specific_path_materialization: false,
                when_specific_path_limit_exceeded: GeneralSemSpecificPathLimitBehaviorV1::Error,
            }
        );

        let encoded = serde_json::to_string(&config).unwrap();
        let decoded: GeneralSemConfigV1 = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, config);
    }

    #[test]
    fn comprehensive_contract_round_trips_canonically_without_reordering_paths() {
        let config = comprehensive_config();
        config.ensure_valid().unwrap();

        let first = serde_json::to_vec(&config).unwrap();
        let decoded: GeneralSemConfigV1 = serde_json::from_slice(&first).unwrap();
        let second = serde_json::to_vec(&decoded).unwrap();
        assert_eq!(decoded, config);
        assert_eq!(second, first);

        let GeneralSemEffectEstimandV1::SpecificPath {
            ordered_relation_ids,
            ..
        } = &decoded.requested_effect_estimands[0]
        else {
            panic!("expected the specific-path fixture");
        };
        assert_eq!(
            ordered_relation_ids,
            &vec!["relation:z".to_string(), "relation:a".to_string()]
        );
    }

    #[test]
    fn serde_rejects_unknown_fields_and_unknown_limit_behaviors() {
        let mut top_level = serde_json::to_value(comprehensive_config()).unwrap();
        top_level["unexpected"] = json!(true);
        assert!(
            serde_json::from_value::<GeneralSemConfigV1>(top_level)
                .unwrap_err()
                .to_string()
                .contains("unknown field")
        );

        let mut nested = serde_json::to_value(comprehensive_config()).unwrap();
        nested["inference"]["unexpected"] = json!(true);
        assert!(
            serde_json::from_value::<GeneralSemConfigV1>(nested)
                .unwrap_err()
                .to_string()
                .contains("unknown field")
        );

        let mut behavior = serde_json::to_value(comprehensive_config()).unwrap();
        behavior["output_policy"]["when_specific_path_limit_exceeded"] = json!("truncate");
        assert!(
            serde_json::from_value::<GeneralSemConfigV1>(behavior)
                .unwrap_err()
                .to_string()
                .contains("unknown variant")
        );
    }

    #[test]
    fn schema_and_request_ids_are_strict_and_canonical() {
        let mut wrong_schema = comprehensive_config();
        wrong_schema.schema_version = 2;
        assert_eq!(
            wrong_schema.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::SchemaVersion { found: 2 })
        );

        let mut duplicate = comprehensive_config();
        duplicate.conditional_effect_probes[0].probe_id = "effect:01:specific".into();
        assert_eq!(
            duplicate.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::DuplicateRequestId {
                request_id: "effect:01:specific".into(),
            })
        );

        let mut out_of_order = comprehensive_config();
        out_of_order.requested_effect_estimands.swap(0, 1);
        assert!(matches!(
            out_of_order.ensure_valid(),
            Err(
                GeneralSemConfigV1ValidationError::NonCanonicalRequestOrder {
                    collection: "requested_effect_estimands",
                    ..
                }
            )
        ));

        let mut padded = comprehensive_config();
        if let GeneralSemEffectEstimandV1::SpecificPath { estimand_id, .. } =
            &mut padded.requested_effect_estimands[0]
        {
            *estimand_id = " effect:01:specific".into();
        }
        assert!(matches!(
            padded.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::StableIdHasSurroundingWhitespace { .. })
        ));

        let mut non_nfc = comprehensive_config();
        non_nfc.conditional_effect_probes[0].moderator_id = "construct:e\u{301}".into();
        assert!(matches!(
            non_nfc.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::StableIdNotNfc { .. })
        ));
    }

    #[test]
    fn specific_indirect_paths_require_two_unique_stable_relation_ids() {
        let mut empty_path = comprehensive_config();
        if let GeneralSemEffectEstimandV1::SpecificPath {
            ordered_relation_ids,
            ..
        } = &mut empty_path.requested_effect_estimands[0]
        {
            ordered_relation_ids.clear();
        }
        assert_eq!(
            empty_path.ensure_valid(),
            Err(
                GeneralSemConfigV1ValidationError::SpecificIndirectPathTooShort {
                    estimand_id: "effect:01:specific".into(),
                }
            )
        );

        let mut empty_relation = comprehensive_config();
        if let GeneralSemEffectEstimandV1::SpecificPath {
            ordered_relation_ids,
            ..
        } = &mut empty_relation.requested_effect_estimands[0]
        {
            ordered_relation_ids[1].clear();
        }
        assert!(matches!(
            empty_relation.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::EmptyStableId { context })
                if context.ends_with("ordered_relation_ids[1]")
        ));

        let mut single_relation = comprehensive_config();
        if let GeneralSemEffectEstimandV1::SpecificPath {
            ordered_relation_ids,
            ..
        } = &mut single_relation.requested_effect_estimands[0]
        {
            ordered_relation_ids.truncate(1);
        }
        assert!(matches!(
            single_relation.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::SpecificIndirectPathTooShort { .. })
        ));

        let mut repeated_relation = comprehensive_config();
        if let GeneralSemEffectEstimandV1::SpecificPath {
            ordered_relation_ids,
            ..
        } = &mut repeated_relation.requested_effect_estimands[0]
        {
            ordered_relation_ids[1] = ordered_relation_ids[0].clone();
        }
        assert!(matches!(
            repeated_relation.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::DuplicateSpecificPathRelation { .. })
        ));
    }

    #[test]
    fn duplicate_scientific_estimands_and_equal_endpoints_are_rejected() {
        let mut duplicate = comprehensive_config();
        duplicate.requested_effect_estimands.insert(
            1,
            GeneralSemEffectEstimandV1::SpecificPath {
                estimand_id: "effect:01b:specific_duplicate".into(),
                ordered_relation_ids: vec!["relation:z".into(), "relation:a".into()],
            },
        );
        assert!(matches!(
            duplicate.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::DuplicateEffectEstimand { .. })
        ));

        let mut equal_endpoints = comprehensive_config();
        let GeneralSemEffectEstimandV1::TotalIndirect { target_id, .. } =
            &mut equal_endpoints.requested_effect_estimands[1]
        else {
            unreachable!()
        };
        *target_id = "construct:x".into();
        assert!(matches!(
            equal_endpoints.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::EffectEndpointsEqual { .. })
        ));
    }

    #[test]
    fn explicit_probe_values_are_finite_nonempty_unique_and_sorted() {
        for values in [vec![], vec![0.0, 0.0], vec![1.0, -1.0]] {
            let mut config = comprehensive_config();
            config.conditional_effect_probes[0].values =
                GeneralSemConditionalProbeValuesV1::Explicit { values };
            assert!(config.ensure_valid().is_err());
        }

        let mut non_finite = comprehensive_config();
        non_finite.conditional_effect_probes[0].values =
            GeneralSemConditionalProbeValuesV1::Explicit {
                values: vec![0.0, f64::INFINITY],
            };
        assert_eq!(
            non_finite.ensure_valid(),
            Err(
                GeneralSemConfigV1ValidationError::NonFiniteExplicitProbeValue {
                    probe_id: "probe:01:explicit".into(),
                    value_index: 1,
                }
            )
        );

        let mut signed_zero_duplicate = comprehensive_config();
        signed_zero_duplicate.conditional_effect_probes[0].values =
            GeneralSemConditionalProbeValuesV1::Explicit {
                values: vec![-0.0, 0.0],
            };
        assert!(matches!(
            signed_zero_duplicate.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::NonCanonicalExplicitProbeValueOrder { .. })
        ));
    }

    #[test]
    fn bootstrap_boundaries_and_all_interval_tail_modes_are_explicit() {
        let intervals = [
            GeneralSemBootstrapIntervalV1::Percentile,
            GeneralSemBootstrapIntervalV1::Bca,
        ];
        let tails = [
            GeneralSemInferenceTailV1::TwoSided,
            GeneralSemInferenceTailV1::OneSidedLower,
            GeneralSemInferenceTailV1::OneSidedUpper,
        ];
        for interval in intervals {
            for tail in tails {
                let mut config = comprehensive_config();
                config.inference = GeneralSemInferenceV1::CaseBootstrap {
                    resamples: GENERAL_SEM_CASE_BOOTSTRAP_MIN_RESAMPLES_V1,
                    seed: 0,
                    confidence_level: 0.95,
                    interval,
                    tail,
                };
                config.ensure_valid().unwrap();
                let round_trip: GeneralSemConfigV1 =
                    serde_json::from_value(serde_json::to_value(&config).unwrap()).unwrap();
                assert_eq!(round_trip.inference, config.inference);
            }
        }

        for invalid_resamples in [0, 1, GENERAL_SEM_CASE_BOOTSTRAP_MAX_RESAMPLES_V1 + 1] {
            let mut invalid = comprehensive_config();
            invalid.inference = GeneralSemInferenceV1::CaseBootstrap {
                resamples: invalid_resamples,
                seed: 9,
                confidence_level: 0.95,
                interval: GeneralSemBootstrapIntervalV1::Percentile,
                tail: GeneralSemInferenceTailV1::TwoSided,
            };
            assert_eq!(
                invalid.ensure_valid(),
                Err(
                    GeneralSemConfigV1ValidationError::BootstrapResamplesOutOfRange {
                        found: invalid_resamples,
                    }
                )
            );
        }

        for valid_resamples in [
            GENERAL_SEM_CASE_BOOTSTRAP_MIN_RESAMPLES_V1,
            GENERAL_SEM_CASE_BOOTSTRAP_MAX_RESAMPLES_V1,
        ] {
            let mut valid = comprehensive_config();
            if let GeneralSemInferenceV1::CaseBootstrap { resamples, .. } = &mut valid.inference {
                *resamples = valid_resamples;
            }
            valid.ensure_valid().unwrap();
        }

        let mut max_seed = comprehensive_config();
        if let GeneralSemInferenceV1::CaseBootstrap { seed, .. } = &mut max_seed.inference {
            *seed = GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1;
        }
        max_seed.ensure_valid().unwrap();

        let mut unsafe_seed = max_seed;
        if let GeneralSemInferenceV1::CaseBootstrap { seed, .. } = &mut unsafe_seed.inference {
            *seed = GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1 + 1;
        }
        assert_eq!(
            unsafe_seed.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::BootstrapSeedOutOfRange {
                found: GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1 + 1,
            })
        );

        for confidence_level in [f64::NAN, f64::NEG_INFINITY, 0.0, 1.0, 1.01] {
            let mut config = comprehensive_config();
            if let GeneralSemInferenceV1::CaseBootstrap {
                confidence_level: configured,
                ..
            } = &mut config.inference
            {
                *configured = confidence_level;
            }
            assert_eq!(
                config.ensure_valid(),
                Err(GeneralSemConfigV1ValidationError::InvalidConfidenceLevel)
            );
        }
    }

    #[test]
    fn deserialization_runs_typed_validation_instead_of_accepting_invalid_requests() {
        let invalid = json!({
            "schema_version": 1,
            "requested_effect_estimands": [],
            "conditional_effect_probes": [],
            "inference": {
                "kind": "case_bootstrap",
                "resamples": 0,
                "seed": 1,
                "confidence_level": 0.95,
                "interval": "percentile",
                "tail": "two_sided"
            },
            "output_policy": {
                "max_materialized_specific_paths": 1,
                "lazy_specific_path_materialization": false,
                "when_specific_path_limit_exceeded": "error"
            }
        });
        let error = serde_json::from_value::<GeneralSemConfigV1>(invalid).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("case-bootstrap inference requires 2..=10000 resamples")
        );
    }

    #[test]
    fn output_limit_is_positive_and_never_silently_truncates() {
        let mut zero_limit = comprehensive_config();
        zero_limit.output_policy.max_materialized_specific_paths = 0;
        assert_eq!(
            zero_limit.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::ZeroMaxMaterializedSpecificPaths)
        );

        let mut invalid_lazy = comprehensive_config();
        invalid_lazy
            .output_policy
            .lazy_specific_path_materialization = false;
        assert_eq!(
            invalid_lazy.ensure_valid(),
            Err(GeneralSemConfigV1ValidationError::LazyLimitBehaviorRequiresLazyMaterialization)
        );

        let mut fail_closed = comprehensive_config();
        fail_closed.output_policy.lazy_specific_path_materialization = false;
        fail_closed.output_policy.when_specific_path_limit_exceeded =
            GeneralSemSpecificPathLimitBehaviorV1::Error;
        fail_closed.ensure_valid().unwrap();

        let encoded: Value = serde_json::to_value(fail_closed).unwrap();
        assert_eq!(
            encoded["output_policy"]["when_specific_path_limit_exceeded"],
            "error"
        );
        assert_ne!(
            encoded["output_policy"]["when_specific_path_limit_exceeded"],
            "truncate"
        );
    }
}
