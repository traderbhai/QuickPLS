use crate::{
    ObservedRoleV4, ObservedScaleV4, SamplingWeightNormalizationV4, SemDataBindingV4, SemModelV4,
    SemModelV4ValidationError, SemVariableV4, SemWeightBindingV4,
};
use serde::{Deserialize, Deserializer, Serialize, de};
use std::{error::Error, fmt};

pub const WEIGHT_DECLARATION_CONTRACT_VERSION_V1: &str = "sem_weight_declaration_v1";

/// A weight declaration resolved through the authoritative SemModelV4 variable table.
///
/// This records authored semantics only. In particular, sampling normalization is
/// preserved but is not applied by this contract.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedWeightDeclarationV1 {
    contract_version: String,
    dataset_id: String,
    binding: ResolvedWeightBindingV1,
}

impl ResolvedWeightDeclarationV1 {
    pub fn contract_version(&self) -> &str {
        &self.contract_version
    }

    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    pub fn binding(&self) -> &ResolvedWeightBindingV1 {
        &self.binding
    }

    pub fn ensure_valid(&self) -> Result<(), WeightDeclarationContractErrorV1> {
        if self.contract_version != WEIGHT_DECLARATION_CONTRACT_VERSION_V1 {
            return Err(WeightDeclarationContractErrorV1::ContractVersion(
                self.contract_version.clone(),
            ));
        }
        if self.dataset_id.trim().is_empty() {
            return Err(WeightDeclarationContractErrorV1::EmptyDatasetId);
        }
        if self.binding.variable_id().trim().is_empty() {
            return Err(WeightDeclarationContractErrorV1::EmptyVariableId);
        }
        if self.binding.source_column().trim().is_empty() {
            return Err(WeightDeclarationContractErrorV1::EmptySourceColumn);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolvedWeightDeclarationWireV1 {
    contract_version: String,
    dataset_id: String,
    binding: ResolvedWeightBindingV1,
}

impl<'de> Deserialize<'de> for ResolvedWeightDeclarationV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ResolvedWeightDeclarationWireV1::deserialize(deserializer)?;
        let declaration = Self {
            contract_version: wire.contract_version,
            dataset_id: wire.dataset_id,
            binding: wire.binding,
        };
        declaration.ensure_valid().map_err(de::Error::custom)?;
        Ok(declaration)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResolvedWeightBindingV1 {
    Case {
        variable_id: String,
        source_column: String,
    },
    Frequency {
        variable_id: String,
        source_column: String,
    },
    Sampling {
        variable_id: String,
        source_column: String,
        normalization: SamplingWeightNormalizationV4,
    },
}

impl ResolvedWeightBindingV1 {
    pub fn variable_id(&self) -> &str {
        match self {
            Self::Case { variable_id, .. }
            | Self::Frequency { variable_id, .. }
            | Self::Sampling { variable_id, .. } => variable_id,
        }
    }

    pub fn source_column(&self) -> &str {
        match self {
            Self::Case { source_column, .. }
            | Self::Frequency { source_column, .. }
            | Self::Sampling { source_column, .. } => source_column,
        }
    }

    pub fn sampling_normalization(&self) -> Option<SamplingWeightNormalizationV4> {
        match self {
            Self::Sampling { normalization, .. } => Some(*normalization),
            Self::Case { .. } | Self::Frequency { .. } => None,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum WeightDeclarationContractErrorV1 {
    #[error("weight declaration contract version must equal sem_weight_declaration_v1 (found {0})")]
    ContractVersion(String),
    #[error("weight declaration dataset id cannot be empty")]
    EmptyDatasetId,
    #[error("weight declaration variable id cannot be empty")]
    EmptyVariableId,
    #[error("weight declaration source column cannot be empty")]
    EmptySourceColumn,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum WeightDeclarationResolutionErrorV1 {
    #[error(transparent)]
    InvalidModel(#[from] SemModelV4ValidationError),
    #[error(transparent)]
    InvalidDeclaration(#[from] WeightDeclarationContractErrorV1),
    #[error("weight declaration dataset id cannot be empty")]
    EmptyDatasetId,
    #[error(
        "weight variable {variable_id} is not an available continuous observed control variable"
    )]
    WeightVariableUnavailable { variable_id: String },
}

pub fn resolve_weight_declaration_v1(
    model: &SemModelV4,
) -> Result<Option<ResolvedWeightDeclarationV1>, WeightDeclarationResolutionErrorV1> {
    model.ensure_valid()?;
    match &model.data_binding {
        SemDataBindingV4::Raw {
            dataset_id,
            weight: Some(weight),
            ..
        } => resolve_weight_declaration_parts_v1(dataset_id, weight, &model.variables).map(Some),
        SemDataBindingV4::Raw { weight: None, .. }
        | SemDataBindingV4::Covariance { .. }
        | SemDataBindingV4::Correlation { .. } => Ok(None),
    }
}

pub(crate) fn resolve_weight_declaration_parts_v1(
    dataset_id: &str,
    weight: &SemWeightBindingV4,
    variables: &[SemVariableV4],
) -> Result<ResolvedWeightDeclarationV1, WeightDeclarationResolutionErrorV1> {
    if dataset_id.trim().is_empty() {
        return Err(WeightDeclarationResolutionErrorV1::EmptyDatasetId);
    }
    let variable_id = weight.variable();
    let source_column = variables.iter().find_map(|variable| match variable {
        SemVariableV4::Observed {
            id,
            source_column,
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            ..
        } if id == variable_id => Some(source_column.clone()),
        _ => None,
    });
    let Some(source_column) = source_column else {
        return Err(
            WeightDeclarationResolutionErrorV1::WeightVariableUnavailable {
                variable_id: variable_id.to_owned(),
            },
        );
    };
    let binding = match weight {
        SemWeightBindingV4::Case { .. } => ResolvedWeightBindingV1::Case {
            variable_id: variable_id.to_owned(),
            source_column,
        },
        SemWeightBindingV4::Frequency { .. } => ResolvedWeightBindingV1::Frequency {
            variable_id: variable_id.to_owned(),
            source_column,
        },
        SemWeightBindingV4::Sampling { normalization, .. } => ResolvedWeightBindingV1::Sampling {
            variable_id: variable_id.to_owned(),
            source_column,
            normalization: *normalization,
        },
    };
    let declaration = ResolvedWeightDeclarationV1 {
        contract_version: WEIGHT_DECLARATION_CONTRACT_VERSION_V1.into(),
        dataset_id: dataset_id.to_owned(),
        binding,
    };
    declaration.ensure_valid()?;
    Ok(declaration)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WeightCapabilityCodeV1 {
    CaseWeightUnsupported,
    FrequencyWeightUnsupported,
    SamplingWeightUnsupported,
    SamplingWeightNormalizationUnsupported,
    LegacyCaseWeightBindingAmbiguous,
}

impl WeightCapabilityCodeV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CaseWeightUnsupported => "case_weight_unsupported",
            Self::FrequencyWeightUnsupported => "frequency_weight_unsupported",
            Self::SamplingWeightUnsupported => "sampling_weight_unsupported",
            Self::SamplingWeightNormalizationUnsupported => {
                "sampling_weight_normalization_unsupported"
            }
            Self::LegacyCaseWeightBindingAmbiguous => "legacy_case_weight_binding_ambiguous",
        }
    }
}

impl fmt::Display for WeightCapabilityCodeV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WeightCapabilityTargetV1 {
    PlsPlanV2,
    CbsemMlV1,
    CbsemMlMeanReplacementV1,
    CbsemProductIndicatorPlanV1,
}

impl WeightCapabilityTargetV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PlsPlanV2 => "pls_plan_v2",
            Self::CbsemMlV1 => "cbsem_ml_v1",
            Self::CbsemMlMeanReplacementV1 => "cbsem_ml_mean_replacement_v1",
            Self::CbsemProductIndicatorPlanV1 => "cbsem_product_indicator_plan_v1",
        }
    }
}

impl fmt::Display for WeightCapabilityTargetV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WeightCapabilityIssueV1 {
    pub code: WeightCapabilityCodeV1,
    pub target: WeightCapabilityTargetV1,
    pub declaration: Option<ResolvedWeightDeclarationV1>,
    pub subject: String,
    pub corrective_action: String,
}

impl WeightCapabilityIssueV1 {
    pub fn unsupported(
        target: WeightCapabilityTargetV1,
        declaration: ResolvedWeightDeclarationV1,
    ) -> Self {
        let (code, corrective_action) = match declaration.binding() {
            ResolvedWeightBindingV1::Case { .. } => (
                WeightCapabilityCodeV1::CaseWeightUnsupported,
                "Remove the case-weight binding or choose an estimator that explicitly supports case weights; no executable plan was emitted.",
            ),
            ResolvedWeightBindingV1::Frequency { .. } => (
                WeightCapabilityCodeV1::FrequencyWeightUnsupported,
                "Remove the frequency-weight binding or choose an estimator that explicitly supports frequency weights; no executable plan was emitted.",
            ),
            ResolvedWeightBindingV1::Sampling { .. } => (
                WeightCapabilityCodeV1::SamplingWeightUnsupported,
                "Remove the sampling-weight binding or choose an estimator with explicit sampling-design support; no executable plan was emitted.",
            ),
        };
        Self {
            code,
            target,
            subject: declaration.binding().variable_id().to_owned(),
            declaration: Some(declaration),
            corrective_action: corrective_action.into(),
        }
    }

    pub fn sampling_normalization_unsupported(
        target: WeightCapabilityTargetV1,
        declaration: ResolvedWeightDeclarationV1,
    ) -> Result<Self, WeightCapabilityIssueContractErrorV1> {
        let issue = Self {
            code: WeightCapabilityCodeV1::SamplingWeightNormalizationUnsupported,
            target,
            subject: declaration.binding().variable_id().to_owned(),
            declaration: Some(declaration),
            corrective_action: "Choose a supported sampling-weight normalization or remove the sampling-weight binding; no executable plan was emitted.".into(),
        };
        issue.ensure_valid()?;
        Ok(issue)
    }

    pub fn legacy_case_weight_binding_ambiguous(
        target: WeightCapabilityTargetV1,
        legacy_case_weight_column: &str,
        declaration: Option<ResolvedWeightDeclarationV1>,
    ) -> Result<Self, WeightCapabilityIssueContractErrorV1> {
        if legacy_case_weight_column.is_empty() {
            return Err(WeightCapabilityIssueContractErrorV1::EmptySubject);
        }
        if declaration.as_ref().is_some_and(|declaration| {
            matches!(
                declaration.binding(),
                ResolvedWeightBindingV1::Case { source_column, .. }
                    if source_column == legacy_case_weight_column
            )
        }) {
            return Ok(Self::unsupported(target, declaration.unwrap()));
        }
        let issue = Self {
            code: WeightCapabilityCodeV1::LegacyCaseWeightBindingAmbiguous,
            target,
            declaration,
            subject: legacy_case_weight_column.to_owned(),
            corrective_action: format!(
                "Legacy settings.case_weight_column '{legacy_case_weight_column}' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted."
            ),
        };
        issue.ensure_valid()?;
        Ok(issue)
    }

    pub fn ensure_valid(&self) -> Result<(), WeightCapabilityIssueContractErrorV1> {
        if self.subject.is_empty() {
            return Err(WeightCapabilityIssueContractErrorV1::EmptySubject);
        }
        if let Some(declaration) = &self.declaration {
            declaration.ensure_valid()?;
        }
        let expected_action = match self.code {
            WeightCapabilityCodeV1::CaseWeightUnsupported => {
                self.ensure_resolved_kind(|binding| {
                    matches!(binding, ResolvedWeightBindingV1::Case { .. })
                })?;
                "Remove the case-weight binding or choose an estimator that explicitly supports case weights; no executable plan was emitted.".into()
            }
            WeightCapabilityCodeV1::FrequencyWeightUnsupported => {
                self.ensure_resolved_kind(|binding| {
                    matches!(binding, ResolvedWeightBindingV1::Frequency { .. })
                })?;
                "Remove the frequency-weight binding or choose an estimator that explicitly supports frequency weights; no executable plan was emitted.".into()
            }
            WeightCapabilityCodeV1::SamplingWeightUnsupported => {
                self.ensure_resolved_kind(|binding| {
                    matches!(binding, ResolvedWeightBindingV1::Sampling { .. })
                })?;
                "Remove the sampling-weight binding or choose an estimator with explicit sampling-design support; no executable plan was emitted.".into()
            }
            WeightCapabilityCodeV1::SamplingWeightNormalizationUnsupported => {
                self.ensure_resolved_kind(|binding| {
                    matches!(binding, ResolvedWeightBindingV1::Sampling { .. })
                })?;
                "Choose a supported sampling-weight normalization or remove the sampling-weight binding; no executable plan was emitted.".into()
            }
            WeightCapabilityCodeV1::LegacyCaseWeightBindingAmbiguous => {
                if self.declaration.as_ref().is_some_and(|declaration| {
                    matches!(
                        declaration.binding(),
                        ResolvedWeightBindingV1::Case { source_column, .. }
                            if source_column == &self.subject
                    )
                }) {
                    return Err(WeightCapabilityIssueContractErrorV1::LegacyBindingIsExact);
                }
                format!(
                    "Legacy settings.case_weight_column '{}' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted.",
                    self.subject
                )
            }
        };
        if self.corrective_action != expected_action {
            return Err(WeightCapabilityIssueContractErrorV1::CorrectiveActionMismatch);
        }
        Ok(())
    }

    fn ensure_resolved_kind(
        &self,
        expected_kind: impl FnOnce(&ResolvedWeightBindingV1) -> bool,
    ) -> Result<(), WeightCapabilityIssueContractErrorV1> {
        let Some(declaration) = &self.declaration else {
            return Err(WeightCapabilityIssueContractErrorV1::DeclarationRequired);
        };
        if !expected_kind(declaration.binding()) {
            return Err(WeightCapabilityIssueContractErrorV1::CodeKindMismatch);
        }
        if self.subject != declaration.binding().variable_id() {
            return Err(WeightCapabilityIssueContractErrorV1::SubjectMismatch);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WeightCapabilityIssueWireV1 {
    code: WeightCapabilityCodeV1,
    target: WeightCapabilityTargetV1,
    #[serde(deserialize_with = "deserialize_required_option")]
    declaration: Option<ResolvedWeightDeclarationV1>,
    subject: String,
    corrective_action: String,
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

impl<'de> Deserialize<'de> for WeightCapabilityIssueV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = WeightCapabilityIssueWireV1::deserialize(deserializer)?;
        let issue = Self {
            code: wire.code,
            target: wire.target,
            declaration: wire.declaration,
            subject: wire.subject,
            corrective_action: wire.corrective_action,
        };
        issue.ensure_valid().map_err(de::Error::custom)?;
        Ok(issue)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum WeightCapabilityIssueContractErrorV1 {
    #[error(transparent)]
    InvalidDeclaration(#[from] WeightDeclarationContractErrorV1),
    #[error("weight capability issue subject cannot be empty")]
    EmptySubject,
    #[error("weight capability issue code requires a resolved declaration")]
    DeclarationRequired,
    #[error("weight capability issue code and declaration kind differ")]
    CodeKindMismatch,
    #[error("weight capability issue subject differs from the resolved variable id")]
    SubjectMismatch,
    #[error("legacy case-weight ambiguity cannot contain an exact matching case binding")]
    LegacyBindingIsExact,
    #[error("weight capability issue corrective action differs from its typed code")]
    CorrectiveActionMismatch,
}

impl fmt::Display for WeightCapabilityIssueV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} for {} at {}: {}",
            self.code, self.target, self.subject, self.corrective_action
        )
    }
}

impl Error for WeightCapabilityIssueV1 {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, StructuralPath,
        convert_legacy_basic_model_v4,
    };
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn weighted_model(weight: SemWeightBindingV4) -> SemModelV4 {
        let legacy = ModelSpec {
            id: Uuid::nil(),
            name: "Weights".into(),
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
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        model.variables.push(SemVariableV4::Observed {
            id: "observed:weight".into(),
            label: "Weight".into(),
            source_column: "survey_weight".into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        let SemDataBindingV4::Raw {
            dataset_id,
            weight: configured,
            ..
        } = &mut model.data_binding
        else {
            unreachable!()
        };
        *dataset_id = "dataset:survey".into();
        *configured = Some(weight);
        model.ensure_valid().unwrap();
        model
    }

    #[test]
    fn resolved_weight_declaration_v1_preserves_kind_variable_source_dataset_and_normalization() {
        let bindings = [
            SemWeightBindingV4::Case {
                variable: "observed:weight".into(),
            },
            SemWeightBindingV4::Frequency {
                variable: "observed:weight".into(),
            },
            SemWeightBindingV4::Sampling {
                variable: "observed:weight".into(),
                normalization: SamplingWeightNormalizationV4::MeanOne,
            },
        ];
        for binding in bindings {
            let declaration = resolve_weight_declaration_v1(&weighted_model(binding))
                .unwrap()
                .unwrap();
            assert_eq!(
                declaration.contract_version(),
                WEIGHT_DECLARATION_CONTRACT_VERSION_V1
            );
            assert_eq!(declaration.dataset_id(), "dataset:survey");
            assert_eq!(declaration.binding().variable_id(), "observed:weight");
            assert_eq!(declaration.binding().source_column(), "survey_weight");
            let round_trip: ResolvedWeightDeclarationV1 =
                serde_json::from_value(serde_json::to_value(&declaration).unwrap()).unwrap();
            assert_eq!(round_trip, declaration);
        }
    }

    #[test]
    fn weight_declaration_changes_sem_model_scientific_identity() {
        let case = weighted_model(SemWeightBindingV4::Case {
            variable: "observed:weight".into(),
        });
        let frequency = weighted_model(SemWeightBindingV4::Frequency {
            variable: "observed:weight".into(),
        });
        let sampling_mean_one = weighted_model(SemWeightBindingV4::Sampling {
            variable: "observed:weight".into(),
            normalization: SamplingWeightNormalizationV4::MeanOne,
        });
        let sampling_sum_to_n = weighted_model(SemWeightBindingV4::Sampling {
            variable: "observed:weight".into(),
            normalization: SamplingWeightNormalizationV4::SumToSampleSize,
        });
        let digests = [
            case.scientific_sha256().unwrap(),
            frequency.scientific_sha256().unwrap(),
            sampling_mean_one.scientific_sha256().unwrap(),
            sampling_sum_to_n.scientific_sha256().unwrap(),
        ];
        for (index, digest) in digests.iter().enumerate() {
            assert!(digests.iter().skip(index + 1).all(|other| other != digest));
        }
    }

    #[test]
    fn weight_capability_issue_v1_uses_stable_typed_codes() {
        let cases = [
            (
                SemWeightBindingV4::Case {
                    variable: "observed:weight".into(),
                },
                WeightCapabilityCodeV1::CaseWeightUnsupported,
            ),
            (
                SemWeightBindingV4::Frequency {
                    variable: "observed:weight".into(),
                },
                WeightCapabilityCodeV1::FrequencyWeightUnsupported,
            ),
            (
                SemWeightBindingV4::Sampling {
                    variable: "observed:weight".into(),
                    normalization: SamplingWeightNormalizationV4::None,
                },
                WeightCapabilityCodeV1::SamplingWeightUnsupported,
            ),
        ];
        for (binding, expected) in cases {
            let declaration = resolve_weight_declaration_v1(&weighted_model(binding))
                .unwrap()
                .unwrap();
            let issue = WeightCapabilityIssueV1::unsupported(
                WeightCapabilityTargetV1::PlsPlanV2,
                declaration,
            );
            assert_eq!(issue.code, expected);
            assert_eq!(
                serde_json::to_value(&issue).unwrap()["code"],
                serde_json::Value::String(expected.as_str().into())
            );
        }

        let case_declaration =
            resolve_weight_declaration_v1(&weighted_model(SemWeightBindingV4::Case {
                variable: "observed:weight".into(),
            }))
            .unwrap()
            .unwrap();
        let case_issue = WeightCapabilityIssueV1::unsupported(
            WeightCapabilityTargetV1::PlsPlanV2,
            case_declaration,
        );
        assert_eq!(
            serde_json::to_value(case_issue).unwrap(),
            serde_json::json!({
                "code": "case_weight_unsupported",
                "target": "pls_plan_v2",
                "declaration": {
                    "contract_version": "sem_weight_declaration_v1",
                    "dataset_id": "dataset:survey",
                    "binding": {
                        "kind": "case",
                        "variable_id": "observed:weight",
                        "source_column": "survey_weight"
                    }
                },
                "subject": "observed:weight",
                "corrective_action": "Remove the case-weight binding or choose an estimator that explicitly supports case weights; no executable plan was emitted."
            })
        );

        let legacy_issue = WeightCapabilityIssueV1::legacy_case_weight_binding_ambiguous(
            WeightCapabilityTargetV1::PlsPlanV2,
            " case_weight ",
            None,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(legacy_issue).unwrap(),
            serde_json::json!({
                "code": "legacy_case_weight_binding_ambiguous",
                "target": "pls_plan_v2",
                "declaration": null,
                "subject": " case_weight ",
                "corrective_action": "Legacy settings.case_weight_column ' case_weight ' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted."
            })
        );

        let whitespace_legacy = WeightCapabilityIssueV1::legacy_case_weight_binding_ambiguous(
            WeightCapabilityTargetV1::PlsPlanV2,
            "   ",
            None,
        )
        .unwrap();
        let whitespace_value = serde_json::json!({
            "code": "legacy_case_weight_binding_ambiguous",
            "target": "pls_plan_v2",
            "declaration": null,
            "subject": "   ",
            "corrective_action": "Legacy settings.case_weight_column '   ' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted."
        });
        assert_eq!(
            serde_json::to_value(&whitespace_legacy).unwrap(),
            whitespace_value
        );
        assert_eq!(
            serde_json::from_value::<WeightCapabilityIssueV1>(whitespace_value).unwrap(),
            whitespace_legacy
        );
    }

    #[test]
    fn weight_capability_issue_v1_deserialization_rejects_incoherent_contracts() {
        let declaration =
            resolve_weight_declaration_v1(&weighted_model(SemWeightBindingV4::Case {
                variable: "observed:weight".into(),
            }))
            .unwrap()
            .unwrap();
        let valid = serde_json::to_value(WeightCapabilityIssueV1::unsupported(
            WeightCapabilityTargetV1::PlsPlanV2,
            declaration,
        ))
        .unwrap();
        assert!(serde_json::from_value::<WeightCapabilityIssueV1>(valid.clone()).is_ok());

        let mut code_kind_drift = valid.clone();
        code_kind_drift["code"] = serde_json::json!("sampling_weight_unsupported");
        assert!(serde_json::from_value::<WeightCapabilityIssueV1>(code_kind_drift).is_err());

        let mut normalization_kind_drift = valid.clone();
        normalization_kind_drift["code"] =
            serde_json::json!("sampling_weight_normalization_unsupported");
        normalization_kind_drift["corrective_action"] = serde_json::json!(
            "Choose a supported sampling-weight normalization or remove the sampling-weight binding; no executable plan was emitted."
        );
        assert!(
            serde_json::from_value::<WeightCapabilityIssueV1>(normalization_kind_drift).is_err()
        );

        let sampling_declaration =
            resolve_weight_declaration_v1(&weighted_model(SemWeightBindingV4::Sampling {
                variable: "observed:weight".into(),
                normalization: SamplingWeightNormalizationV4::None,
            }))
            .unwrap()
            .unwrap();
        let mut case_code_with_sampling = serde_json::to_value(
            WeightCapabilityIssueV1::sampling_normalization_unsupported(
                WeightCapabilityTargetV1::PlsPlanV2,
                sampling_declaration,
            )
            .unwrap(),
        )
        .unwrap();
        case_code_with_sampling["code"] = serde_json::json!("case_weight_unsupported");
        case_code_with_sampling["corrective_action"] = serde_json::json!(
            "Remove the case-weight binding or choose an estimator that explicitly supports case weights; no executable plan was emitted."
        );
        assert!(
            serde_json::from_value::<WeightCapabilityIssueV1>(case_code_with_sampling).is_err()
        );

        let mut subject_drift = valid.clone();
        subject_drift["subject"] = serde_json::json!("observed:other");
        assert!(serde_json::from_value::<WeightCapabilityIssueV1>(subject_drift).is_err());

        let mut action_drift = valid.clone();
        action_drift["corrective_action"] = serde_json::json!("Execute anyway.");
        assert!(serde_json::from_value::<WeightCapabilityIssueV1>(action_drift).is_err());

        let mut contract_drift = valid.clone();
        contract_drift["declaration"]["contract_version"] = serde_json::json!("future");
        assert!(serde_json::from_value::<WeightCapabilityIssueV1>(contract_drift).is_err());

        let mut missing_declaration = valid.clone();
        missing_declaration
            .as_object_mut()
            .unwrap()
            .remove("declaration");
        assert!(serde_json::from_value::<WeightCapabilityIssueV1>(missing_declaration).is_err());

        let mut false_legacy_ambiguity = valid;
        false_legacy_ambiguity["code"] = serde_json::json!("legacy_case_weight_binding_ambiguous");
        false_legacy_ambiguity["subject"] = serde_json::json!("survey_weight");
        false_legacy_ambiguity["corrective_action"] = serde_json::json!(
            "Legacy settings.case_weight_column 'survey_weight' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted."
        );
        assert!(serde_json::from_value::<WeightCapabilityIssueV1>(false_legacy_ambiguity).is_err());
    }

    #[test]
    fn special_issue_constructors_and_parts_resolver_enforce_contracts() {
        let case_declaration =
            resolve_weight_declaration_v1(&weighted_model(SemWeightBindingV4::Case {
                variable: "observed:weight".into(),
            }))
            .unwrap()
            .unwrap();
        assert!(
            WeightCapabilityIssueV1::sampling_normalization_unsupported(
                WeightCapabilityTargetV1::PlsPlanV2,
                case_declaration.clone(),
            )
            .is_err()
        );

        let exact_legacy = WeightCapabilityIssueV1::legacy_case_weight_binding_ambiguous(
            WeightCapabilityTargetV1::PlsPlanV2,
            "survey_weight",
            Some(case_declaration),
        )
        .unwrap();
        assert_eq!(
            exact_legacy.code,
            WeightCapabilityCodeV1::CaseWeightUnsupported
        );
        assert!(
            WeightCapabilityIssueV1::legacy_case_weight_binding_ambiguous(
                WeightCapabilityTargetV1::PlsPlanV2,
                "",
                None,
            )
            .is_err()
        );

        let sampling_declaration =
            resolve_weight_declaration_v1(&weighted_model(SemWeightBindingV4::Sampling {
                variable: "observed:weight".into(),
                normalization: SamplingWeightNormalizationV4::MeanOne,
            }))
            .unwrap()
            .unwrap();
        assert_eq!(
            WeightCapabilityIssueV1::sampling_normalization_unsupported(
                WeightCapabilityTargetV1::PlsPlanV2,
                sampling_declaration,
            )
            .unwrap()
            .code,
            WeightCapabilityCodeV1::SamplingWeightNormalizationUnsupported
        );

        let invalid_variables = vec![SemVariableV4::Observed {
            id: "".into(),
            label: "Weight".into(),
            source_column: "weight".into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        }];
        assert!(matches!(
            resolve_weight_declaration_parts_v1(
                "dataset",
                &SemWeightBindingV4::Case {
                    variable: "".into(),
                },
                &invalid_variables,
            ),
            Err(WeightDeclarationResolutionErrorV1::InvalidDeclaration(
                WeightDeclarationContractErrorV1::EmptyVariableId
            ))
        ));
    }
}
