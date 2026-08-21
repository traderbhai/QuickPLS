use crate::{
    AnalysisRecipe, AnalysisSettings, GeneralSemConfigV1, GeneralSemConfigV1ValidationError,
    LegacyBasicModelConversionErrorV4, LegacyBasicModelInterpretationV4, LegacyDisplayCovarianceV4,
    MethodConfig, ModelSpec, SemAnnotationV4, SemEndpointV4, SemModelV4, SemModelV4ValidationError,
    SemParameterTargetV4, SemParameterV4, SemRelationV4, convert_legacy_basic_model_v4,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Staged wire contract for SemModelV4-backed recipes. The currently executable
/// recipe remains `AnalysisRecipe` schema v3 until estimator adapters consume
/// this contract explicitly.
pub const ANALYSIS_RECIPE_V4_SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnalysisRecipeModelBindingV4 {
    EmbeddedSemModelV4 {
        model: SemModelV4,
        scientific_sha256: String,
    },
    ProjectSemModelV4Reference {
        model_id: String,
        scientific_sha256: String,
    },
    LegacyEstimandUnspecified {
        legacy_model_id: String,
        legacy_model_sha256: String,
    },
}

impl AnalysisRecipeModelBindingV4 {
    pub fn model_id(&self) -> &str {
        match self {
            Self::EmbeddedSemModelV4 { model, .. } => &model.id,
            Self::ProjectSemModelV4Reference { model_id, .. } => model_id,
            Self::LegacyEstimandUnspecified {
                legacy_model_id, ..
            } => legacy_model_id,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LegacyEstimandConfirmationV4 {
    NotLegacy,
    LegacyEstimandUnspecified,
    ConfirmedComposite,
    ConfirmedCommonFactor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LegacyRecipeSourceV4 {
    pub source_schema_version: u32,
    pub source_recipe_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AnalysisRecipeV4 {
    pub schema_version: u32,
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub dataset_fingerprint: String,
    pub model_binding: AnalysisRecipeModelBindingV4,
    pub estimand_confirmation: LegacyEstimandConfirmationV4,
    pub settings: AnalysisSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_config: Option<MethodConfig>,
    /// Optional General SEM effect/probing/inference request layered over the
    /// estimator-specific method configuration. Existing Recipe-v4 documents
    /// omit this field and retain their exact behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general_sem_config: Option<GeneralSemConfigV1>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_source: Option<LegacyRecipeSourceV4>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisRecipeV4ExecutionReadiness {
    Ready,
    NeedsEstimandConfirmation,
    ModelResolutionRequired,
    MethodConfigurationRequired,
    EstimatorAdapterNotImplemented,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum AnalysisRecipeV4Error {
    #[error("analysis recipe v4 requires schema_version 4 (found {0})")]
    Schema(u32),
    #[error("analysis recipe v4 dataset_fingerprint must not be empty")]
    EmptyDatasetFingerprint,
    #[error("analysis recipe v4 model identifier must not be empty")]
    EmptyModelId,
    #[error("analysis recipe v4 {field} must be a lowercase 64-character SHA-256 digest")]
    InvalidSha256 { field: &'static str },
    #[error("analysis recipe v4 estimand confirmation does not match its model binding")]
    EstimandBindingMismatch,
    #[error("embedded SemModelV4 identifier differs from the declared binding identifier")]
    EmbeddedModelIdMismatch,
    #[error("SemModelV4 scientific digest does not match the recipe binding")]
    ScientificDigestMismatch,
    #[error("the supplied legacy model does not match the pending recipe binding")]
    LegacyModelMismatch,
    #[error("only historical analysis recipe schemas 1 through 3 migrate to schema v4 (found {0})")]
    UnsupportedMigrationSource(u32),
    #[error("the recipe still requires factor-versus-composite confirmation")]
    EstimandConfirmationRequired,
    #[error("the requested legacy interpretation is not a confirmation")]
    InvalidLegacyInterpretation,
    #[error(transparent)]
    LegacyConversion(#[from] LegacyBasicModelConversionErrorV4),
    #[error(transparent)]
    InvalidSemModel(#[from] SemModelV4ValidationError),
    #[error(transparent)]
    InvalidGeneralSemConfig(#[from] GeneralSemConfigV1ValidationError),
}

impl AnalysisRecipeV4 {
    pub fn ensure_valid(&self) -> Result<(), AnalysisRecipeV4Error> {
        if self.schema_version != ANALYSIS_RECIPE_V4_SCHEMA_VERSION {
            return Err(AnalysisRecipeV4Error::Schema(self.schema_version));
        }
        if self.dataset_fingerprint.trim().is_empty() {
            return Err(AnalysisRecipeV4Error::EmptyDatasetFingerprint);
        }
        if self.model_binding.model_id().trim().is_empty() {
            return Err(AnalysisRecipeV4Error::EmptyModelId);
        }

        match (&self.model_binding, self.estimand_confirmation) {
            (
                AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
                    legacy_model_sha256,
                    ..
                },
                LegacyEstimandConfirmationV4::LegacyEstimandUnspecified,
            ) => validate_sha256("legacy_model_sha256", legacy_model_sha256)?,
            (
                AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                    model,
                    scientific_sha256,
                },
                LegacyEstimandConfirmationV4::NotLegacy
                | LegacyEstimandConfirmationV4::ConfirmedComposite
                | LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
            ) => {
                model.ensure_valid()?;
                validate_sha256("scientific_sha256", scientific_sha256)?;
                if model.scientific_sha256()? != *scientific_sha256 {
                    return Err(AnalysisRecipeV4Error::ScientificDigestMismatch);
                }
            }
            (
                AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                    scientific_sha256, ..
                },
                LegacyEstimandConfirmationV4::NotLegacy
                | LegacyEstimandConfirmationV4::ConfirmedComposite
                | LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
            ) => validate_sha256("scientific_sha256", scientific_sha256)?,
            _ => return Err(AnalysisRecipeV4Error::EstimandBindingMismatch),
        }

        if let Some(source) = &self.legacy_source {
            if !(1..=3).contains(&source.source_schema_version) {
                return Err(AnalysisRecipeV4Error::UnsupportedMigrationSource(
                    source.source_schema_version,
                ));
            }
            validate_sha256("source_recipe_sha256", &source.source_recipe_sha256)?;
        }
        if let Some(config) = &self.general_sem_config {
            config.ensure_valid()?;
        }
        Ok(())
    }

    /// Reports execution readiness without implying that current estimators
    /// consume this staged recipe. Callers must explicitly attest that an
    /// estimator adapter exists for the selected capability cell.
    pub fn execution_readiness(
        &self,
        resolved_project_model: Option<&SemModelV4>,
        estimator_adapter_available: bool,
    ) -> Result<AnalysisRecipeV4ExecutionReadiness, AnalysisRecipeV4Error> {
        self.ensure_valid()?;
        let model = match &self.model_binding {
            AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified { .. } => {
                return Ok(AnalysisRecipeV4ExecutionReadiness::NeedsEstimandConfirmation);
            }
            AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 { model, .. } => model,
            AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                model_id,
                scientific_sha256,
            } => {
                let Some(model) = resolved_project_model else {
                    return Ok(AnalysisRecipeV4ExecutionReadiness::ModelResolutionRequired);
                };
                if model.id != *model_id {
                    return Err(AnalysisRecipeV4Error::EmbeddedModelIdMismatch);
                }
                model.ensure_valid()?;
                if model.scientific_sha256()? != *scientific_sha256 {
                    return Err(AnalysisRecipeV4Error::ScientificDigestMismatch);
                }
                model
            }
        };
        model.ensure_valid()?;
        if self.method_config.is_none() {
            return Ok(AnalysisRecipeV4ExecutionReadiness::MethodConfigurationRequired);
        }
        if !estimator_adapter_available {
            return Ok(AnalysisRecipeV4ExecutionReadiness::EstimatorAdapterNotImplemented);
        }
        Ok(AnalysisRecipeV4ExecutionReadiness::Ready)
    }
}

pub fn migrate_analysis_recipe_to_v4_pending(
    source: &AnalysisRecipe,
) -> Result<AnalysisRecipeV4, AnalysisRecipeV4Error> {
    if !(1..=3).contains(&source.schema_version) {
        return Err(AnalysisRecipeV4Error::UnsupportedMigrationSource(
            source.schema_version,
        ));
    }
    let source_recipe_sha256 = sha256_serialized(source);
    let legacy_model_sha256 = sha256_serialized(&source.model);
    let migrated = AnalysisRecipeV4 {
        schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
        id: source.id,
        created_at: source.created_at,
        dataset_fingerprint: source.dataset_fingerprint.clone(),
        model_binding: AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
            legacy_model_id: source.model.id.to_string(),
            legacy_model_sha256,
        },
        estimand_confirmation: LegacyEstimandConfirmationV4::LegacyEstimandUnspecified,
        settings: source.settings.clone(),
        method_config: source.method_config.clone(),
        general_sem_config: None,
        metadata: source.metadata.clone(),
        legacy_source: Some(LegacyRecipeSourceV4 {
            source_schema_version: source.schema_version,
            source_recipe_sha256,
        }),
    };
    migrated.ensure_valid()?;
    Ok(migrated)
}

pub fn confirm_legacy_recipe_estimand_v4(
    pending: &AnalysisRecipeV4,
    legacy_model: &ModelSpec,
    display_covariances: &[LegacyDisplayCovarianceV4],
    interpretation: LegacyBasicModelInterpretationV4,
) -> Result<(AnalysisRecipeV4, SemModelV4), AnalysisRecipeV4Error> {
    pending.ensure_valid()?;
    let AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
        legacy_model_id,
        legacy_model_sha256,
    } = &pending.model_binding
    else {
        return Err(AnalysisRecipeV4Error::EstimandBindingMismatch);
    };
    if legacy_model.id.to_string() != *legacy_model_id
        || sha256_serialized(legacy_model) != *legacy_model_sha256
    {
        return Err(AnalysisRecipeV4Error::LegacyModelMismatch);
    }
    let confirmation = match interpretation {
        LegacyBasicModelInterpretationV4::PlsComposite => {
            LegacyEstimandConfirmationV4::ConfirmedComposite
        }
        LegacyBasicModelInterpretationV4::CbsemCommonFactor => {
            LegacyEstimandConfirmationV4::ConfirmedCommonFactor
        }
        LegacyBasicModelInterpretationV4::Unspecified => {
            return Err(AnalysisRecipeV4Error::InvalidLegacyInterpretation);
        }
    };
    let model = convert_legacy_basic_model_v4(legacy_model, interpretation, display_covariances)?;
    let mut confirmed = pending.clone();
    confirmed.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
        model_id: model.id.clone(),
        scientific_sha256: model.scientific_sha256()?,
    };
    confirmed.estimand_confirmation = confirmation;
    confirmed.ensure_valid()?;
    Ok((confirmed, model))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConvertDisplayCovarianceV4 {
    pub annotation_id: String,
    pub relation_id: String,
    pub parameter_id: String,
    pub parameter_label: String,
    #[serde(default)]
    pub start: Option<f64>,
    #[serde(default)]
    pub lower: Option<f64>,
    #[serde(default)]
    pub upper: Option<f64>,
    #[serde(default)]
    pub equality_label: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ConvertDisplayCovarianceV4Error {
    #[error("display-only covariance annotation {0} does not exist")]
    AnnotationMissing(String),
    #[error("annotation {0} is not a display-only covariance")]
    AnnotationKind(String),
    #[error("relation identifier {0} already exists")]
    RelationIdExists(String),
    #[error("parameter identifier {0} already exists")]
    ParameterIdExists(String),
    #[error(transparent)]
    InvalidModel(#[from] SemModelV4ValidationError),
}

/// Explicitly promotes one presentation-only covariance annotation into a
/// scientific covariance. Migration itself never calls this operation.
pub fn convert_display_covariance_to_model_v4(
    source: &SemModelV4,
    operation: &ConvertDisplayCovarianceV4,
) -> Result<SemModelV4, ConvertDisplayCovarianceV4Error> {
    source.ensure_valid()?;
    if source
        .relations
        .iter()
        .any(|relation| relation.id() == operation.relation_id)
    {
        return Err(ConvertDisplayCovarianceV4Error::RelationIdExists(
            operation.relation_id.clone(),
        ));
    }
    if source
        .parameters
        .iter()
        .any(|parameter| parameter.id() == operation.parameter_id)
    {
        return Err(ConvertDisplayCovarianceV4Error::ParameterIdExists(
            operation.parameter_id.clone(),
        ));
    }
    let annotation = source
        .annotations
        .iter()
        .find(|annotation| annotation.id() == operation.annotation_id)
        .ok_or_else(|| {
            ConvertDisplayCovarianceV4Error::AnnotationMissing(operation.annotation_id.clone())
        })?;
    let SemAnnotationV4::DisplayOnlyCovariance { left, right, .. } = annotation else {
        return Err(ConvertDisplayCovarianceV4Error::AnnotationKind(
            operation.annotation_id.clone(),
        ));
    };

    let left = SemEndpointV4::Variable(left.clone());
    let right = SemEndpointV4::Variable(right.clone());
    let mut converted = source.clone();
    converted
        .annotations
        .retain(|annotation| annotation.id() != operation.annotation_id);
    converted.relations.push(SemRelationV4::Covariance {
        id: operation.relation_id.clone(),
        left: left.clone(),
        right: right.clone(),
        parameter: operation.parameter_id.clone(),
    });
    converted.parameters.push(SemParameterV4::Free {
        id: operation.parameter_id.clone(),
        label: operation.parameter_label.clone(),
        target: SemParameterTargetV4::Covariance { left, right },
        start: operation.start,
        lower: operation.lower,
        upper: operation.upper,
        equality_label: operation.equality_label.clone(),
        group_overrides: Vec::new(),
    });
    converted = converted.canonicalized();
    converted.ensure_valid()?;
    Ok(converted)
}

pub fn sha256_serialized<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("scientific contract values must serialize");
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_sha256(field: &'static str, value: &str) -> Result<(), AnalysisRecipeV4Error> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(AnalysisRecipeV4Error::InvalidSha256 { field })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, Construct,
        GENERAL_SEM_CONFIG_V1_SCHEMA_VERSION, MeasurementMode, MethodConfig, StructuralPath,
    };

    fn legacy_model() -> ModelSpec {
        ModelSpec {
            id: Uuid::from_u128(1),
            name: "Legacy model".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "Predictor".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Outcome".into(),
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

    fn legacy_recipe() -> AnalysisRecipe {
        AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(2),
            created_at: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: "dataset-sha256".into(),
            model: legacy_model(),
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn migration_is_pending_and_cannot_execute_without_estimand_confirmation() {
        let migrated = migrate_analysis_recipe_to_v4_pending(&legacy_recipe()).unwrap();
        assert_eq!(migrated.schema_version, 4);
        assert_eq!(
            migrated.execution_readiness(None, false).unwrap(),
            AnalysisRecipeV4ExecutionReadiness::NeedsEstimandConfirmation
        );
        assert!(matches!(
            migrated.model_binding,
            AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified { .. }
        ));
    }

    #[test]
    fn confirmation_creates_an_explicit_sem_model_reference_but_not_an_estimator_adapter() {
        let source = legacy_recipe();
        let pending = migrate_analysis_recipe_to_v4_pending(&source).unwrap();
        let (confirmed, model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source.model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        assert_eq!(
            confirmed.execution_readiness(Some(&model), false).unwrap(),
            AnalysisRecipeV4ExecutionReadiness::EstimatorAdapterNotImplemented
        );
        assert_eq!(
            confirmed.estimand_confirmation,
            LegacyEstimandConfirmationV4::ConfirmedComposite
        );
    }

    #[test]
    fn display_covariance_remains_nonscientific_until_the_explicit_conversion_operation() {
        let source = legacy_recipe();
        let pending = migrate_analysis_recipe_to_v4_pending(&source).unwrap();
        let (confirmed, model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source.model,
            &[LegacyDisplayCovarianceV4 {
                id: "visual-covariance".into(),
                left_construct: "x".into(),
                right_construct: "y".into(),
                label: Some("display only".into()),
            }],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        let original_hash = model.scientific_sha256().unwrap();
        assert_eq!(confirmed.model_binding.model_id(), model.id);
        assert!(
            !model
                .relations
                .iter()
                .any(|relation| matches!(relation, SemRelationV4::Covariance { .. }))
        );

        let converted = convert_display_covariance_to_model_v4(
            &model,
            &ConvertDisplayCovarianceV4 {
                annotation_id: "visual-covariance".into(),
                relation_id: "scientific-covariance".into(),
                parameter_id: "scientific-covariance-parameter".into(),
                parameter_label: "Cov(X, Y)".into(),
                start: Some(0.1),
                lower: None,
                upper: None,
                equality_label: None,
            },
        )
        .unwrap();
        assert!(converted.annotations.is_empty());
        assert!(
            converted
                .relations
                .iter()
                .any(|relation| matches!(relation, SemRelationV4::Covariance { .. }))
        );
        assert_ne!(converted.scientific_sha256().unwrap(), original_hash);
    }

    #[test]
    fn binding_hash_tampering_fails_closed() {
        let mut migrated = migrate_analysis_recipe_to_v4_pending(&legacy_recipe()).unwrap();
        if let AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
            legacy_model_sha256,
            ..
        } = &mut migrated.model_binding
        {
            *legacy_model_sha256 = "0".repeat(64);
        }
        assert_eq!(
            confirm_legacy_recipe_estimand_v4(
                &migrated,
                &legacy_model(),
                &[],
                LegacyBasicModelInterpretationV4::PlsComposite,
            )
            .unwrap_err(),
            AnalysisRecipeV4Error::LegacyModelMismatch
        );
    }

    #[test]
    fn general_sem_config_is_additive_versioned_and_validated() {
        let source = legacy_recipe();
        let pending = migrate_analysis_recipe_to_v4_pending(&source).unwrap();
        assert!(
            serde_json::to_value(&pending)
                .unwrap()
                .get("general_sem_config")
                .is_none()
        );

        let (mut configured, _) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source.model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        configured.general_sem_config = Some(GeneralSemConfigV1::default());
        configured.ensure_valid().unwrap();
        assert_eq!(
            serde_json::to_value(&configured).unwrap()["general_sem_config"]["schema_version"],
            GENERAL_SEM_CONFIG_V1_SCHEMA_VERSION
        );

        configured
            .general_sem_config
            .as_mut()
            .unwrap()
            .schema_version = 2;
        assert!(matches!(
            configured.ensure_valid(),
            Err(AnalysisRecipeV4Error::InvalidGeneralSemConfig(
                GeneralSemConfigV1ValidationError::SchemaVersion { found: 2 }
            ))
        ));
    }
}
