use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const PROJECT_SCHEMA_VERSION: u32 = 2;
pub const RESULT_SCHEMA_VERSION: u32 = 1;
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMethod {
    PlsPm,
    Bootstrap,
    Plsc,
    Wpls,
    Cca,
    CtaPls,
    Endogeneity,
    NonlinearEffects,
    ModeratedMediation,
    Predict,
    Mga,
    Ipma,
    Cbsem,
    Pca,
    Gsca,
    Regression,
    Nca,
    Legacy,
}

impl AnalysisMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PlsPm => "pls_pm",
            Self::Bootstrap => "bootstrap",
            Self::Plsc => "plsc",
            Self::Wpls => "wpls",
            Self::Cca => "cca",
            Self::CtaPls => "cta_pls",
            Self::Endogeneity => "endogeneity",
            Self::NonlinearEffects => "nonlinear_effects",
            Self::ModeratedMediation => "moderated_mediation",
            Self::Predict => "predict",
            Self::Mga => "mga",
            Self::Ipma => "ipma",
            Self::Cbsem => "cbsem",
            Self::Pca => "pca",
            Self::Gsca => "gsca",
            Self::Regression => "regression",
            Self::Nca => "nca",
            Self::Legacy => "legacy",
        }
    }
}

impl std::fmt::Display for AnalysisMethod {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementMode {
    Reflective,
    Formative,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Construct {
    pub id: String,
    pub name: String,
    pub short_name: String,
    pub mode: MeasurementMode,
    pub indicators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralPath {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlPath {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionMethod {
    TwoStageProductScore,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractionTerm {
    pub id: String,
    pub predictor: String,
    pub moderator: String,
    pub product_construct: String,
    pub outcome: String,
    pub method: InteractionMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HigherOrderMethod {
    RepeatedIndicators,
    TwoStage,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HigherOrderConstruct {
    pub id: String,
    pub components: Vec<String>,
    pub method: HigherOrderMethod,
    #[serde(default)]
    pub stage_one_recipe: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelSpec {
    pub id: Uuid,
    pub name: String,
    pub constructs: Vec<Construct>,
    pub paths: Vec<StructuralPath>,
    #[serde(default)]
    pub controls: Vec<ControlPath>,
    #[serde(default)]
    pub higher_order_constructs: Vec<HigherOrderConstruct>,
    #[serde(default)]
    pub interactions: Vec<InteractionTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeightingScheme {
    Path,
    Factor,
    Pca,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Preprocessing {
    #[default]
    Standardized,
    MeanCentered,
    Unstandardized,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MissingDataPolicy {
    #[default]
    ListwiseDeletion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisSettings {
    pub method: AnalysisMethod,
    pub weighting_scheme: WeightingScheme,
    pub tolerance: f64,
    pub max_iterations: u32,
    pub bootstrap_samples: u32,
    #[serde(default)]
    pub studentized_inner_samples: u32,
    #[serde(default)]
    pub permutation_samples: u32,
    pub seed: u64,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default = "default_confidence_level")]
    pub confidence_level: f64,
    #[serde(default)]
    pub preprocessing: Preprocessing,
    #[serde(default)]
    pub missing_data: MissingDataPolicy,
    #[serde(default)]
    pub case_weight_column: Option<String>,
}

impl Default for AnalysisSettings {
    fn default() -> Self {
        Self {
            method: AnalysisMethod::PlsPm,
            weighting_scheme: WeightingScheme::Path,
            tolerance: 1e-7,
            max_iterations: 3_000,
            bootstrap_samples: 0,
            studentized_inner_samples: 0,
            permutation_samples: 0,
            seed: 20_260_718,
            workers: default_workers(),
            confidence_level: default_confidence_level(),
            preprocessing: Preprocessing::Standardized,
            missing_data: MissingDataPolicy::ListwiseDeletion,
            case_weight_column: None,
        }
    }
}

const fn default_workers() -> usize {
    1
}

const fn default_confidence_level() -> f64 {
    0.95
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisRecipe {
    pub schema_version: u32,
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub dataset_fingerprint: String,
    pub model: ModelSpec,
    pub settings: AnalysisSettings,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Cancelling,
    Committing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobSnapshot {
    pub id: Uuid,
    pub state: JobState,
    pub phase: String,
    pub completed_units: u64,
    pub total_units: u64,
    pub message: Option<String>,
}

impl JobSnapshot {
    pub fn queued(total_units: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            state: JobState::Queued,
            phase: "queued".into(),
            completed_units: 0,
            total_units,
            message: None,
        }
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn method_identifier_has_a_stable_wire_format_and_rejects_unknown_values() {
        let settings = AnalysisSettings::default();
        let encoded = serde_json::to_value(&settings).unwrap();
        assert_eq!(encoded["method"], "pls_pm");
        for (method, expected) in [
            (AnalysisMethod::Plsc, "plsc"),
            (AnalysisMethod::Wpls, "wpls"),
            (AnalysisMethod::Cca, "cca"),
            (AnalysisMethod::CtaPls, "cta_pls"),
            (AnalysisMethod::Endogeneity, "endogeneity"),
            (AnalysisMethod::NonlinearEffects, "nonlinear_effects"),
            (AnalysisMethod::ModeratedMediation, "moderated_mediation"),
            (AnalysisMethod::Ipma, "ipma"),
        ] {
            assert_eq!(method.as_str(), expected);
            assert_eq!(serde_json::to_value(method).unwrap(), expected);
        }
        let mut unknown = encoded;
        unknown["method"] = serde_json::json!("not_a_method");
        assert!(serde_json::from_value::<AnalysisSettings>(unknown).is_err());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Information,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub level: DiagnosticLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunProvenance {
    pub recipe_id: Uuid,
    pub dataset_fingerprint: String,
    pub method: AnalysisMethod,
    pub method_version: String,
    pub engine_version: String,
    pub seed: u64,
    pub settings: AnalysisSettings,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisResult {
    pub schema_version: u32,
    pub id: Uuid,
    pub status: RunStatus,
    pub provenance: RunProvenance,
    pub diagnostics: Vec<Diagnostic>,
    pub payload: AnalysisPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnalysisPayload {
    PlsPmV1 {
        estimation: serde_json::Value,
        assessment: serde_json::Value,
    },
    PlsPmV2 {
        estimation: serde_json::Value,
        assessment: serde_json::Value,
        bootstrap: serde_json::Value,
    },
    PlsPmV3 {
        estimation: serde_json::Value,
        assessment: serde_json::Value,
        #[serde(default)]
        bootstrap: Option<serde_json::Value>,
        #[serde(default)]
        permutation: Option<serde_json::Value>,
    },
    Legacy {
        value: serde_json::Value,
    },
}

impl AnalysisResult {
    pub fn completed_pls(
        recipe: &AnalysisRecipe,
        method_version: impl Into<String>,
        started_at: DateTime<Utc>,
        estimation: serde_json::Value,
        assessment: serde_json::Value,
        warnings: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            schema_version: RESULT_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            status: RunStatus::Completed,
            provenance: RunProvenance {
                recipe_id: recipe.id,
                dataset_fingerprint: recipe.dataset_fingerprint.clone(),
                method: recipe.settings.method,
                method_version: method_version.into(),
                engine_version: ENGINE_VERSION.into(),
                seed: recipe.settings.seed,
                settings: recipe.settings.clone(),
                started_at,
                completed_at: Utc::now(),
            },
            diagnostics: warnings
                .into_iter()
                .map(|message| Diagnostic {
                    code: "estimation.warning".into(),
                    level: DiagnosticLevel::Warning,
                    message,
                })
                .collect(),
            payload: AnalysisPayload::PlsPmV1 {
                estimation,
                assessment,
            },
        }
    }

    pub fn completed_pls_bootstrap(
        recipe: &AnalysisRecipe,
        method_version: impl Into<String>,
        started_at: DateTime<Utc>,
        estimation: serde_json::Value,
        assessment: serde_json::Value,
        bootstrap: serde_json::Value,
        warnings: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut result = Self::completed_pls(
            recipe,
            method_version,
            started_at,
            estimation.clone(),
            assessment.clone(),
            warnings,
        );
        let payload = std::mem::replace(
            &mut result.payload,
            AnalysisPayload::Legacy {
                value: serde_json::Value::Null,
            },
        );
        let AnalysisPayload::PlsPmV1 {
            estimation,
            assessment,
        } = payload
        else {
            unreachable!()
        };
        result.payload = AnalysisPayload::PlsPmV2 {
            estimation,
            assessment,
            bootstrap,
        };
        result
    }

    pub fn completed_pls_inference(
        recipe: &AnalysisRecipe,
        method_version: impl Into<String>,
        started_at: DateTime<Utc>,
        estimation: serde_json::Value,
        assessment: serde_json::Value,
        bootstrap: Option<serde_json::Value>,
        permutation: Option<serde_json::Value>,
        warnings: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut result = Self::completed_pls(
            recipe,
            method_version,
            started_at,
            estimation.clone(),
            assessment.clone(),
            warnings,
        );
        result.payload = AnalysisPayload::PlsPmV3 {
            estimation,
            assessment,
            bootstrap,
            permutation,
        };
        result
    }
}

impl AnalysisRecipe {
    pub fn new(dataset_bytes: &[u8], model: ModelSpec, settings: AnalysisSettings) -> Self {
        Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: sha256_hex(dataset_bytes),
            model,
            settings,
            metadata: BTreeMap::new(),
        }
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
