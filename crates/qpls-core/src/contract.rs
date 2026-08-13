use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Current wire schema for an executable analysis recipe. This is deliberately
/// independent from the project archive schema.
pub const ANALYSIS_RECIPE_SCHEMA_VERSION: u32 = 3;
pub const LEGACY_ANALYSIS_RECIPE_SCHEMA_VERSION: u32 = 2;
/// Backward-compatible name used by schema-v2 recipe callers. Project archives
/// use `qpls_project::PROJECT_ARCHIVE_VERSION` instead.
pub const PROJECT_SCHEMA_VERSION: u32 = LEGACY_ANALYSIS_RECIPE_SCHEMA_VERSION;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisRecipeSchemaGeneration {
    V1,
    V2,
    V3,
    Future(u32),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupAnalysisMethod {
    Micom,
    MgaPermutation,
}

impl GroupAnalysisMethod {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Micom => "micom",
            Self::MgaPermutation => "mga_permutation",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SegmentationMethodConfig {
    pub segments: u32,
    pub starts: u32,
    pub minimum_segment_share: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemModelType {
    Cfa,
    Sem,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemEstimator {
    Ml,
    RobustMl,
    Wlsmv,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemInput {
    Raw,
    Covariance,
    Correlation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemInvarianceStep {
    Configural,
    Metric,
    Scalar,
}

impl CbsemInvarianceStep {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Configural => "configural",
            Self::Metric => "metric",
            Self::Scalar => "scalar",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "rule", rename_all = "snake_case", deny_unknown_fields)]
pub enum PcaRetentionConfig {
    Kaiser,
    Fixed { components: u32 },
    VarianceThreshold { threshold: f64 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RobustStandardError {
    Hc3,
}

impl RobustStandardError {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Hc3 => "hc3",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "model", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProcessRelationshipConfig {
    Mediation {
        x: String,
        mediator: String,
    },
    Moderation {
        x: String,
        moderator: String,
    },
    ModeratedMediation {
        x: String,
        mediator: String,
        moderator: String,
    },
    Graph {
        focal_predictor: String,
        paths: Vec<ProcessPathConfig>,
        moderators: Vec<ProcessModeratorConfig>,
        moderations: Vec<ProcessModerationConfig>,
        continuous_product_centering: ProcessContinuousCentering,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessPathConfig {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessModeratorScale {
    #[serde(rename = "continuous")]
    Continuous,
    #[serde(rename = "binary_0_1")]
    Binary01,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessModeratorConfig {
    pub variable: String,
    pub scale: ProcessModeratorScale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessModerationConfig {
    pub from: String,
    pub to: String,
    pub moderator: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditioning_moderator: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessContinuousCentering {
    #[serde(rename = "equation_complete_case_mean_v1")]
    EquationCompleteCaseMeanV1,
}

impl ProcessRelationshipConfig {
    const fn wire_name(&self) -> &'static str {
        match self {
            Self::Mediation { .. } => "mediation",
            Self::Moderation { .. } => "moderation",
            Self::ModeratedMediation { .. } => "moderated_mediation",
            Self::Graph { .. } => "graph",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RegressionModelConfig {
    Ols {
        robust_se: RobustStandardError,
    },
    Logistic,
    Process {
        relationship: ProcessRelationshipConfig,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegressionBootstrapAlgorithm {
    CaseResampling,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegressionBootstrapInterval {
    Percentile,
    Bca,
}

/// Dedicated outer case-resampling for standalone regression. The replicate
/// count, seed, worker count, and confidence level remain canonical in
/// `AnalysisSettings`; this typed value makes the scientific workflow
/// explicit and prevents it from being confused with PLS-PM resampling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RegressionBootstrapConfig {
    pub algorithm: RegressionBootstrapAlgorithm,
    pub intervals: Vec<RegressionBootstrapInterval>,
}

impl RegressionModelConfig {
    const fn wire_name(&self) -> &'static str {
        match self {
            Self::Ols { .. } => "ols",
            Self::Logistic => "logistic",
            Self::Process { .. } => "process",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NcaCeiling {
    CeFdh,
    CrFdh,
    Both,
}

impl NcaCeiling {
    const fn as_str(self) -> &'static str {
        match self {
            Self::CeFdh => "ce_fdh",
            Self::CrFdh => "cr_fdh",
            Self::Both => "both",
        }
    }
}

/// Schema-v3 executable configuration. The stable `kind` tag names the native
/// workflow; method-specific values are no longer interpreted from metadata.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MethodConfig {
    PlsAlgorithm,
    PlsBootstrap,
    PlsPermutation,
    Plsc,
    Wpls,
    Cca,
    CtaPls,
    Endogeneity,
    NonlinearEffects,
    ModeratedMediation,
    Predict {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pls_pos: Option<SegmentationMethodConfig>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fimix: Option<SegmentationMethodConfig>,
    },
    Mga {
        group_column: String,
        group_a: String,
        group_b: String,
        methods: Vec<GroupAnalysisMethod>,
        permutation_samples: u32,
        configural_invariance_confirmed: bool,
    },
    Ipma {
        targets: Vec<String>,
    },
    Cbsem {
        model_type: CbsemModelType,
        estimator: CbsemEstimator,
        input: CbsemInput,
        mean_structure: bool,
        bootstrap_samples: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group_column: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        invariance_steps: Vec<CbsemInvarianceStep>,
    },
    Pca {
        variables: Vec<String>,
        retention: PcaRetentionConfig,
    },
    Gsca,
    Regression {
        outcome: String,
        predictors: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        controls: Vec<String>,
        model: RegressionModelConfig,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bootstrap: Option<RegressionBootstrapConfig>,
    },
    Nca {
        condition: String,
        outcome: String,
        ceiling: NcaCeiling,
        permutation_samples: u32,
    },
    Legacy,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum MethodConfigWire {
    PlsAlgorithm,
    PlsBootstrap,
    PlsPermutation,
    Plsc,
    Wpls,
    Cca,
    CtaPls,
    Endogeneity,
    NonlinearEffects,
    ModeratedMediation,
    Predict {
        #[serde(default)]
        pls_pos: Option<SegmentationMethodConfig>,
        #[serde(default)]
        fimix: Option<SegmentationMethodConfig>,
    },
    Mga {
        group_column: String,
        group_a: String,
        group_b: String,
        methods: Vec<GroupAnalysisMethod>,
        permutation_samples: u32,
        configural_invariance_confirmed: bool,
    },
    Ipma {
        targets: Vec<String>,
    },
    Cbsem {
        model_type: CbsemModelType,
        estimator: CbsemEstimator,
        input: CbsemInput,
        mean_structure: bool,
        bootstrap_samples: u32,
        #[serde(default)]
        group_column: Option<String>,
        #[serde(default)]
        invariance_steps: Vec<CbsemInvarianceStep>,
    },
    Pca {
        variables: Vec<String>,
        retention: PcaRetentionConfig,
    },
    Gsca,
    Regression {
        outcome: String,
        predictors: Vec<String>,
        #[serde(default)]
        controls: Vec<String>,
        model: RegressionModelConfig,
        #[serde(default)]
        bootstrap: Option<RegressionBootstrapConfig>,
    },
    Nca {
        condition: String,
        outcome: String,
        ceiling: NcaCeiling,
        permutation_samples: u32,
    },
    Legacy,
}

impl From<MethodConfigWire> for MethodConfig {
    fn from(value: MethodConfigWire) -> Self {
        match value {
            MethodConfigWire::PlsAlgorithm => Self::PlsAlgorithm,
            MethodConfigWire::PlsBootstrap => Self::PlsBootstrap,
            MethodConfigWire::PlsPermutation => Self::PlsPermutation,
            MethodConfigWire::Plsc => Self::Plsc,
            MethodConfigWire::Wpls => Self::Wpls,
            MethodConfigWire::Cca => Self::Cca,
            MethodConfigWire::CtaPls => Self::CtaPls,
            MethodConfigWire::Endogeneity => Self::Endogeneity,
            MethodConfigWire::NonlinearEffects => Self::NonlinearEffects,
            MethodConfigWire::ModeratedMediation => Self::ModeratedMediation,
            MethodConfigWire::Predict { pls_pos, fimix } => Self::Predict { pls_pos, fimix },
            MethodConfigWire::Mga {
                group_column,
                group_a,
                group_b,
                methods,
                permutation_samples,
                configural_invariance_confirmed,
            } => Self::Mga {
                group_column,
                group_a,
                group_b,
                methods,
                permutation_samples,
                configural_invariance_confirmed,
            },
            MethodConfigWire::Ipma { targets } => Self::Ipma { targets },
            MethodConfigWire::Cbsem {
                model_type,
                estimator,
                input,
                mean_structure,
                bootstrap_samples,
                group_column,
                invariance_steps,
            } => Self::Cbsem {
                model_type,
                estimator,
                input,
                mean_structure,
                bootstrap_samples,
                group_column,
                invariance_steps,
            },
            MethodConfigWire::Pca {
                variables,
                retention,
            } => Self::Pca {
                variables,
                retention,
            },
            MethodConfigWire::Gsca => Self::Gsca,
            MethodConfigWire::Regression {
                outcome,
                predictors,
                controls,
                model,
                bootstrap,
            } => Self::Regression {
                outcome,
                predictors,
                controls,
                model,
                bootstrap,
            },
            MethodConfigWire::Nca {
                condition,
                outcome,
                ceiling,
                permutation_samples,
            } => Self::Nca {
                condition,
                outcome,
                ceiling,
                permutation_samples,
            },
            MethodConfigWire::Legacy => Self::Legacy,
        }
    }
}

impl<'de> Deserialize<'de> for MethodConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("method_config must be an object"))?;
        let kind = object
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| D::Error::custom("method_config requires a string kind"))?;
        let allowed: &[&str] = match kind {
            "pls_algorithm"
            | "pls_bootstrap"
            | "pls_permutation"
            | "plsc"
            | "wpls"
            | "cca"
            | "cta_pls"
            | "endogeneity"
            | "nonlinear_effects"
            | "moderated_mediation"
            | "gsca"
            | "legacy" => &["kind"],
            "predict" => &["kind", "pls_pos", "fimix"],
            "mga" => &[
                "kind",
                "group_column",
                "group_a",
                "group_b",
                "methods",
                "permutation_samples",
                "configural_invariance_confirmed",
            ],
            "ipma" => &["kind", "targets"],
            "cbsem" => &[
                "kind",
                "model_type",
                "estimator",
                "input",
                "mean_structure",
                "bootstrap_samples",
                "group_column",
                "invariance_steps",
            ],
            "pca" => &["kind", "variables", "retention"],
            "regression" => &[
                "kind",
                "outcome",
                "predictors",
                "controls",
                "model",
                "bootstrap",
            ],
            "nca" => &[
                "kind",
                "condition",
                "outcome",
                "ceiling",
                "permutation_samples",
            ],
            _ => &[],
        };
        if let Some(unknown) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
            return Err(D::Error::custom(format!(
                "unknown method_config field `{unknown}` for kind `{kind}`"
            )));
        }
        serde_json::from_value::<MethodConfigWire>(value)
            .map(Into::into)
            .map_err(D::Error::custom)
    }
}

impl MethodConfig {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::PlsAlgorithm => "pls_algorithm",
            Self::PlsBootstrap => "pls_bootstrap",
            Self::PlsPermutation => "pls_permutation",
            Self::Plsc => "plsc",
            Self::Wpls => "wpls",
            Self::Cca => "cca",
            Self::CtaPls => "cta_pls",
            Self::Endogeneity => "endogeneity",
            Self::NonlinearEffects => "nonlinear_effects",
            Self::ModeratedMediation => "moderated_mediation",
            Self::Predict { .. } => "predict",
            Self::Mga { .. } => "mga",
            Self::Ipma { .. } => "ipma",
            Self::Cbsem { .. } => "cbsem",
            Self::Pca { .. } => "pca",
            Self::Gsca => "gsca",
            Self::Regression { .. } => "regression",
            Self::Nca { .. } => "nca",
            Self::Legacy => "legacy",
        }
    }

    pub const fn supports_method(&self, method: AnalysisMethod) -> bool {
        match self {
            Self::PlsAlgorithm | Self::PlsBootstrap | Self::PlsPermutation => {
                matches!(method, AnalysisMethod::PlsPm)
            }
            Self::Plsc => matches!(method, AnalysisMethod::Plsc),
            Self::Wpls => matches!(method, AnalysisMethod::Wpls),
            Self::Cca => matches!(method, AnalysisMethod::Cca),
            Self::CtaPls => matches!(method, AnalysisMethod::CtaPls),
            Self::Endogeneity => matches!(method, AnalysisMethod::Endogeneity),
            Self::NonlinearEffects => matches!(method, AnalysisMethod::NonlinearEffects),
            Self::ModeratedMediation => matches!(method, AnalysisMethod::ModeratedMediation),
            Self::Predict { .. } => matches!(method, AnalysisMethod::Predict),
            Self::Mga { .. } => matches!(method, AnalysisMethod::Mga),
            Self::Ipma { .. } => matches!(method, AnalysisMethod::Ipma),
            Self::Cbsem { .. } => matches!(method, AnalysisMethod::Cbsem),
            Self::Pca { .. } => matches!(method, AnalysisMethod::Pca),
            Self::Gsca => matches!(method, AnalysisMethod::Gsca),
            Self::Regression { .. } => matches!(method, AnalysisMethod::Regression),
            Self::Nca { .. } => matches!(method, AnalysisMethod::Nca),
            Self::Legacy => matches!(method, AnalysisMethod::Legacy),
        }
    }

    pub fn default_for_settings(settings: &AnalysisSettings) -> Self {
        match settings.method {
            AnalysisMethod::PlsPm if settings.bootstrap_samples > 0 => Self::PlsBootstrap,
            AnalysisMethod::PlsPm if settings.permutation_samples > 0 => Self::PlsPermutation,
            AnalysisMethod::PlsPm => Self::PlsAlgorithm,
            AnalysisMethod::Bootstrap => Self::PlsBootstrap,
            AnalysisMethod::Plsc => Self::Plsc,
            AnalysisMethod::Wpls => Self::Wpls,
            AnalysisMethod::Cca => Self::Cca,
            AnalysisMethod::CtaPls => Self::CtaPls,
            AnalysisMethod::Endogeneity => Self::Endogeneity,
            AnalysisMethod::NonlinearEffects => Self::NonlinearEffects,
            AnalysisMethod::ModeratedMediation => Self::ModeratedMediation,
            AnalysisMethod::Predict => Self::Predict {
                pls_pos: None,
                fimix: None,
            },
            AnalysisMethod::Mga => Self::Mga {
                group_column: String::new(),
                group_a: String::new(),
                group_b: String::new(),
                methods: vec![
                    GroupAnalysisMethod::Micom,
                    GroupAnalysisMethod::MgaPermutation,
                ],
                permutation_samples: 5_000,
                configural_invariance_confirmed: false,
            },
            AnalysisMethod::Ipma => Self::Ipma {
                targets: Vec::new(),
            },
            AnalysisMethod::Cbsem => Self::Cbsem {
                model_type: CbsemModelType::Sem,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Raw,
                mean_structure: false,
                bootstrap_samples: 0,
                group_column: None,
                invariance_steps: Vec::new(),
            },
            AnalysisMethod::Pca => Self::Pca {
                variables: Vec::new(),
                retention: PcaRetentionConfig::Kaiser,
            },
            AnalysisMethod::Gsca => Self::Gsca,
            AnalysisMethod::Regression => Self::Regression {
                outcome: String::new(),
                predictors: Vec::new(),
                controls: Vec::new(),
                model: RegressionModelConfig::Ols {
                    robust_se: RobustStandardError::Hc3,
                },
                bootstrap: None,
            },
            AnalysisMethod::Nca => Self::Nca {
                condition: String::new(),
                outcome: String::new(),
                ceiling: NcaCeiling::Both,
                permutation_samples: 999,
            },
            AnalysisMethod::Legacy => Self::Legacy,
        }
    }

    /// Compatibility projection for engines that have not yet moved from the
    /// v2 metadata keys. Schema-v3 validation uses this projection only after
    /// rejecting executable keys supplied by the caller.
    fn legacy_metadata_projection(&self) -> BTreeMap<String, String> {
        let mut metadata = BTreeMap::new();
        match self {
            Self::Predict { pls_pos, fimix } => {
                if let Some(config) = pls_pos {
                    metadata.insert("segment_count".into(), config.segments.to_string());
                    metadata.insert("segment_starts".into(), config.starts.to_string());
                    metadata.insert(
                        "minimum_segment_share".into(),
                        config.minimum_segment_share.to_string(),
                    );
                }
                if let Some(config) = fimix {
                    metadata.insert("group_methods".into(), "fimix".into());
                    metadata.insert("fimix_classes".into(), config.segments.to_string());
                    metadata.insert("segment_starts".into(), config.starts.to_string());
                    metadata.insert(
                        "minimum_segment_share".into(),
                        config.minimum_segment_share.to_string(),
                    );
                }
            }
            Self::Mga {
                group_column,
                group_a,
                group_b,
                methods,
                permutation_samples,
                configural_invariance_confirmed,
            } => {
                metadata.insert("mga_group_column".into(), group_column.clone());
                metadata.insert("mga_group_a".into(), group_a.clone());
                metadata.insert("mga_group_b".into(), group_b.clone());
                metadata.insert(
                    "group_methods".into(),
                    methods
                        .iter()
                        .map(|method| method.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                );
                metadata.insert(
                    "group_permutation_samples".into(),
                    permutation_samples.to_string(),
                );
                metadata.insert(
                    "micom_configural_confirmed".into(),
                    configural_invariance_confirmed.to_string(),
                );
            }
            Self::Ipma { targets } => {
                metadata.insert("ipma_targets".into(), targets.join(","));
            }
            Self::Cbsem {
                model_type,
                estimator,
                input,
                mean_structure,
                bootstrap_samples,
                group_column,
                invariance_steps,
            } => {
                metadata.insert(
                    "cbsem_model_type".into(),
                    match model_type {
                        CbsemModelType::Cfa => "cfa",
                        CbsemModelType::Sem => "sem",
                    }
                    .into(),
                );
                metadata.insert(
                    "cbsem_estimator".into(),
                    match estimator {
                        CbsemEstimator::Ml => "ml",
                        CbsemEstimator::RobustMl => "robust_ml",
                        CbsemEstimator::Wlsmv => "wlsmv",
                    }
                    .into(),
                );
                metadata.insert(
                    "cbsem_input".into(),
                    match input {
                        CbsemInput::Raw => "raw",
                        CbsemInput::Covariance => "covariance",
                        CbsemInput::Correlation => "correlation",
                    }
                    .into(),
                );
                metadata.insert("cbsem_mean_structure".into(), mean_structure.to_string());
                if *bootstrap_samples > 0 {
                    metadata.insert(
                        "cbsem_bootstrap_samples".into(),
                        bootstrap_samples.to_string(),
                    );
                }
                if let Some(column) = group_column {
                    metadata.insert("cbsem_group_column".into(), column.clone());
                }
                if !invariance_steps.is_empty() {
                    metadata.insert(
                        "cbsem_invariance_steps".into(),
                        invariance_steps
                            .iter()
                            .map(|step| step.as_str())
                            .collect::<Vec<_>>()
                            .join(","),
                    );
                }
            }
            Self::Pca {
                variables,
                retention,
            } => {
                metadata.insert("pca_variables".into(), variables.join(","));
                match retention {
                    PcaRetentionConfig::Kaiser => {
                        metadata.insert("pca_component_rule".into(), "kaiser".into());
                    }
                    PcaRetentionConfig::Fixed { components } => {
                        metadata.insert("pca_component_rule".into(), "fixed".into());
                        metadata.insert("pca_components".into(), components.to_string());
                    }
                    PcaRetentionConfig::VarianceThreshold { threshold } => {
                        metadata.insert("pca_component_rule".into(), "variance_threshold".into());
                        metadata.insert("pca_variance_threshold".into(), threshold.to_string());
                    }
                }
            }
            Self::Regression {
                outcome,
                predictors,
                controls,
                model,
                ..
            } => {
                metadata.insert("regression_type".into(), model.wire_name().into());
                metadata.insert("regression_outcome".into(), outcome.clone());
                metadata.insert("regression_predictors".into(), predictors.join(","));
                if !controls.is_empty() {
                    metadata.insert("regression_controls".into(), controls.join(","));
                }
                match model {
                    RegressionModelConfig::Ols { robust_se } => {
                        metadata.insert("robust_se".into(), robust_se.as_str().into());
                    }
                    RegressionModelConfig::Logistic => {}
                    RegressionModelConfig::Process { relationship } => {
                        metadata.insert("process_model".into(), relationship.wire_name().into());
                        match relationship {
                            ProcessRelationshipConfig::Mediation { x, mediator } => {
                                metadata.insert("process_x".into(), x.clone());
                                metadata.insert("process_m".into(), mediator.clone());
                            }
                            ProcessRelationshipConfig::Moderation { x, moderator } => {
                                metadata.insert("process_x".into(), x.clone());
                                metadata.insert("process_w".into(), moderator.clone());
                            }
                            ProcessRelationshipConfig::ModeratedMediation {
                                x,
                                mediator,
                                moderator,
                            } => {
                                metadata.insert("process_x".into(), x.clone());
                                metadata.insert("process_m".into(), mediator.clone());
                                metadata.insert("process_w".into(), moderator.clone());
                            }
                            ProcessRelationshipConfig::Graph { .. } => {
                                // Graph PROCESS v2 is consumed directly from the typed
                                // capability. It is intentionally not projected into the
                                // lossy v1 metadata keys.
                            }
                        }
                    }
                }
            }
            Self::Nca {
                condition,
                outcome,
                ceiling,
                permutation_samples,
            } => {
                metadata.insert("nca_x".into(), condition.clone());
                metadata.insert("nca_y".into(), outcome.clone());
                metadata.insert("nca_ceiling".into(), ceiling.as_str().into());
                metadata.insert(
                    "nca_permutation_samples".into(),
                    permutation_samples.to_string(),
                );
            }
            Self::PlsAlgorithm
            | Self::PlsBootstrap
            | Self::PlsPermutation
            | Self::Plsc
            | Self::Wpls
            | Self::Cca
            | Self::CtaPls
            | Self::Endogeneity
            | Self::NonlinearEffects
            | Self::ModeratedMediation
            | Self::Gsca
            | Self::Legacy => {}
        }
        metadata
    }
}

pub const EXECUTABLE_LEGACY_METADATA_KEYS: &[&str] = &[
    "segment_count",
    "segmentation.segment_count",
    "groups.segment_count",
    "pls_pos_segments",
    "segmentation.pls_pos_segments",
    "segment_starts",
    "segmentation.segment_starts",
    "groups.segment_starts",
    "minimum_segment_share",
    "segmentation.minimum_segment_share",
    "groups.minimum_segment_share",
    "fimix_classes",
    "segmentation.fimix_classes",
    "groups.fimix_classes",
    "mga_group_column",
    "mga.group_column",
    "mga_group_a",
    "mga.group_a",
    "mga_group_b",
    "mga.group_b",
    "group_methods",
    "group_permutation_samples",
    "micom_configural_confirmed",
    "ipma_targets",
    "ipma.targets",
    "cbsem_model_type",
    "cbsem.model_type",
    "cbsem_estimator",
    "cbsem.estimator",
    "cbsem_input",
    "cbsem.input",
    "cbsem_mean_structure",
    "cbsem.mean_structure",
    "cbsem_bootstrap_samples",
    "cbsem_group_column",
    "cbsem_invariance_steps",
    "pca_variables",
    "pca.variables",
    "pca_component_rule",
    "pca_components",
    "pca_variance_threshold",
    "regression_type",
    "regression_outcome",
    "regression_predictors",
    "regression.predictors",
    "regression_controls",
    "regression.controls",
    "robust_se",
    "process_model",
    "process_x",
    "process_m",
    "process_w",
    "nca_x",
    "nca_y",
    "nca_ceiling",
    "nca_permutation_samples",
];

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyRecipeNormalization {
    /// Canonical schema-v3 settings. The historical `bootstrap` method alias
    /// is normalized to `pls_pm`; the source recipe remains unchanged.
    pub settings: AnalysisSettings,
    pub method_config: MethodConfig,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AnalysisRecipeMigrationError {
    #[error("only analysis recipe schema v2 can be migrated to v3 (found {0})")]
    UnsupportedSourceSchema(u32),
    #[error("schema-v2 recipe already contains method_config and is ambiguous")]
    UnexpectedMethodConfig,
    #[error("PLS schema-v2 recipe enables bootstrap and permutation together")]
    AmbiguousPlsInference,
    #[error("schema-v2 bootstrap method alias requires bootstrap_samples greater than zero")]
    BootstrapAliasWithoutSamples,
    #[error("missing executable metadata key {0}")]
    MissingMetadata(&'static str),
    #[error("conflicting legacy metadata aliases {left} and {right}")]
    ConflictingAliases {
        left: &'static str,
        right: &'static str,
    },
    #[error("invalid value for legacy metadata key {key}: {value}")]
    InvalidMetadata { key: &'static str, value: String },
    #[error("legacy executable metadata key {key} is not valid for method {method}")]
    MetadataMethodMismatch { key: String, method: AnalysisMethod },
    #[error(
        "historical PROCESS v1 recipes are archive-readable only and cannot be migrated into executable schema-v3 recipes; author a graph-defined PROCESS v2 recipe"
    )]
    ArchiveOnlyProcessV1,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("method_config kind {config_kind} is incompatible with settings.method {method}")]
pub struct MethodConfigMismatchError {
    pub config_kind: &'static str,
    pub method: AnalysisMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AnalysisRecipeConstructionError {
    #[error(
        "analysis method {0} requires an explicit method_config; use AnalysisRecipe::new_with_method_config"
    )]
    ExplicitMethodConfigRequired(AnalysisMethod),
    #[error("PLS studentized bootstrap requires bootstrap_samples greater than zero")]
    StudentizedBootstrapWithoutBootstrap,
    #[error("the historical bootstrap method alias requires bootstrap_samples greater than zero")]
    BootstrapAliasWithoutSamples,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EffectiveRecipeMetadataError {
    #[error("analysis recipe schema {0} cannot provide executable metadata")]
    UnsupportedSchema(u32),
    #[error("historical analysis recipe schema {0} unexpectedly contains method_config")]
    HistoricalMethodConfig(u32),
    #[error("analysis recipe schema v3 requires method_config")]
    MissingMethodConfig,
    #[error("method_config kind {config_kind} is incompatible with settings.method {method}")]
    MethodMismatch {
        config_kind: &'static str,
        method: AnalysisMethod,
    },
    #[error("schema-v3 metadata contains executable keys also owned by method_config: {keys:?}")]
    ExecutableMetadataConflict { keys: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisRecipe {
    pub schema_version: u32,
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub dataset_fingerprint: String,
    pub model: ModelSpec,
    pub settings: AnalysisSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_config: Option<MethodConfig>,
    #[serde(default)]
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

    fn fixture_model() -> ModelSpec {
        ModelSpec {
            id: Uuid::nil(),
            name: "Fixture".into(),
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

    fn fixture_recipe(
        schema_version: u32,
        settings: AnalysisSettings,
        metadata: BTreeMap<String, String>,
    ) -> AnalysisRecipe {
        AnalysisRecipe {
            schema_version,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: "fixture".into(),
            model: fixture_model(),
            settings,
            method_config: None,
            metadata,
        }
    }

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

    #[test]
    fn method_config_native_kind_wire_names_are_stable() {
        let configs = vec![
            MethodConfig::PlsAlgorithm,
            MethodConfig::PlsBootstrap,
            MethodConfig::PlsPermutation,
            MethodConfig::Plsc,
            MethodConfig::Wpls,
            MethodConfig::Cca,
            MethodConfig::CtaPls,
            MethodConfig::Endogeneity,
            MethodConfig::NonlinearEffects,
            MethodConfig::ModeratedMediation,
            MethodConfig::Predict {
                pls_pos: None,
                fimix: None,
            },
            MethodConfig::Mga {
                group_column: "group".into(),
                group_a: "A".into(),
                group_b: "B".into(),
                methods: vec![
                    GroupAnalysisMethod::Micom,
                    GroupAnalysisMethod::MgaPermutation,
                ],
                permutation_samples: 5_000,
                configural_invariance_confirmed: true,
            },
            MethodConfig::Ipma {
                targets: vec!["y".into()],
            },
            MethodConfig::Cbsem {
                model_type: CbsemModelType::Sem,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Raw,
                mean_structure: false,
                bootstrap_samples: 0,
                group_column: None,
                invariance_steps: Vec::new(),
            },
            MethodConfig::Pca {
                variables: vec!["x1".into(), "x2".into()],
                retention: PcaRetentionConfig::Kaiser,
            },
            MethodConfig::Gsca,
            MethodConfig::Regression {
                outcome: "y".into(),
                predictors: vec!["x".into()],
                controls: Vec::new(),
                model: RegressionModelConfig::Ols {
                    robust_se: RobustStandardError::Hc3,
                },
                bootstrap: None,
            },
            MethodConfig::Nca {
                condition: "x".into(),
                outcome: "y".into(),
                ceiling: NcaCeiling::Both,
                permutation_samples: 999,
            },
            MethodConfig::Legacy,
        ];
        let expected = [
            "pls_algorithm",
            "pls_bootstrap",
            "pls_permutation",
            "plsc",
            "wpls",
            "cca",
            "cta_pls",
            "endogeneity",
            "nonlinear_effects",
            "moderated_mediation",
            "predict",
            "mga",
            "ipma",
            "cbsem",
            "pca",
            "gsca",
            "regression",
            "nca",
            "legacy",
        ]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<Vec<String>>();
        assert_eq!(
            configs
                .iter()
                .map(|config| serde_json::to_value(config).unwrap()["kind"]
                    .as_str()
                    .unwrap()
                    .to_owned())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            configs
                .iter()
                .map(MethodConfig::kind)
                .map(ToOwned::to_owned)
                .collect::<Vec<String>>(),
            expected
        );
    }

    #[test]
    fn regression_bootstrap_and_method_config_nested_wire_tags_are_stable() {
        assert_eq!(
            serde_json::to_value(MethodConfig::Pca {
                variables: vec!["x1".into(), "x2".into()],
                retention: PcaRetentionConfig::Fixed { components: 2 },
            })
            .unwrap(),
            serde_json::json!({
                "kind": "pca",
                "variables": ["x1", "x2"],
                "retention": {"rule": "fixed", "components": 2}
            })
        );
        assert_eq!(
            serde_json::to_value(MethodConfig::Regression {
                outcome: "y".into(),
                predictors: vec!["x".into()],
                controls: Vec::new(),
                model: RegressionModelConfig::Process {
                    relationship: ProcessRelationshipConfig::ModeratedMediation {
                        x: "x".into(),
                        mediator: "m".into(),
                        moderator: "w".into(),
                    },
                },
                bootstrap: None,
            })
            .unwrap(),
            serde_json::json!({
                "kind": "regression",
                "outcome": "y",
                "predictors": ["x"],
                "model": {
                    "type": "process",
                    "relationship": {
                        "model": "moderated_mediation",
                        "x": "x",
                        "mediator": "m",
                        "moderator": "w"
                    }
                }
            })
        );
        assert_eq!(
            serde_json::to_value(MethodConfig::Regression {
                outcome: "y".into(),
                predictors: vec!["x".into(), "m".into(), "w".into(), "b".into()],
                controls: vec!["c".into()],
                model: RegressionModelConfig::Process {
                    relationship: ProcessRelationshipConfig::Graph {
                        focal_predictor: "x".into(),
                        paths: vec![
                            ProcessPathConfig {
                                from: "x".into(),
                                to: "m".into()
                            },
                            ProcessPathConfig {
                                from: "m".into(),
                                to: "y".into()
                            },
                            ProcessPathConfig {
                                from: "x".into(),
                                to: "y".into()
                            },
                        ],
                        moderators: vec![
                            ProcessModeratorConfig {
                                variable: "w".into(),
                                scale: ProcessModeratorScale::Continuous
                            },
                            ProcessModeratorConfig {
                                variable: "b".into(),
                                scale: ProcessModeratorScale::Binary01
                            },
                        ],
                        moderations: vec![ProcessModerationConfig {
                            from: "x".into(),
                            to: "y".into(),
                            moderator: "w".into(),
                            conditioning_moderator: Some("b".into()),
                        }],
                        continuous_product_centering:
                            ProcessContinuousCentering::EquationCompleteCaseMeanV1,
                    },
                },
                bootstrap: Some(RegressionBootstrapConfig {
                    algorithm: RegressionBootstrapAlgorithm::CaseResampling,
                    intervals: vec![
                        RegressionBootstrapInterval::Percentile,
                        RegressionBootstrapInterval::Bca
                    ],
                }),
            })
            .unwrap(),
            serde_json::json!({
                "kind": "regression",
                "outcome": "y",
                "predictors": ["x", "m", "w", "b"],
                "controls": ["c"],
                "model": {
                    "type": "process",
                    "relationship": {
                        "model": "graph",
                        "focal_predictor": "x",
                        "paths": [
                            {"from": "x", "to": "m"},
                            {"from": "m", "to": "y"},
                            {"from": "x", "to": "y"}
                        ],
                        "moderators": [
                            {"variable": "w", "scale": "continuous"},
                            {"variable": "b", "scale": "binary_0_1"}
                        ],
                        "moderations": [{
                            "from": "x",
                            "to": "y",
                            "moderator": "w",
                            "conditioning_moderator": "b"
                        }],
                        "continuous_product_centering": "equation_complete_case_mean_v1"
                    }
                },
                "bootstrap": {
                    "algorithm": "case_resampling",
                    "intervals": ["percentile", "bca"]
                }
            })
        );
        assert_eq!(
            serde_json::to_value(MethodConfig::Cbsem {
                model_type: CbsemModelType::Sem,
                estimator: CbsemEstimator::RobustMl,
                input: CbsemInput::Covariance,
                mean_structure: true,
                bootstrap_samples: 999,
                group_column: Some("group".into()),
                invariance_steps: vec![
                    CbsemInvarianceStep::Configural,
                    CbsemInvarianceStep::Metric
                ],
            })
            .unwrap(),
            serde_json::json!({
                "kind": "cbsem",
                "model_type": "sem",
                "estimator": "robust_ml",
                "input": "covariance",
                "mean_structure": true,
                "bootstrap_samples": 999,
                "group_column": "group",
                "invariance_steps": ["configural", "metric"]
            })
        );

        let regression_bootstrap = MethodConfig::Regression {
            outcome: "y".into(),
            predictors: vec!["x".into()],
            controls: Vec::new(),
            model: RegressionModelConfig::Logistic,
            bootstrap: Some(RegressionBootstrapConfig {
                algorithm: RegressionBootstrapAlgorithm::CaseResampling,
                intervals: vec![
                    RegressionBootstrapInterval::Percentile,
                    RegressionBootstrapInterval::Bca,
                ],
            }),
        };
        let encoded = serde_json::to_value(&regression_bootstrap).unwrap();
        assert_eq!(
            encoded["bootstrap"],
            serde_json::json!({
                "algorithm": "case_resampling",
                "intervals": ["percentile", "bca"]
            })
        );
        assert_eq!(
            serde_json::from_value::<MethodConfig>(encoded).unwrap(),
            regression_bootstrap
        );
        assert!(
            serde_json::from_value::<MethodConfig>(serde_json::json!({
                "kind": "regression",
                "outcome": "y",
                "predictors": ["x"],
                "model": {"type": "logistic"},
                "bootstrap": {
                    "algorithm": "case_resampling",
                    "intervals": ["percentile", "bca"],
                    "studentized": true
                }
            }))
            .is_err()
        );
    }

    #[test]
    fn method_config_deserialization_rejects_unknown_and_misspelled_fields() {
        let invalid = [
            serde_json::json!({
                "kind": "pls_algorithm",
                "weighting_sheme": "path"
            }),
            serde_json::json!({
                "kind": "predict",
                "pls_pos": {
                    "segments": 2,
                    "starts": 10,
                    "minimum_segment_share": 0.10,
                    "minimum_segment_shrae": 0.10
                }
            }),
            serde_json::json!({
                "kind": "mga",
                "group_column": "group",
                "group_a": "A",
                "group_b": "B",
                "methods": ["micom", "mga_permutation"],
                "permutation_samples": 5000,
                "configural_invariance_confirmed": true,
                "permutaton_samples": 5000
            }),
            serde_json::json!({
                "kind": "cbsem",
                "model_type": "sem",
                "estimator": "ml",
                "input": "raw",
                "mean_structure": false,
                "bootstrap_samples": 0,
                "estimatr": "ml"
            }),
            serde_json::json!({
                "kind": "pca",
                "variables": ["x1", "x2"],
                "retention": {
                    "rule": "fixed",
                    "components": 2,
                    "componentz": 2
                }
            }),
            serde_json::json!({
                "kind": "regression",
                "outcome": "y",
                "predictors": ["x"],
                "model": {
                    "type": "ols",
                    "robust_se": "hc3",
                    "robust_see": "hc3"
                }
            }),
            serde_json::json!({
                "kind": "regression",
                "outcome": "y",
                "predictors": ["x"],
                "model": {
                    "type": "process",
                    "relationship": {
                        "model": "mediation",
                        "x": "x",
                        "mediator": "m",
                        "medaitor": "m"
                    }
                }
            }),
            serde_json::json!({"kind": "pls_algoritm"}),
            serde_json::json!({
                "kind": "mga",
                "group_column": "group",
                "group_a": "A",
                "group_b": "B",
                "methods": ["micomm"],
                "permutation_samples": 5000,
                "configural_invariance_confirmed": true
            }),
        ];

        for payload in invalid {
            assert!(
                serde_json::from_value::<MethodConfig>(payload.clone()).is_err(),
                "misspelled method_config unexpectedly deserialized: {payload}"
            );
        }
    }

    #[test]
    fn cbsem_method_config_requires_an_explicit_bootstrap_choice() {
        let missing_bootstrap_samples = serde_json::json!({
            "kind": "cbsem",
            "model_type": "sem",
            "estimator": "ml",
            "input": "raw",
            "mean_structure": false
        });
        let error = serde_json::from_value::<MethodConfig>(missing_bootstrap_samples)
            .expect_err("CB-SEM must distinguish an omitted bootstrap choice from explicit zero");
        assert!(
            error.to_string().contains("bootstrap_samples"),
            "unexpected CB-SEM deserialization error: {error}"
        );

        let explicit_zero = serde_json::json!({
            "kind": "cbsem",
            "model_type": "sem",
            "estimator": "ml",
            "input": "raw",
            "mean_structure": false,
            "bootstrap_samples": 0
        });
        assert!(serde_json::from_value::<MethodConfig>(explicit_zero).is_ok());
    }

    #[test]
    fn schema_v3_serializes_typed_config_and_keeps_non_executable_metadata() {
        let mut recipe = AnalysisRecipe::new(b"data", fixture_model(), AnalysisSettings::default());
        recipe.metadata.insert("status".into(), "validated".into());
        recipe.metadata.insert("demo".into(), "fixture".into());
        let encoded = serde_json::to_value(&recipe).unwrap();
        assert_eq!(encoded["schema_version"], ANALYSIS_RECIPE_SCHEMA_VERSION);
        assert_eq!(encoded["method_config"]["kind"], "pls_algorithm");
        assert_eq!(encoded["metadata"]["status"], "validated");
        assert_eq!(encoded["metadata"]["demo"], "fixture");
        assert_eq!(
            serde_json::from_value::<AnalysisRecipe>(encoded).unwrap(),
            recipe
        );
        assert_eq!(recipe.effective_metadata().unwrap(), recipe.metadata);
    }

    #[test]
    fn analysis_recipe_new_requires_explicit_configs_for_parameterized_methods() {
        for method in [
            AnalysisMethod::Mga,
            AnalysisMethod::Pca,
            AnalysisMethod::Regression,
            AnalysisMethod::Nca,
        ] {
            let mut settings = AnalysisSettings::default();
            settings.method = method;
            assert_eq!(
                AnalysisRecipe::try_new(b"data", fixture_model(), settings).unwrap_err(),
                AnalysisRecipeConstructionError::ExplicitMethodConfigRequired(method)
            );
        }

        let mut pca_settings = AnalysisSettings::default();
        pca_settings.method = AnalysisMethod::Pca;
        let explicit = AnalysisRecipe::new_with_method_config(
            b"data",
            fixture_model(),
            pca_settings.clone(),
            MethodConfig::Pca {
                variables: vec!["x1".into(), "x2".into()],
                retention: PcaRetentionConfig::Kaiser,
            },
        )
        .unwrap();
        assert!(matches!(
            explicit.method_config,
            Some(MethodConfig::Pca { .. })
        ));

        assert!(
            std::panic::catch_unwind(|| {
                AnalysisRecipe::new(b"data", fixture_model(), pca_settings)
            })
            .is_err(),
            "the compatibility constructor must fail loudly instead of inventing an empty PCA config"
        );
    }

    #[test]
    fn analysis_recipe_try_new_canonicalizes_bootstrap_and_rejects_incomplete_resampling() {
        let mut alias = AnalysisSettings::default();
        alias.method = AnalysisMethod::Bootstrap;
        alias.bootstrap_samples = 999;
        let recipe = AnalysisRecipe::try_new(b"data", fixture_model(), alias).unwrap();
        assert_eq!(recipe.settings.method, AnalysisMethod::PlsPm);
        assert_eq!(recipe.method_config, Some(MethodConfig::PlsBootstrap));

        let mut empty_alias = AnalysisSettings::default();
        empty_alias.method = AnalysisMethod::Bootstrap;
        assert_eq!(
            AnalysisRecipe::try_new(b"data", fixture_model(), empty_alias).unwrap_err(),
            AnalysisRecipeConstructionError::BootstrapAliasWithoutSamples
        );

        let mut orphaned_studentized = AnalysisSettings::default();
        orphaned_studentized.studentized_inner_samples = 99;
        assert_eq!(
            AnalysisRecipe::try_new(b"data", fixture_model(), orphaned_studentized).unwrap_err(),
            AnalysisRecipeConstructionError::StudentizedBootstrapWithoutBootstrap
        );
    }

    #[test]
    fn historical_v1_and_v2_deserialize_without_silent_method_config() {
        for version in [1, 2] {
            let source = fixture_recipe(version, AnalysisSettings::default(), BTreeMap::new());
            let encoded = serde_json::to_value(&source).unwrap();
            assert!(encoded.get("method_config").is_none());
            let decoded: AnalysisRecipe = serde_json::from_value(encoded).unwrap();
            assert_eq!(decoded.method_config, None);
            assert_eq!(decoded.effective_metadata().unwrap(), decoded.metadata);
            assert_eq!(
                decoded.schema_generation(),
                if version == 1 {
                    AnalysisRecipeSchemaGeneration::V1
                } else {
                    AnalysisRecipeSchemaGeneration::V2
                }
            );
        }
        let v1 = fixture_recipe(1, AnalysisSettings::default(), BTreeMap::new());
        assert_eq!(
            v1.migrated_v3().unwrap_err(),
            AnalysisRecipeMigrationError::UnsupportedSourceSchema(1)
        );
        let mut ambiguous_historical = v1;
        ambiguous_historical.method_config = Some(MethodConfig::PlsAlgorithm);
        assert_eq!(
            ambiguous_historical.effective_metadata().unwrap_err(),
            EffectiveRecipeMetadataError::HistoricalMethodConfig(1)
        );
        assert!(
            crate::validate_recipe(&ambiguous_historical)
                .iter()
                .any(|issue| issue.code == "method_config.historical_unexpected")
        );
    }

    #[test]
    fn legacy_bootstrap_alias_migrates_to_executable_pls_pm_without_mutating_source() {
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Bootstrap;
        settings.bootstrap_samples = 999;
        let source = fixture_recipe(
            2,
            settings,
            BTreeMap::from([("status".into(), "historical_bootstrap".into())]),
        );

        let normalized = source.normalize_legacy_v2().unwrap();
        assert_eq!(source.settings.method, AnalysisMethod::Bootstrap);
        assert_eq!(source.schema_version, 2);
        assert_eq!(source.method_config, None);
        assert_eq!(normalized.settings.method, AnalysisMethod::PlsPm);
        assert_eq!(normalized.method_config, MethodConfig::PlsBootstrap);

        let migrated = source.migrated_v3().unwrap();
        assert_eq!(migrated.id, source.id);
        assert_eq!(migrated.settings.method, AnalysisMethod::PlsPm);
        assert_eq!(migrated.method_config, Some(MethodConfig::PlsBootstrap));
        assert_eq!(migrated.metadata["status"], "historical_bootstrap");
        assert!(
            crate::validate_recipe(&migrated)
                .iter()
                .all(|issue| issue.severity != crate::Severity::Error)
        );

        let fresh = source.migrated_v3_with_fresh_id().unwrap();
        assert_ne!(fresh.id, source.id);
        let mut identity_normalized = fresh;
        identity_normalized.id = source.id;
        assert_eq!(identity_normalized, migrated);
        assert_eq!(source.settings.method, AnalysisMethod::Bootstrap);

        let mut empty_alias = source.clone();
        empty_alias.settings.bootstrap_samples = 0;
        assert_eq!(
            empty_alias.migrated_v3().unwrap_err(),
            AnalysisRecipeMigrationError::BootstrapAliasWithoutSamples
        );
        let mut ambiguous_alias = source;
        ambiguous_alias.settings.permutation_samples = 99;
        assert_eq!(
            ambiguous_alias.migrated_v3().unwrap_err(),
            AnalysisRecipeMigrationError::AmbiguousPlsInference
        );
    }

    #[test]
    fn explicit_v2_regression_migration_is_deterministic_and_preserves_annotations() {
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Regression;
        let source = fixture_recipe(
            2,
            settings,
            BTreeMap::from([
                (
                    "status".into(),
                    "validated_regression_ols_v1_bounded_scope".into(),
                ),
                ("demo".into(), "fixture".into()),
                ("regression_type".into(), "ols".into()),
                ("regression_outcome".into(), "y".into()),
                ("regression_predictors".into(), " x, m ".into()),
                ("regression_controls".into(), "z".into()),
                ("robust_se".into(), "hc3".into()),
            ]),
        );
        let migrated = source.migrated_v3().unwrap();
        assert_eq!(source.schema_version, 2);
        assert!(source.method_config.is_none());
        assert_eq!(migrated.schema_version, ANALYSIS_RECIPE_SCHEMA_VERSION);
        assert_eq!(
            migrated.method_config,
            Some(MethodConfig::Regression {
                outcome: "y".into(),
                predictors: vec!["x".into(), "m".into()],
                controls: vec!["z".into()],
                model: RegressionModelConfig::Ols {
                    robust_se: RobustStandardError::Hc3,
                },
                bootstrap: None,
            })
        );
        assert_eq!(
            migrated.metadata,
            BTreeMap::from([
                ("demo".into(), "fixture".into()),
                (
                    "status".into(),
                    "validated_regression_ols_v1_bounded_scope".into()
                ),
            ])
        );
        assert!(migrated.executable_legacy_metadata_keys().is_empty());
        let effective = migrated.effective_metadata().unwrap();
        assert_eq!(effective["regression_outcome"], "y");
        assert_eq!(effective["regression_predictors"], "x,m");
        assert_eq!(effective["robust_se"], "hc3");
        assert_eq!(
            effective["status"],
            "validated_regression_ols_v1_bounded_scope"
        );
        assert_eq!(source.migrated_v3().unwrap(), migrated);
    }

    #[test]
    fn historical_process_v1_migration_fails_closed_instead_of_creating_an_executable_recipe() {
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Regression;
        let source = fixture_recipe(
            LEGACY_ANALYSIS_RECIPE_SCHEMA_VERSION,
            settings,
            BTreeMap::from([
                ("regression_type".into(), "process".into()),
                ("regression_outcome".into(), "y".into()),
                ("regression_predictors".into(), "x,m".into()),
                ("process_model".into(), "mediation".into()),
                ("process_x".into(), "x".into()),
                ("process_m".into(), "m".into()),
            ]),
        );
        assert_eq!(
            source.migrated_v3().unwrap_err(),
            AnalysisRecipeMigrationError::ArchiveOnlyProcessV1
        );
        assert_eq!(source.schema_version, LEGACY_ANALYSIS_RECIPE_SCHEMA_VERSION);
        assert!(source.method_config.is_none());
    }

    #[test]
    fn v2_migration_rejects_alias_conflicts_and_cross_method_metadata() {
        let mut ipma_settings = AnalysisSettings::default();
        ipma_settings.method = AnalysisMethod::Ipma;
        let aliases = fixture_recipe(
            2,
            ipma_settings,
            BTreeMap::from([
                ("ipma_targets".into(), "y".into()),
                ("ipma.targets".into(), "x".into()),
            ]),
        );
        assert!(matches!(
            aliases.normalize_legacy_v2(),
            Err(AnalysisRecipeMigrationError::ConflictingAliases { .. })
        ));

        let mut plsc_settings = AnalysisSettings::default();
        plsc_settings.method = AnalysisMethod::Plsc;
        let mismatch = fixture_recipe(
            2,
            plsc_settings,
            BTreeMap::from([("nca_x".into(), "x".into())]),
        );
        assert!(matches!(
            mismatch.normalize_legacy_v2(),
            Err(AnalysisRecipeMigrationError::MetadataMethodMismatch { .. })
        ));

        let mut ambiguous_settings = AnalysisSettings::default();
        ambiguous_settings.bootstrap_samples = 999;
        ambiguous_settings.permutation_samples = 999;
        let ambiguous = fixture_recipe(2, ambiguous_settings, BTreeMap::new());
        assert_eq!(
            ambiguous.normalize_legacy_v2().unwrap_err(),
            AnalysisRecipeMigrationError::AmbiguousPlsInference
        );
    }

    #[test]
    fn schema_v3_validation_requires_matching_config_and_rejects_legacy_execution_keys() {
        let mut missing = fixture_recipe(
            ANALYSIS_RECIPE_SCHEMA_VERSION,
            AnalysisSettings::default(),
            BTreeMap::new(),
        );
        let missing_issues = crate::validate_recipe(&missing);
        assert!(
            missing_issues
                .iter()
                .any(|issue| issue.code == "method_config.required")
        );

        missing.method_config = Some(MethodConfig::Plsc);
        let mismatch_issues = crate::validate_recipe(&missing);
        assert!(
            mismatch_issues
                .iter()
                .any(|issue| issue.code == "method_config.method_mismatch")
        );
        assert_eq!(
            missing.effective_metadata().unwrap_err(),
            EffectiveRecipeMetadataError::MethodMismatch {
                config_kind: "plsc",
                method: AnalysisMethod::PlsPm,
            }
        );

        missing.method_config = Some(MethodConfig::PlsAlgorithm);
        missing
            .metadata
            .insert("regression_outcome".into(), "conflict".into());
        let conflict_issues = crate::validate_recipe(&missing);
        assert!(conflict_issues.iter().any(|issue| {
            issue.code == "method_config.legacy_metadata_conflict"
                && issue.subject.as_deref() == Some("regression_outcome")
        }));
        assert_eq!(
            missing.effective_metadata().unwrap_err(),
            EffectiveRecipeMetadataError::ExecutableMetadataConflict {
                keys: vec!["regression_outcome".into()],
            }
        );
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

    #[allow(clippy::too_many_arguments)]
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
    /// Compatibility convenience constructor for workflows with a complete,
    /// deterministic default configuration.
    ///
    /// Parameterized workflows that cannot be inferred without scientific
    /// user input fail loudly. New code that accepts a user-selected method
    /// should use [`Self::try_new`] or [`Self::new_with_method_config`].
    #[track_caller]
    pub fn new(dataset_bytes: &[u8], model: ModelSpec, settings: AnalysisSettings) -> Self {
        Self::try_new(dataset_bytes, model, settings)
            .unwrap_or_else(|error| panic!("cannot construct analysis recipe: {error}"))
    }

    /// Constructs a new schema-v3 recipe only when its method configuration
    /// can be inferred without inventing parameter values.
    pub fn try_new(
        dataset_bytes: &[u8],
        model: ModelSpec,
        mut settings: AnalysisSettings,
    ) -> Result<Self, AnalysisRecipeConstructionError> {
        if settings.method == AnalysisMethod::Bootstrap {
            if settings.bootstrap_samples == 0 {
                return Err(AnalysisRecipeConstructionError::BootstrapAliasWithoutSamples);
            }
            settings.method = AnalysisMethod::PlsPm;
        }
        if settings.method == AnalysisMethod::PlsPm
            && settings.studentized_inner_samples > 0
            && settings.bootstrap_samples == 0
        {
            return Err(AnalysisRecipeConstructionError::StudentizedBootstrapWithoutBootstrap);
        }

        let method_config = match settings.method {
            AnalysisMethod::Mga
            | AnalysisMethod::Pca
            | AnalysisMethod::Regression
            | AnalysisMethod::Nca => {
                return Err(
                    AnalysisRecipeConstructionError::ExplicitMethodConfigRequired(settings.method),
                );
            }
            AnalysisMethod::Cbsem => MethodConfig::Cbsem {
                model_type: if model.paths.is_empty() {
                    CbsemModelType::Cfa
                } else {
                    CbsemModelType::Sem
                },
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Raw,
                mean_structure: false,
                bootstrap_samples: 0,
                group_column: None,
                invariance_steps: Vec::new(),
            },
            _ => MethodConfig::default_for_settings(&settings),
        };

        Ok(Self {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: sha256_hex(dataset_bytes),
            model,
            settings,
            method_config: Some(method_config),
            metadata: BTreeMap::new(),
        })
    }

    pub fn new_with_method_config(
        dataset_bytes: &[u8],
        model: ModelSpec,
        settings: AnalysisSettings,
        method_config: MethodConfig,
    ) -> Result<Self, MethodConfigMismatchError> {
        if !method_config.supports_method(settings.method) {
            return Err(MethodConfigMismatchError {
                config_kind: method_config.kind(),
                method: settings.method,
            });
        }
        Ok(Self {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: sha256_hex(dataset_bytes),
            model,
            settings,
            method_config: Some(method_config),
            metadata: BTreeMap::new(),
        })
    }

    pub const fn schema_generation(&self) -> AnalysisRecipeSchemaGeneration {
        match self.schema_version {
            1 => AnalysisRecipeSchemaGeneration::V1,
            2 => AnalysisRecipeSchemaGeneration::V2,
            3 => AnalysisRecipeSchemaGeneration::V3,
            version => AnalysisRecipeSchemaGeneration::Future(version),
        }
    }

    /// Explicitly parses a historical v2 recipe without changing it. Callers
    /// can inspect the normalization before choosing to persist a v3 copy.
    pub fn normalize_legacy_v2(
        &self,
    ) -> Result<LegacyRecipeNormalization, AnalysisRecipeMigrationError> {
        if self.schema_version != LEGACY_ANALYSIS_RECIPE_SCHEMA_VERSION {
            return Err(AnalysisRecipeMigrationError::UnsupportedSourceSchema(
                self.schema_version,
            ));
        }
        if self.method_config.is_some() {
            return Err(AnalysisRecipeMigrationError::UnexpectedMethodConfig);
        }
        if matches!(
            self.settings.method,
            AnalysisMethod::PlsPm | AnalysisMethod::Bootstrap
        ) && self.settings.bootstrap_samples > 0
            && self.settings.permutation_samples > 0
        {
            return Err(AnalysisRecipeMigrationError::AmbiguousPlsInference);
        }
        if self.settings.method == AnalysisMethod::Bootstrap && self.settings.bootstrap_samples == 0
        {
            return Err(AnalysisRecipeMigrationError::BootstrapAliasWithoutSamples);
        }

        let mut settings = self.settings.clone();
        if settings.method == AnalysisMethod::Bootstrap {
            settings.method = AnalysisMethod::PlsPm;
        }

        let (method_config, allowed_keys): (MethodConfig, &[&str]) = match self.settings.method {
            AnalysisMethod::PlsPm => {
                let config = if self.settings.bootstrap_samples > 0 {
                    MethodConfig::PlsBootstrap
                } else if self.settings.permutation_samples > 0 {
                    MethodConfig::PlsPermutation
                } else {
                    MethodConfig::PlsAlgorithm
                };
                (config, &[])
            }
            AnalysisMethod::Bootstrap => (MethodConfig::PlsBootstrap, &[]),
            AnalysisMethod::Plsc => (MethodConfig::Plsc, &[]),
            AnalysisMethod::Wpls => (MethodConfig::Wpls, &[]),
            AnalysisMethod::Cca => (MethodConfig::Cca, &[]),
            AnalysisMethod::CtaPls => (MethodConfig::CtaPls, &[]),
            AnalysisMethod::Endogeneity => (MethodConfig::Endogeneity, &[]),
            AnalysisMethod::NonlinearEffects => (MethodConfig::NonlinearEffects, &[]),
            AnalysisMethod::ModeratedMediation => (MethodConfig::ModeratedMediation, &[]),
            AnalysisMethod::Predict => (
                legacy_predict_config(&self.metadata)?,
                PREDICT_LEGACY_METADATA_KEYS,
            ),
            AnalysisMethod::Mga => (legacy_mga_config(&self.metadata)?, MGA_LEGACY_METADATA_KEYS),
            AnalysisMethod::Ipma => (
                legacy_ipma_config(&self.metadata, &self.model)?,
                IPMA_LEGACY_METADATA_KEYS,
            ),
            AnalysisMethod::Cbsem => (
                legacy_cbsem_config(&self.metadata, &self.model)?,
                CBSEM_LEGACY_METADATA_KEYS,
            ),
            AnalysisMethod::Pca => (legacy_pca_config(&self.metadata)?, PCA_LEGACY_METADATA_KEYS),
            AnalysisMethod::Gsca => (MethodConfig::Gsca, &[]),
            AnalysisMethod::Regression => (
                legacy_regression_config(&self.metadata)?,
                REGRESSION_LEGACY_METADATA_KEYS,
            ),
            AnalysisMethod::Nca => (legacy_nca_config(&self.metadata)?, NCA_LEGACY_METADATA_KEYS),
            AnalysisMethod::Legacy => (MethodConfig::Legacy, &[]),
        };

        if let Some(key) = self.metadata.keys().find(|key| {
            is_executable_legacy_metadata_key(key) && !allowed_keys.contains(&key.as_str())
        }) {
            return Err(AnalysisRecipeMigrationError::MetadataMethodMismatch {
                key: key.clone(),
                method: self.settings.method,
            });
        }
        let metadata = self
            .metadata
            .iter()
            .filter(|(key, _)| !is_executable_legacy_metadata_key(key))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        Ok(LegacyRecipeNormalization {
            settings,
            method_config,
            metadata,
        })
    }

    /// Creates a v3 copy and leaves the historical v2 value untouched.
    pub fn migrated_v3(&self) -> Result<Self, AnalysisRecipeMigrationError> {
        let normalized = self.normalize_legacy_v2()?;
        let mut migrated = self.clone();
        migrated.schema_version = ANALYSIS_RECIPE_SCHEMA_VERSION;
        migrated.settings = normalized.settings;
        migrated.method_config = Some(normalized.method_config);
        migrated.metadata = normalized.metadata;
        Ok(migrated)
    }

    /// Creates an explicit schema-v3 copy with a fresh recipe identifier for a
    /// new execution/append. Deterministic [`Self::migrated_v3`] remains
    /// identity-preserving for inspection and archive transformation.
    pub fn migrated_v3_with_fresh_id(&self) -> Result<Self, AnalysisRecipeMigrationError> {
        let mut migrated = self.migrated_v3()?;
        migrated.id = Uuid::new_v4();
        Ok(migrated)
    }

    pub fn executable_legacy_metadata_keys(&self) -> Vec<&str> {
        self.metadata
            .keys()
            .filter(|key| is_executable_legacy_metadata_key(key))
            .map(String::as_str)
            .collect()
    }

    /// Returns the authoritative compatibility view for existing engines.
    ///
    /// V1/v2 return their original metadata only. V3 preserves non-executable
    /// annotations and adds an executable projection from `method_config`;
    /// caller-supplied executable keys are rejected, so there is no precedence
    /// ambiguity. Future/invalid schemas also fail closed.
    pub fn effective_metadata(
        &self,
    ) -> Result<BTreeMap<String, String>, EffectiveRecipeMetadataError> {
        match self.schema_generation() {
            AnalysisRecipeSchemaGeneration::V1 | AnalysisRecipeSchemaGeneration::V2 => {
                if self.method_config.is_some() {
                    return Err(EffectiveRecipeMetadataError::HistoricalMethodConfig(
                        self.schema_version,
                    ));
                }
                Ok(self.metadata.clone())
            }
            AnalysisRecipeSchemaGeneration::V3 => {
                let config = self
                    .method_config
                    .as_ref()
                    .ok_or(EffectiveRecipeMetadataError::MissingMethodConfig)?;
                if !config.supports_method(self.settings.method) {
                    return Err(EffectiveRecipeMetadataError::MethodMismatch {
                        config_kind: config.kind(),
                        method: self.settings.method,
                    });
                }
                let keys = self
                    .executable_legacy_metadata_keys()
                    .into_iter()
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                if !keys.is_empty() {
                    return Err(EffectiveRecipeMetadataError::ExecutableMetadataConflict { keys });
                }
                let mut metadata = self.metadata.clone();
                metadata.extend(config.legacy_metadata_projection());
                Ok(metadata)
            }
            AnalysisRecipeSchemaGeneration::Future(version) => {
                Err(EffectiveRecipeMetadataError::UnsupportedSchema(version))
            }
        }
    }

    /// Clones the recipe and replaces metadata with `effective_metadata` for a
    /// legacy metadata-reading engine. The original persisted recipe is never
    /// mutated or reinterpreted.
    pub fn with_effective_metadata(&self) -> Result<Self, EffectiveRecipeMetadataError> {
        let mut projected = self.clone();
        projected.metadata = self.effective_metadata()?;
        Ok(projected)
    }
}

const PREDICT_LEGACY_METADATA_KEYS: &[&str] = &[
    "segment_count",
    "segmentation.segment_count",
    "groups.segment_count",
    "pls_pos_segments",
    "segmentation.pls_pos_segments",
    "segment_starts",
    "segmentation.segment_starts",
    "groups.segment_starts",
    "minimum_segment_share",
    "segmentation.minimum_segment_share",
    "groups.minimum_segment_share",
    "fimix_classes",
    "segmentation.fimix_classes",
    "groups.fimix_classes",
    "group_methods",
];
const MGA_LEGACY_METADATA_KEYS: &[&str] = &[
    "mga_group_column",
    "mga.group_column",
    "mga_group_a",
    "mga.group_a",
    "mga_group_b",
    "mga.group_b",
    "group_methods",
    "group_permutation_samples",
    "micom_configural_confirmed",
];
const IPMA_LEGACY_METADATA_KEYS: &[&str] = &["ipma_targets", "ipma.targets"];
const CBSEM_LEGACY_METADATA_KEYS: &[&str] = &[
    "cbsem_model_type",
    "cbsem.model_type",
    "cbsem_estimator",
    "cbsem.estimator",
    "cbsem_input",
    "cbsem.input",
    "cbsem_mean_structure",
    "cbsem.mean_structure",
    "cbsem_bootstrap_samples",
    "cbsem_group_column",
    "cbsem_invariance_steps",
];
const PCA_LEGACY_METADATA_KEYS: &[&str] = &[
    "pca_variables",
    "pca.variables",
    "pca_component_rule",
    "pca_components",
    "pca_variance_threshold",
];
const REGRESSION_LEGACY_METADATA_KEYS: &[&str] = &[
    "regression_type",
    "regression_outcome",
    "regression_predictors",
    "regression.predictors",
    "regression_controls",
    "regression.controls",
    "robust_se",
    "process_model",
    "process_x",
    "process_m",
    "process_w",
];
const NCA_LEGACY_METADATA_KEYS: &[&str] =
    &["nca_x", "nca_y", "nca_ceiling", "nca_permutation_samples"];

pub fn is_executable_legacy_metadata_key(key: &str) -> bool {
    EXECUTABLE_LEGACY_METADATA_KEYS.contains(&key)
}

fn legacy_alias_value(
    metadata: &BTreeMap<String, String>,
    keys: &[&'static str],
) -> Result<Option<String>, AnalysisRecipeMigrationError> {
    let mut selected: Option<(&'static str, String)> = None;
    for &key in keys {
        let Some(value) = metadata.get(key) else {
            continue;
        };
        let value = value.trim().to_owned();
        if let Some((selected_key, selected_value)) = &selected
            && selected_value != &value
        {
            return Err(AnalysisRecipeMigrationError::ConflictingAliases {
                left: *selected_key,
                right: key,
            });
        }
        selected = Some((key, value));
    }
    Ok(selected
        .map(|(_, value)| value)
        .filter(|value| !value.is_empty()))
}

fn legacy_required(
    metadata: &BTreeMap<String, String>,
    keys: &[&'static str],
) -> Result<String, AnalysisRecipeMigrationError> {
    legacy_alias_value(metadata, keys)?
        .ok_or(AnalysisRecipeMigrationError::MissingMetadata(keys[0]))
}

fn normalized_legacy_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn legacy_list(
    metadata: &BTreeMap<String, String>,
    keys: &[&'static str],
) -> Result<Vec<String>, AnalysisRecipeMigrationError> {
    let mut selected: Option<(&'static str, Vec<String>)> = None;
    for &key in keys {
        let Some(value) = metadata.get(key) else {
            continue;
        };
        let value = normalized_legacy_list(value);
        if let Some((selected_key, selected_value)) = &selected
            && selected_value != &value
        {
            return Err(AnalysisRecipeMigrationError::ConflictingAliases {
                left: *selected_key,
                right: key,
            });
        }
        selected = Some((key, value));
    }
    Ok(selected.map(|(_, value)| value).unwrap_or_default())
}

fn legacy_u32(
    metadata: &BTreeMap<String, String>,
    keys: &[&'static str],
    default: Option<u32>,
) -> Result<u32, AnalysisRecipeMigrationError> {
    match legacy_alias_value(metadata, keys)? {
        Some(value) => {
            value
                .parse::<u32>()
                .map_err(|_| AnalysisRecipeMigrationError::InvalidMetadata {
                    key: keys[0],
                    value,
                })
        }
        None => default.ok_or(AnalysisRecipeMigrationError::MissingMetadata(keys[0])),
    }
}

fn legacy_f64(
    metadata: &BTreeMap<String, String>,
    keys: &[&'static str],
    default: Option<f64>,
) -> Result<f64, AnalysisRecipeMigrationError> {
    match legacy_alias_value(metadata, keys)? {
        Some(value) => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or(AnalysisRecipeMigrationError::InvalidMetadata {
                key: keys[0],
                value,
            }),
        None => default.ok_or(AnalysisRecipeMigrationError::MissingMetadata(keys[0])),
    }
}

fn legacy_bool(
    metadata: &BTreeMap<String, String>,
    keys: &[&'static str],
    default: bool,
) -> Result<bool, AnalysisRecipeMigrationError> {
    match legacy_alias_value(metadata, keys)? {
        Some(value) if value.eq_ignore_ascii_case("true") => Ok(true),
        Some(value) if value.eq_ignore_ascii_case("false") => Ok(false),
        Some(value) => Err(AnalysisRecipeMigrationError::InvalidMetadata {
            key: keys[0],
            value,
        }),
        None => Ok(default),
    }
}

fn legacy_predict_config(
    metadata: &BTreeMap<String, String>,
) -> Result<MethodConfig, AnalysisRecipeMigrationError> {
    let methods = legacy_list(metadata, &["group_methods"])?
        .into_iter()
        .map(|method| method.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if let Some(method) = methods
        .iter()
        .find(|method| !matches!(method.as_str(), "fimix" | "pls_pos"))
    {
        return Err(AnalysisRecipeMigrationError::InvalidMetadata {
            key: "group_methods",
            value: method.clone(),
        });
    }
    let pls_pos_segments = legacy_alias_value(
        metadata,
        &[
            "segment_count",
            "segmentation.pls_pos_segments",
            "pls_pos_segments",
        ],
    )?
    .map(|value| {
        value
            .parse::<u32>()
            .map_err(|_| AnalysisRecipeMigrationError::InvalidMetadata {
                key: "segment_count",
                value,
            })
    })
    .transpose()?;
    let fimix_segments = legacy_alias_value(
        metadata,
        &[
            "fimix_classes",
            "segmentation.fimix_classes",
            "groups.fimix_classes",
        ],
    )?
    .map(|value| {
        value
            .parse::<u32>()
            .map_err(|_| AnalysisRecipeMigrationError::InvalidMetadata {
                key: "fimix_classes",
                value,
            })
    })
    .transpose()?;
    let fimix_fallback_segments = legacy_alias_value(
        metadata,
        &[
            "segment_count",
            "segmentation.segment_count",
            "groups.segment_count",
        ],
    )?
    .map(|value| {
        value
            .parse::<u32>()
            .map_err(|_| AnalysisRecipeMigrationError::InvalidMetadata {
                key: "segment_count",
                value,
            })
    })
    .transpose()?;
    let fimix_segments = fimix_segments.or_else(|| {
        methods
            .iter()
            .any(|method| method == "fimix")
            .then_some(fimix_fallback_segments.unwrap_or(2))
    });
    let starts = legacy_u32(
        metadata,
        &[
            "segment_starts",
            "segmentation.segment_starts",
            "groups.segment_starts",
        ],
        Some(10),
    )?;
    let minimum_segment_share = legacy_f64(
        metadata,
        &[
            "minimum_segment_share",
            "segmentation.minimum_segment_share",
            "groups.minimum_segment_share",
        ],
        Some(0.10),
    )?;
    if methods.iter().any(|method| method == "pls_pos") && pls_pos_segments.is_none() {
        return Err(AnalysisRecipeMigrationError::MissingMetadata(
            "segment_count",
        ));
    }
    if pls_pos_segments.is_none()
        && fimix_segments.is_none()
        && PREDICT_LEGACY_METADATA_KEYS
            .iter()
            .any(|key| *key != "group_methods" && metadata.contains_key(*key))
    {
        return Err(AnalysisRecipeMigrationError::MissingMetadata(
            "segment_count_or_fimix_classes",
        ));
    }
    let build = |segments| SegmentationMethodConfig {
        segments,
        starts,
        minimum_segment_share,
    };
    Ok(MethodConfig::Predict {
        pls_pos: pls_pos_segments.map(build),
        fimix: fimix_segments.map(build),
    })
}

fn legacy_mga_config(
    metadata: &BTreeMap<String, String>,
) -> Result<MethodConfig, AnalysisRecipeMigrationError> {
    let methods = legacy_list(metadata, &["group_methods"])?
        .into_iter()
        .map(|method| match method.to_ascii_lowercase().as_str() {
            "micom" => Ok(GroupAnalysisMethod::Micom),
            "mga_permutation" => Ok(GroupAnalysisMethod::MgaPermutation),
            _ => Err(AnalysisRecipeMigrationError::InvalidMetadata {
                key: "group_methods",
                value: method,
            }),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MethodConfig::Mga {
        group_column: legacy_required(metadata, &["mga_group_column", "mga.group_column"])?,
        group_a: legacy_required(metadata, &["mga_group_a", "mga.group_a"])?,
        group_b: legacy_required(metadata, &["mga_group_b", "mga.group_b"])?,
        methods,
        permutation_samples: legacy_u32(metadata, &["group_permutation_samples"], None)?,
        configural_invariance_confirmed: legacy_bool(
            metadata,
            &["micom_configural_confirmed"],
            false,
        )?,
    })
}

fn legacy_ipma_config(
    metadata: &BTreeMap<String, String>,
    model: &ModelSpec,
) -> Result<MethodConfig, AnalysisRecipeMigrationError> {
    let mut targets = legacy_list(metadata, &["ipma_targets", "ipma.targets"])?;
    if targets.is_empty() {
        targets = model
            .constructs
            .iter()
            .filter(|construct| model.paths.iter().any(|path| path.target == construct.id))
            .map(|construct| construct.id.clone())
            .collect();
    }
    Ok(MethodConfig::Ipma { targets })
}

fn legacy_cbsem_config(
    metadata: &BTreeMap<String, String>,
    model: &ModelSpec,
) -> Result<MethodConfig, AnalysisRecipeMigrationError> {
    let model_type =
        match legacy_alias_value(metadata, &["cbsem_model_type", "cbsem.model_type"])?.as_deref() {
            Some("cfa") => CbsemModelType::Cfa,
            Some("sem") => CbsemModelType::Sem,
            Some(value) => {
                return Err(AnalysisRecipeMigrationError::InvalidMetadata {
                    key: "cbsem_model_type",
                    value: value.into(),
                });
            }
            None if model.paths.is_empty() => CbsemModelType::Cfa,
            None => CbsemModelType::Sem,
        };
    let estimator =
        match legacy_alias_value(metadata, &["cbsem_estimator", "cbsem.estimator"])?.as_deref() {
            None | Some("ml") => CbsemEstimator::Ml,
            Some("robust_ml") => CbsemEstimator::RobustMl,
            Some("wlsmv") => CbsemEstimator::Wlsmv,
            Some(value) => {
                return Err(AnalysisRecipeMigrationError::InvalidMetadata {
                    key: "cbsem_estimator",
                    value: value.into(),
                });
            }
        };
    let input = match legacy_required(metadata, &["cbsem_input", "cbsem.input"])?.as_str() {
        "raw" => CbsemInput::Raw,
        "covariance" => CbsemInput::Covariance,
        "correlation" => CbsemInput::Correlation,
        value => {
            return Err(AnalysisRecipeMigrationError::InvalidMetadata {
                key: "cbsem_input",
                value: value.into(),
            });
        }
    };
    let invariance_steps = legacy_list(metadata, &["cbsem_invariance_steps"])?
        .into_iter()
        .map(|step| match step.as_str() {
            "configural" => Ok(CbsemInvarianceStep::Configural),
            "metric" => Ok(CbsemInvarianceStep::Metric),
            "scalar" => Ok(CbsemInvarianceStep::Scalar),
            _ => Err(AnalysisRecipeMigrationError::InvalidMetadata {
                key: "cbsem_invariance_steps",
                value: step,
            }),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MethodConfig::Cbsem {
        model_type,
        estimator,
        input,
        mean_structure: legacy_bool(
            metadata,
            &["cbsem_mean_structure", "cbsem.mean_structure"],
            false,
        )?,
        bootstrap_samples: legacy_u32(metadata, &["cbsem_bootstrap_samples"], Some(0))?,
        group_column: legacy_alias_value(metadata, &["cbsem_group_column"])?,
        invariance_steps,
    })
}

fn legacy_pca_config(
    metadata: &BTreeMap<String, String>,
) -> Result<MethodConfig, AnalysisRecipeMigrationError> {
    let variables = legacy_list(metadata, &["pca_variables", "pca.variables"])?;
    if variables.is_empty() {
        return Err(AnalysisRecipeMigrationError::MissingMetadata(
            "pca_variables",
        ));
    }
    let retention = match legacy_alias_value(metadata, &["pca_component_rule"])?.as_deref() {
        None | Some("kaiser") => PcaRetentionConfig::Kaiser,
        Some("fixed") => PcaRetentionConfig::Fixed {
            components: legacy_u32(metadata, &["pca_components"], None)?,
        },
        Some("variance_threshold") => PcaRetentionConfig::VarianceThreshold {
            threshold: legacy_f64(metadata, &["pca_variance_threshold"], None)?,
        },
        Some(value) => {
            return Err(AnalysisRecipeMigrationError::InvalidMetadata {
                key: "pca_component_rule",
                value: value.into(),
            });
        }
    };
    Ok(MethodConfig::Pca {
        variables,
        retention,
    })
}

fn legacy_regression_config(
    metadata: &BTreeMap<String, String>,
) -> Result<MethodConfig, AnalysisRecipeMigrationError> {
    let outcome = legacy_required(metadata, &["regression_outcome"])?;
    let predictors = legacy_list(
        metadata,
        &["regression_predictors", "regression.predictors"],
    )?;
    if predictors.is_empty() {
        return Err(AnalysisRecipeMigrationError::MissingMetadata(
            "regression_predictors",
        ));
    }
    let controls = legacy_list(metadata, &["regression_controls", "regression.controls"])?;
    let regression_type = legacy_alias_value(metadata, &["regression_type"])?;
    if regression_type.as_deref() == Some("process") {
        return Err(AnalysisRecipeMigrationError::ArchiveOnlyProcessV1);
    }
    let model = match regression_type.as_deref() {
        None | Some("ols") => match legacy_required(metadata, &["robust_se"])?.as_str() {
            "hc3" => RegressionModelConfig::Ols {
                robust_se: RobustStandardError::Hc3,
            },
            value => {
                return Err(AnalysisRecipeMigrationError::InvalidMetadata {
                    key: "robust_se",
                    value: value.into(),
                });
            }
        },
        Some("logistic") => RegressionModelConfig::Logistic,
        Some("process") => {
            let x = legacy_alias_value(metadata, &["process_x"])?
                .unwrap_or_else(|| predictors[0].clone());
            let relationship = match legacy_alias_value(metadata, &["process_model"])?.as_deref() {
                None | Some("mediation") => ProcessRelationshipConfig::Mediation {
                    x,
                    mediator: legacy_required(metadata, &["process_m"])?,
                },
                Some("moderation") => ProcessRelationshipConfig::Moderation {
                    x,
                    moderator: legacy_required(metadata, &["process_w"])?,
                },
                Some("moderated_mediation") => ProcessRelationshipConfig::ModeratedMediation {
                    x,
                    mediator: legacy_required(metadata, &["process_m"])?,
                    moderator: legacy_required(metadata, &["process_w"])?,
                },
                Some(value) => {
                    return Err(AnalysisRecipeMigrationError::InvalidMetadata {
                        key: "process_model",
                        value: value.into(),
                    });
                }
            };
            RegressionModelConfig::Process { relationship }
        }
        Some(value) => {
            return Err(AnalysisRecipeMigrationError::InvalidMetadata {
                key: "regression_type",
                value: value.into(),
            });
        }
    };
    Ok(MethodConfig::Regression {
        outcome,
        predictors,
        controls,
        model,
        bootstrap: None,
    })
}

fn legacy_nca_config(
    metadata: &BTreeMap<String, String>,
) -> Result<MethodConfig, AnalysisRecipeMigrationError> {
    let ceiling = match legacy_alias_value(metadata, &["nca_ceiling"])?.as_deref() {
        None | Some("both") => NcaCeiling::Both,
        Some("ce_fdh") => NcaCeiling::CeFdh,
        Some("cr_fdh") => NcaCeiling::CrFdh,
        Some(value) => {
            return Err(AnalysisRecipeMigrationError::InvalidMetadata {
                key: "nca_ceiling",
                value: value.into(),
            });
        }
    };
    Ok(MethodConfig::Nca {
        condition: legacy_required(metadata, &["nca_x"])?,
        outcome: legacy_required(metadata, &["nca_y"])?,
        ceiling,
        permutation_samples: legacy_u32(metadata, &["nca_permutation_samples"], Some(999))?,
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
