use crate::{MeasurementMode, ModelSpec};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub const SEM_MODEL_V4_SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemModelV4 {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub variables: Vec<SemVariableV4>,
    pub relations: Vec<SemRelationV4>,
    pub parameters: Vec<SemParameterV4>,
    pub constraints: Vec<SemConstraintV4>,
    pub derived_terms: Vec<SemDerivedTermV4>,
    pub group: SemGroupV4,
    pub data_binding: SemDataBindingV4,
    #[serde(default)]
    pub annotations: Vec<SemAnnotationV4>,
    #[serde(default)]
    pub presentation: SemPresentationV4,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemVariableV4 {
    Observed {
        id: String,
        label: String,
        source_column: String,
        scale: ObservedScaleV4,
        role: ObservedRoleV4,
        #[serde(default)]
        categories: Vec<String>,
        #[serde(default)]
        value_labels: BTreeMap<String, String>,
        #[serde(default)]
        missing_markers: Vec<String>,
        #[serde(default)]
        transformation_lineage: Vec<ObservedTransformationStepV4>,
    },
    CommonFactor {
        id: String,
        label: String,
        identification: FactorIdentificationV4,
        mean_policy: FactorMeanPolicyV4,
        disturbance_policy: FactorDisturbancePolicyV4,
    },
    Composite {
        id: String,
        label: String,
        weighting: CompositeWeightingV4,
    },
    Derived {
        id: String,
        label: String,
    },
}

impl SemVariableV4 {
    pub fn id(&self) -> &str {
        match self {
            Self::Observed { id, .. }
            | Self::CommonFactor { id, .. }
            | Self::Composite { id, .. }
            | Self::Derived { id, .. } => id,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Observed { label, .. }
            | Self::CommonFactor { label, .. }
            | Self::Composite { label, .. }
            | Self::Derived { label, .. } => label,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedScaleV4 {
    Continuous,
    Binary,
    Ordinal,
    Nominal,
    Identifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedRoleV4 {
    Indicator,
    Structural,
    Both,
    Control,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompositeWeightingV4 {
    ModeA,
    ModeB,
    Unit {
        normalization: CompositeWeightNormalizationV4,
    },
    Custom {
        weights: BTreeMap<String, f64>,
        normalization: CompositeWeightNormalizationV4,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CompositeWeightNormalizationV4 {
    None,
    SumToOne,
    UnitVariance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservedTransformationStepV4 {
    pub id: String,
    pub input_columns: Vec<String>,
    pub output_column: String,
    pub operation: ObservedTransformationOperationV4,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedTransformationOperationV4 {
    Recode {
        mappings: BTreeMap<String, String>,
        unmapped: RecodeUnmappedPolicyV4,
    },
    MeanCenter,
    Standardize {
        denominator: StandardizationDenominatorV4,
    },
    Log {
        base: f64,
        offset: f64,
    },
    Formula {
        expression: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RecodeUnmappedPolicyV4 {
    Keep,
    SetMissing,
    Reject,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum StandardizationDenominatorV4 {
    Sample,
    Population,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FactorIdentificationV4 {
    MarkerLoading { indicator: String },
    FixedVariance,
    EffectsCoding,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FactorMeanPolicyV4 {
    FixedZero,
    Estimated {
        parameter: String,
    },
    ReferenceGroup {
        reference_group: String,
        parameter: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FactorDisturbancePolicyV4 {
    ExogenousVariance { parameter: String },
    EndogenousDisturbance { parameter: String },
    FixedZero { parameter: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SemEndpointV4 {
    Variable(String),
    ResidualOf(String),
    DisturbanceOf(String),
}

impl SemEndpointV4 {
    pub fn variable_id(&self) -> &str {
        match self {
            Self::Variable(id) | Self::ResidualOf(id) | Self::DisturbanceOf(id) => id,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum StructuralRelationRoleV4 {
    #[default]
    Structural,
    Control,
}

impl StructuralRelationRoleV4 {
    pub(crate) fn is_structural(&self) -> bool {
        *self == Self::Structural
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemRelationV4 {
    MeasurementEffect {
        id: String,
        construct: String,
        indicator: String,
        parameter: String,
    },
    MeasurementCausal {
        id: String,
        indicator: String,
        composite: String,
        parameter: String,
    },
    Structural {
        id: String,
        source: String,
        target: String,
        parameter: String,
        #[serde(
            default,
            skip_serializing_if = "StructuralRelationRoleV4::is_structural"
        )]
        role: StructuralRelationRoleV4,
        #[serde(default)]
        intercept_parameter: Option<String>,
    },
    Covariance {
        id: String,
        left: SemEndpointV4,
        right: SemEndpointV4,
        parameter: String,
    },
}

impl SemRelationV4 {
    pub fn id(&self) -> &str {
        match self {
            Self::MeasurementEffect { id, .. }
            | Self::MeasurementCausal { id, .. }
            | Self::Structural { id, .. }
            | Self::Covariance { id, .. } => id,
        }
    }

    pub fn parameter(&self) -> &str {
        match self {
            Self::MeasurementEffect { parameter, .. }
            | Self::MeasurementCausal { parameter, .. }
            | Self::Structural { parameter, .. }
            | Self::Covariance { parameter, .. } => parameter,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemParameterTargetV4 {
    Loading {
        construct: String,
        indicator: String,
    },
    Weight {
        indicator: String,
        composite: String,
    },
    Regression {
        source: String,
        target: String,
    },
    Variance {
        endpoint: SemEndpointV4,
    },
    Covariance {
        left: SemEndpointV4,
        right: SemEndpointV4,
    },
    Intercept {
        variable: String,
    },
    Mean {
        variable: String,
    },
    Threshold {
        variable: String,
        index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemParameterGroupOverrideV4 {
    pub group: String,
    pub specification: SemParameterGroupOverrideSpecV4,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemParameterGroupOverrideSpecV4 {
    Free {
        #[serde(default)]
        start: Option<f64>,
        #[serde(default)]
        lower: Option<f64>,
        #[serde(default)]
        upper: Option<f64>,
    },
    Fixed {
        value: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemParameterV4 {
    Free {
        id: String,
        label: String,
        target: SemParameterTargetV4,
        #[serde(default)]
        start: Option<f64>,
        #[serde(default)]
        lower: Option<f64>,
        #[serde(default)]
        upper: Option<f64>,
        #[serde(default)]
        equality_label: Option<String>,
        #[serde(default)]
        group_overrides: Vec<SemParameterGroupOverrideV4>,
    },
    Fixed {
        id: String,
        label: String,
        target: SemParameterTargetV4,
        value: f64,
        #[serde(default)]
        group_overrides: Vec<SemParameterGroupOverrideV4>,
    },
    Derived {
        id: String,
        label: String,
        target: SemParameterTargetV4,
        expression: String,
        #[serde(default)]
        group_overrides: Vec<SemParameterGroupOverrideV4>,
    },
}

impl SemParameterV4 {
    pub fn id(&self) -> &str {
        match self {
            Self::Free { id, .. } | Self::Fixed { id, .. } | Self::Derived { id, .. } => id,
        }
    }

    pub fn target(&self) -> &SemParameterTargetV4 {
        match self {
            Self::Free { target, .. }
            | Self::Fixed { target, .. }
            | Self::Derived { target, .. } => target,
        }
    }

    pub fn fixed_value(&self) -> Option<f64> {
        match self {
            Self::Fixed { value, .. } => Some(*value),
            _ => None,
        }
    }

    pub fn group_overrides(&self) -> &[SemParameterGroupOverrideV4] {
        match self {
            Self::Free {
                group_overrides, ..
            }
            | Self::Fixed {
                group_overrides, ..
            }
            | Self::Derived {
                group_overrides, ..
            } => group_overrides,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemLinearConstraintTermV4 {
    pub parameter: String,
    pub coefficient: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemConstraintV4 {
    Equality {
        id: String,
        parameters: Vec<String>,
    },
    Bound {
        id: String,
        parameter: String,
        #[serde(default)]
        lower: Option<f64>,
        #[serde(default)]
        upper: Option<f64>,
    },
    Linear {
        id: String,
        terms: Vec<SemLinearConstraintTermV4>,
        value: f64,
    },
}

impl SemConstraintV4 {
    pub fn id(&self) -> &str {
        match self {
            Self::Equality { id, .. } | Self::Bound { id, .. } | Self::Linear { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InteractionMethodV4 {
    ProductIndicator,
    TwoStage,
    Orthogonalizing,
}

/// Scientific lower-order-term policy for a version-2 interaction.
///
/// New canvas helpers create `Strong` interactions. The other policies remain
/// representable so imported or expert-authored models are never rewritten;
/// estimator capability preflight decides whether an exact policy is runnable.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InteractionHierarchyPolicyV2 {
    /// Every operand has a main effect on the focal outcome and every immediate
    /// lower-order interaction is present. Strong lower-order terms make the
    /// requirement transitive for interactions of order four and above.
    Strong,
    /// Main effects are present, but lower-order interactions are not required.
    Weak,
    /// The scientific model deliberately imposes no hierarchy requirement.
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ProductIndicatorCenteringV4 {
    None,
    MeanCenter,
    DoubleMeanCenter,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ProductIndicatorStandardizationV4 {
    None,
    SampleStandardDeviation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ProductIndicatorPairingV4 {
    AllPairs,
}

/// Scientific settings for a product-indicator latent interaction.
///
/// This object is deliberately required (rather than defaulted) whenever an
/// interaction selects `ProductIndicator`. Historical schema-v4 documents
/// without the object still deserialize, but semantic validation fails closed
/// instead of silently assigning a different product construction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProductIndicatorSpecificationV4 {
    pub centering: ProductIndicatorCenteringV4,
    pub standardization: ProductIndicatorStandardizationV4,
    pub pairing: ProductIndicatorPairingV4,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HigherOrderConstructionApproachV4 {
    RepeatedIndicators,
    ExtendedRepeatedIndicators,
    EmbeddedTwoStage,
    DisjointTwoStage,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HigherOrderMeasurementTypeV4 {
    ReflectiveReflective,
    ReflectiveFormative,
    FormativeReflective,
    FormativeFormative,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemDerivedTermV4 {
    Interaction {
        id: String,
        output: String,
        predictor: String,
        moderator: String,
        focal_relation: String,
        method: InteractionMethodV4,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        product_indicator: Option<ProductIndicatorSpecificationV4>,
    },
    /// Stable multi-operand interaction authority. `operands[0]` is the focal
    /// predictor and subsequent operands are moderators in authored order.
    /// Existing two-way `interaction` documents remain readable unchanged.
    InteractionV2 {
        id: String,
        output: String,
        operands: Vec<String>,
        focal_relation: String,
        method: InteractionMethodV4,
        hierarchy_policy: InteractionHierarchyPolicyV2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        product_indicator: Option<ProductIndicatorSpecificationV4>,
    },
    HigherOrder {
        id: String,
        output: String,
        components: Vec<String>,
        approach: HigherOrderConstructionApproachV4,
        measurement_type: HigherOrderMeasurementTypeV4,
    },
    Polynomial {
        id: String,
        output: String,
        source: String,
        degree: u8,
    },
}

impl SemDerivedTermV4 {
    pub fn id(&self) -> &str {
        match self {
            Self::Interaction { id, .. }
            | Self::InteractionV2 { id, .. }
            | Self::HigherOrder { id, .. }
            | Self::Polynomial { id, .. } => id,
        }
    }

    pub fn output(&self) -> &str {
        match self {
            Self::Interaction { output, .. }
            | Self::InteractionV2 { output, .. }
            | Self::HigherOrder { output, .. }
            | Self::Polynomial { output, .. } => output,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SemGroupLevelV4 {
    pub id: String,
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemGroupV4 {
    #[default]
    SingleGroup,
    ObservedGroups {
        grouping_variable: String,
        levels: Vec<SemGroupLevelV4>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemMatrixSampleMetadataV4 {
    pub sample_size: usize,
    /// Declares whether covariance values (or the standard deviations paired
    /// with a correlation matrix) use the unbiased n-1 denominator or the ML
    /// n denominator. Estimators must consume this value explicitly.
    pub covariance_denominator: SemCovarianceDenominatorV4,
    #[serde(default)]
    pub effective_sample_size: Option<f64>,
    #[serde(default)]
    pub degrees_of_freedom: Option<usize>,
    #[serde(default)]
    pub group_sample_sizes: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SemCovarianceDenominatorV4 {
    SampleNMinusOne,
    MaximumLikelihoodN,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SamplingWeightNormalizationV4 {
    None,
    MeanOne,
    SumToSampleSize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemWeightBindingV4 {
    Case {
        variable: String,
    },
    Frequency {
        variable: String,
    },
    Sampling {
        variable: String,
        normalization: SamplingWeightNormalizationV4,
    },
}

impl SemWeightBindingV4 {
    pub fn variable(&self) -> &str {
        match self {
            Self::Case { variable }
            | Self::Frequency { variable }
            | Self::Sampling { variable, .. } => variable,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MissingDataPolicyV4 {
    ListwiseDeletion,
    PairwiseDeletion,
    MeanReplacement,
    FullInformationMaximumLikelihood,
    MultipleImputation { imputations: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemDataBindingV4 {
    Raw {
        dataset_id: String,
        missing_data: MissingDataPolicyV4,
        #[serde(default)]
        weight: Option<SemWeightBindingV4>,
        #[serde(default)]
        cluster_variable: Option<String>,
        #[serde(default)]
        strata_variable: Option<String>,
    },
    Covariance {
        dataset_id: String,
        variables: Vec<String>,
        #[serde(default)]
        means: Option<BTreeMap<String, f64>>,
        #[serde(default)]
        standard_deviations: Option<BTreeMap<String, f64>>,
        sample: SemMatrixSampleMetadataV4,
    },
    Correlation {
        dataset_id: String,
        variables: Vec<String>,
        #[serde(default)]
        means: Option<BTreeMap<String, f64>>,
        #[serde(default)]
        standard_deviations: Option<BTreeMap<String, f64>>,
        sample: SemMatrixSampleMetadataV4,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemAnnotationV4 {
    DisplayOnlyCovariance {
        id: String,
        left: String,
        right: String,
        #[serde(default)]
        label: Option<String>,
    },
    Caption {
        id: String,
        text: String,
    },
    Note {
        id: String,
        subject: String,
        text: String,
    },
}

impl SemAnnotationV4 {
    pub fn id(&self) -> &str {
        match self {
            Self::DisplayOnlyCovariance { id, .. }
            | Self::Caption { id, .. }
            | Self::Note { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemCanvasNodeV4 {
    pub variable: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub style: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SemCanvasEdgeV4 {
    pub relation: String,
    #[serde(default)]
    pub routing: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SemCanvasShapeKindV4 {
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Diamond,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemCanvasShapeV4 {
    pub id: String,
    pub shape: SemCanvasShapeKindV4,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub style: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemCanvasImageV4 {
    pub id: String,
    pub asset_ref: String,
    pub alt_text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub style: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemCanvasLineV4 {
    pub id: String,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub start_marker: Option<String>,
    #[serde(default)]
    pub end_marker: Option<String>,
    #[serde(default)]
    pub style: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemPresentationV4 {
    #[default]
    None,
    Canvas {
        nodes: Vec<SemCanvasNodeV4>,
        edges: Vec<SemCanvasEdgeV4>,
        #[serde(default)]
        shapes: Vec<SemCanvasShapeV4>,
        #[serde(default)]
        images: Vec<SemCanvasImageV4>,
        #[serde(default)]
        lines: Vec<SemCanvasLineV4>,
        #[serde(default)]
        zoom: Option<f64>,
        #[serde(default)]
        pan_x: Option<f64>,
        #[serde(default)]
        pan_y: Option<f64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SemModelV4Issue {
    pub code: String,
    pub subject: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("SEM model v4 validation failed")]
pub struct SemModelV4ValidationError {
    pub issues: Vec<SemModelV4Issue>,
}

fn issue(
    code: &str,
    subject: impl Into<Option<String>>,
    message: impl Into<String>,
) -> SemModelV4Issue {
    SemModelV4Issue {
        code: code.into(),
        subject: subject.into(),
        message: message.into(),
    }
}

impl SemModelV4 {
    /// Validates that an authoring document has a stable schema and unambiguous
    /// typed content. Drafts may omit not-yet-authored science, but any typed
    /// objects they contain must have coherent references, shapes, and values.
    pub fn validate_authoring_integrity(&self) -> Vec<SemModelV4Issue> {
        let mut issues = self.validate_authoring_structure();
        self.append_draft_reference_issues(&mut issues);
        let parameters = self
            .parameters
            .iter()
            .map(|parameter| (parameter.id(), parameter))
            .collect::<HashMap<_, _>>();
        let mut policy_issues = Vec::new();
        validate_identification(self, &parameters, &mut policy_issues);
        issues.extend(
            policy_issues
                .into_iter()
                .filter(is_authoring_integrity_policy_issue),
        );
        issues
    }

    fn append_draft_reference_issues(&self, issues: &mut Vec<SemModelV4Issue>) {
        for variable in &self.variables {
            let SemVariableV4::CommonFactor {
                id,
                identification: FactorIdentificationV4::MarkerLoading { indicator },
                ..
            } = variable
            else {
                continue;
            };
            if !self.relations.iter().any(|relation| {
                matches!(
                    relation,
                    SemRelationV4::MeasurementEffect {
                        construct,
                        indicator: measured_indicator,
                        ..
                    } if construct == id && measured_indicator == indicator
                )
            }) {
                issues.push(issue(
                    "authoring.marker.reference_invalid",
                    Some(id.clone()),
                    "Marker-loading identification must reference an effect indicator of the factor",
                ));
            }
        }
    }

    fn validate_authoring_structure(&self) -> Vec<SemModelV4Issue> {
        let mut issues = Vec::new();
        if self.schema_version != SEM_MODEL_V4_SCHEMA_VERSION {
            issues.push(issue(
                "schema.version",
                Some(self.schema_version.to_string()),
                "SEM model schema_version must be 4",
            ));
        }
        if self.id.trim().is_empty() {
            issues.push(issue("model.id.empty", None, "Model id cannot be empty"));
        }
        if self.name.trim().is_empty() {
            issues.push(issue(
                "model.name.empty",
                None,
                "Model name cannot be empty",
            ));
        }

        let mut object_ids = HashMap::<&str, &str>::new();
        for (kind, id) in self
            .variables
            .iter()
            .map(|value| ("variable", value.id()))
            .chain(self.relations.iter().map(|value| ("relation", value.id())))
            .chain(
                self.parameters
                    .iter()
                    .map(|value| ("parameter", value.id())),
            )
            .chain(
                self.constraints
                    .iter()
                    .map(|value| ("constraint", value.id())),
            )
            .chain(
                self.derived_terms
                    .iter()
                    .map(|value| ("derived_term", value.id())),
            )
            .chain(
                self.annotations
                    .iter()
                    .map(|value| ("annotation", value.id())),
            )
        {
            if id.trim().is_empty() {
                issues.push(issue(
                    "object.id.empty",
                    Some(kind.to_string()),
                    "Object id cannot be empty",
                ));
            } else if let Some(previous) = object_ids.insert(id, kind) {
                issues.push(issue(
                    "object.id.duplicate",
                    Some(id.to_string()),
                    format!("Object id is shared by {previous} and {kind}"),
                ));
            }
        }

        self.append_authoring_content_issues(&mut issues);
        issues
    }

    pub fn ensure_authoring_integrity(&self) -> Result<(), SemModelV4ValidationError> {
        let issues = self.validate_authoring_integrity();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(SemModelV4ValidationError { issues })
        }
    }

    fn append_authoring_content_issues(&self, issues: &mut Vec<SemModelV4Issue>) {
        let variables = self
            .variables
            .iter()
            .map(|variable| (variable.id(), variable))
            .collect::<HashMap<_, _>>();
        let parameters = self
            .parameters
            .iter()
            .map(|parameter| (parameter.id(), parameter))
            .collect::<HashMap<_, _>>();

        for variable in &self.variables {
            if variable.label().trim().is_empty() {
                issues.push(issue(
                    "variable.label.empty",
                    Some(variable.id().to_string()),
                    "Variable label cannot be empty",
                ));
            }
            if let SemVariableV4::Observed { .. } = variable {
                validate_observed_variable(variable, issues);
            }
        }

        let mut structural_pairs = HashSet::new();
        let mut covariance_pairs = HashSet::new();
        for relation in &self.relations {
            let parameter = parameters.get(relation.parameter()).copied();
            if parameter.is_none() {
                issues.push(issue(
                    "relation.parameter.unknown",
                    Some(relation.id().to_string()),
                    format!("Unknown parameter {}", relation.parameter()),
                ));
            }
            match relation {
                SemRelationV4::MeasurementEffect {
                    id,
                    construct,
                    indicator,
                    ..
                } => {
                    if !matches!(
                        variables.get(construct.as_str()),
                        Some(SemVariableV4::CommonFactor { .. } | SemVariableV4::Composite { .. })
                    ) {
                        issues.push(issue(
                            "measurement.effect.construct.invalid",
                            Some(id.clone()),
                            "Effect measurement source must be a common factor or composite",
                        ));
                    }
                    if !matches!(
                        variables.get(indicator.as_str()),
                        Some(SemVariableV4::Observed { .. })
                    ) {
                        issues.push(issue(
                            "measurement.indicator.invalid",
                            Some(id.clone()),
                            "Measurement indicator must be an observed variable",
                        ));
                    }
                    let expected = SemParameterTargetV4::Loading {
                        construct: construct.clone(),
                        indicator: indicator.clone(),
                    };
                    if parameter.is_some_and(|parameter| parameter.target() != &expected) {
                        issues.push(issue(
                            "relation.parameter.target_mismatch",
                            Some(id.clone()),
                            "Measurement-effect parameter target does not match the relation",
                        ));
                    }
                }
                SemRelationV4::MeasurementCausal {
                    id,
                    indicator,
                    composite,
                    ..
                } => {
                    if !matches!(
                        variables.get(indicator.as_str()),
                        Some(SemVariableV4::Observed { .. })
                    ) {
                        issues.push(issue(
                            "measurement.indicator.invalid",
                            Some(id.clone()),
                            "Measurement indicator must be an observed variable",
                        ));
                    }
                    if !matches!(
                        variables.get(composite.as_str()),
                        Some(SemVariableV4::Composite { .. })
                    ) {
                        issues.push(issue(
                            "measurement.causal.composite.invalid",
                            Some(id.clone()),
                            "Causal measurement target must be a composite",
                        ));
                    }
                    let expected = SemParameterTargetV4::Weight {
                        indicator: indicator.clone(),
                        composite: composite.clone(),
                    };
                    if parameter.is_some_and(|parameter| parameter.target() != &expected) {
                        issues.push(issue(
                            "relation.parameter.target_mismatch",
                            Some(id.clone()),
                            "Measurement-causal parameter target does not match the relation",
                        ));
                    }
                }
                SemRelationV4::Structural {
                    id,
                    source,
                    target,
                    intercept_parameter,
                    ..
                } => {
                    if !variables.contains_key(source.as_str())
                        || !variables.contains_key(target.as_str())
                    {
                        issues.push(issue(
                            "structural.variable.unknown",
                            Some(id.clone()),
                            "Structural relation references an unknown variable",
                        ));
                    }
                    if source == target {
                        issues.push(issue(
                            "structural.self",
                            Some(id.clone()),
                            "A structural relation cannot be a self-loop",
                        ));
                    }
                    if !structural_pairs.insert((source.as_str(), target.as_str())) {
                        issues.push(issue(
                            "structural.duplicate",
                            Some(id.clone()),
                            "Structural relation is duplicated",
                        ));
                    }
                    let expected = SemParameterTargetV4::Regression {
                        source: source.clone(),
                        target: target.clone(),
                    };
                    if parameter.is_some_and(|parameter| parameter.target() != &expected) {
                        issues.push(issue(
                            "relation.parameter.target_mismatch",
                            Some(id.clone()),
                            "Structural parameter target does not match the relation",
                        ));
                    }
                    if let Some(intercept) = intercept_parameter {
                        match parameters.get(intercept.as_str()) {
                            Some(parameter)
                                if parameter.target()
                                    == &SemParameterTargetV4::Intercept {
                                        variable: target.clone(),
                                    } => {}
                            Some(_) => issues.push(issue(
                                "structural.intercept.target_mismatch",
                                Some(id.clone()),
                                "Intercept parameter target must match the structural outcome",
                            )),
                            None => issues.push(issue(
                                "structural.intercept.unknown",
                                Some(id.clone()),
                                format!("Unknown intercept parameter {intercept}"),
                            )),
                        }
                    }
                }
                SemRelationV4::Covariance {
                    id, left, right, ..
                } => {
                    validate_endpoint(left, id, &variables, issues);
                    validate_endpoint(right, id, &variables, issues);
                    if left == right {
                        issues.push(issue(
                            "covariance.self",
                            Some(id.clone()),
                            "Use a variance parameter rather than a covariance self-loop",
                        ));
                    }
                    let pair = canonical_endpoint_pair(left.clone(), right.clone());
                    if !covariance_pairs.insert(pair.clone()) {
                        issues.push(issue(
                            "covariance.duplicate",
                            Some(id.clone()),
                            "Scientific covariance relation is duplicated",
                        ));
                    }
                    let expected = SemParameterTargetV4::Covariance {
                        left: pair.0,
                        right: pair.1,
                    };
                    if parameter.is_some_and(|parameter| {
                        canonical_parameter_target(parameter.target().clone()) != expected
                    }) {
                        issues.push(issue(
                            "relation.parameter.target_mismatch",
                            Some(id.clone()),
                            "Covariance parameter target does not match the relation",
                        ));
                    }
                }
            }
        }

        validate_observed_roles(&self.variables, &self.relations, issues);

        for parameter in &self.parameters {
            validate_parameter(parameter, &variables, &self.group, issues);
        }
        validate_constraints(&self.constraints, &parameters, issues);
        validate_derived_terms(&self.derived_terms, &variables, &self.relations, issues);
        validate_group(&self.group, &variables, issues);
        validate_data_binding(&self.data_binding, &variables, &self.group, issues);
        validate_annotations(&self.annotations, &variables, issues);
        validate_presentation(&self.presentation, &variables, &self.relations, issues);
    }

    /// Full scientific validation. Passing authoring-integrity validation alone
    /// never implies that a model is ready for compilation or execution.
    pub fn validate(&self) -> Vec<SemModelV4Issue> {
        // Preserve the established ready-model validation behavior and issue
        // ordering exactly; draft integrity selects only non-readiness policy
        // issues from the same validator above.
        let mut issues = self.validate_authoring_structure();
        let parameters = self
            .parameters
            .iter()
            .map(|parameter| (parameter.id(), parameter))
            .collect::<HashMap<_, _>>();
        validate_identification(self, &parameters, &mut issues);
        issues
    }

    pub fn ensure_valid(&self) -> Result<(), SemModelV4ValidationError> {
        let issues = self.validate();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(SemModelV4ValidationError { issues })
        }
    }

    pub fn has_structural_feedback(&self) -> bool {
        let nodes = self
            .variables
            .iter()
            .map(|variable| variable.id())
            .collect::<HashSet<_>>();
        let mut indegree = nodes
            .iter()
            .map(|id| (*id, 0usize))
            .collect::<HashMap<_, _>>();
        let mut outgoing = HashMap::<&str, Vec<&str>>::new();
        for relation in &self.relations {
            if let SemRelationV4::Structural { source, target, .. } = relation {
                if source != target
                    && nodes.contains(source.as_str())
                    && nodes.contains(target.as_str())
                {
                    *indegree.get_mut(target.as_str()).expect("known target") += 1;
                    outgoing.entry(source).or_default().push(target);
                }
            }
        }
        let mut ready = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
            .collect::<Vec<_>>();
        let mut visited = 0usize;
        while let Some(source) = ready.pop() {
            visited += 1;
            for target in outgoing.get(source).into_iter().flatten() {
                let degree = indegree.get_mut(target).expect("known target");
                *degree -= 1;
                if *degree == 0 {
                    ready.push(target);
                }
            }
        }
        visited != nodes.len()
    }

    pub fn canonicalized(&self) -> Self {
        let mut model = self.clone();
        for variable in &mut model.variables {
            if let SemVariableV4::Observed {
                missing_markers, ..
            } = variable
            {
                missing_markers.sort();
            }
        }
        model
            .variables
            .sort_by(|left, right| left.id().cmp(right.id()));
        for relation in &mut model.relations {
            if let SemRelationV4::Covariance { left, right, .. } = relation {
                let pair = canonical_endpoint_pair(left.clone(), right.clone());
                *left = pair.0;
                *right = pair.1;
            }
        }
        model
            .relations
            .sort_by(|left, right| left.id().cmp(right.id()));
        for parameter in &mut model.parameters {
            match parameter {
                SemParameterV4::Free {
                    target,
                    group_overrides,
                    ..
                }
                | SemParameterV4::Fixed {
                    target,
                    group_overrides,
                    ..
                }
                | SemParameterV4::Derived {
                    target,
                    group_overrides,
                    ..
                } => {
                    *target = canonical_parameter_target(target.clone());
                    group_overrides.sort_by(|left, right| left.group.cmp(&right.group));
                }
            }
        }
        model
            .parameters
            .sort_by(|left, right| left.id().cmp(right.id()));
        for constraint in &mut model.constraints {
            match constraint {
                SemConstraintV4::Equality { parameters, .. } => parameters.sort(),
                SemConstraintV4::Linear { terms, .. } => {
                    terms.sort_by(|left, right| left.parameter.cmp(&right.parameter));
                }
                SemConstraintV4::Bound { .. } => {}
            }
        }
        model
            .constraints
            .sort_by(|left, right| left.id().cmp(right.id()));
        for term in &mut model.derived_terms {
            if let SemDerivedTermV4::HigherOrder { components, .. } = term {
                components.sort();
            }
        }
        model
            .derived_terms
            .sort_by(|left, right| left.id().cmp(right.id()));
        if let SemGroupV4::ObservedGroups { levels, .. } = &mut model.group {
            levels.sort_by(|left, right| left.id.cmp(&right.id));
        }
        for annotation in &mut model.annotations {
            if let SemAnnotationV4::DisplayOnlyCovariance { left, right, .. } = annotation {
                if left > right {
                    std::mem::swap(left, right);
                }
            }
        }
        model
            .annotations
            .sort_by(|left, right| left.id().cmp(right.id()));
        if let SemPresentationV4::Canvas {
            nodes,
            edges,
            shapes,
            images,
            lines,
            ..
        } = &mut model.presentation
        {
            nodes.sort_by(|left, right| left.variable.cmp(&right.variable));
            edges.sort_by(|left, right| left.relation.cmp(&right.relation));
            shapes.sort_by(|left, right| left.id.cmp(&right.id));
            images.sort_by(|left, right| left.id.cmp(&right.id));
            lines.sort_by(|left, right| left.id.cmp(&right.id));
        }
        model
    }

    /// Deterministic identity input for the complete authoring document,
    /// including annotations and presentation. Unlike scientific identity,
    /// this is available for an incomplete but structurally unambiguous draft.
    pub fn model_document_hash_input(&self) -> Result<Vec<u8>, SemModelV4ValidationError> {
        self.ensure_authoring_integrity()?;
        Ok(serde_json::to_vec(&self.canonicalized()).expect("SemModelV4 is serializable"))
    }

    pub fn model_document_sha256(&self) -> Result<String, SemModelV4ValidationError> {
        let digest = Sha256::digest(self.model_document_hash_input()?);
        Ok(format!("{digest:x}"))
    }

    /// Deterministic scientific identity input. Annotations and canvas presentation are
    /// intentionally excluded because they do not change the estimand or compiled plan.
    pub fn scientific_hash_input(&self) -> Result<Vec<u8>, SemModelV4ValidationError> {
        self.ensure_valid()?;
        let mut scientific = self.canonicalized();
        scientific.annotations.clear();
        scientific.presentation = SemPresentationV4::None;
        Ok(serde_json::to_vec(&scientific).expect("SemModelV4 is serializable"))
    }

    pub fn scientific_sha256(&self) -> Result<String, SemModelV4ValidationError> {
        let digest = Sha256::digest(self.scientific_hash_input()?);
        Ok(format!("{digest:x}"))
    }
}

fn validate_observed_roles(
    variables: &[SemVariableV4],
    relations: &[SemRelationV4],
    issues: &mut Vec<SemModelV4Issue>,
) {
    for variable in variables {
        let SemVariableV4::Observed {
            id, role, scale, ..
        } = variable
        else {
            continue;
        };
        let used_as_indicator = relations.iter().any(|relation| {
            matches!(relation, SemRelationV4::MeasurementEffect { indicator, .. } | SemRelationV4::MeasurementCausal { indicator, .. } if indicator == id)
        });
        let used_as_source = relations.iter().any(
            |relation| matches!(relation, SemRelationV4::Structural { source, .. } if source == id),
        );
        let used_as_target = relations.iter().any(
            |relation| matches!(relation, SemRelationV4::Structural { target, .. } if target == id),
        );
        let invalid = match role {
            ObservedRoleV4::Indicator => used_as_source || used_as_target,
            ObservedRoleV4::Structural => used_as_indicator,
            ObservedRoleV4::Both => false,
            ObservedRoleV4::Control => used_as_indicator || used_as_target,
        };
        if invalid {
            issues.push(issue(
                "observed.role.usage_invalid",
                Some(id.clone()),
                "Observed-variable role is inconsistent with its measurement or structural use",
            ));
        }
        if matches!(scale, ObservedScaleV4::Identifier)
            && (used_as_indicator || used_as_source || used_as_target)
        {
            issues.push(issue(
                "observed.identifier.model_use_invalid",
                Some(id.clone()),
                "Identifier variables cannot participate in measurement or structural relations",
            ));
        }
    }
}

fn validate_observed_variable(variable: &SemVariableV4, issues: &mut Vec<SemModelV4Issue>) {
    let SemVariableV4::Observed {
        id,
        source_column,
        scale,
        categories,
        value_labels,
        missing_markers,
        transformation_lineage,
        ..
    } = variable
    else {
        return;
    };
    if source_column.trim().is_empty() {
        issues.push(issue(
            "observed.source_column.empty",
            Some(id.clone()),
            "Observed variable source column cannot be empty",
        ));
    }
    let required_categories = match scale {
        ObservedScaleV4::Binary => Some(2),
        ObservedScaleV4::Ordinal | ObservedScaleV4::Nominal => Some(usize::MAX),
        ObservedScaleV4::Continuous | ObservedScaleV4::Identifier => None,
    };
    match required_categories {
        Some(2) if categories.len() != 2 => issues.push(issue(
            "observed.categories.binary_invalid",
            Some(id.clone()),
            "Binary variables require exactly two categories",
        )),
        Some(_) if categories.len() < 2 => issues.push(issue(
            "observed.categories.insufficient",
            Some(id.clone()),
            "Ordinal and nominal variables require at least two categories",
        )),
        None if !categories.is_empty() => issues.push(issue(
            "observed.categories.scale_invalid",
            Some(id.clone()),
            "Continuous and identifier variables cannot declare categories",
        )),
        _ => {}
    }
    let category_set = categories.iter().collect::<HashSet<_>>();
    if category_set.len() != categories.len()
        || categories.iter().any(|category| category.trim().is_empty())
    {
        issues.push(issue(
            "observed.categories.duplicate_or_empty",
            Some(id.clone()),
            "Observed categories must be non-empty and unique",
        ));
    }
    if value_labels.iter().any(|(value, label)| {
        value.trim().is_empty() || label.trim().is_empty() || !category_set.contains(value)
    }) {
        issues.push(issue(
            "observed.value_labels.invalid",
            Some(id.clone()),
            "Value labels must be non-empty and reference declared categories",
        ));
    }
    let missing_set = missing_markers.iter().collect::<HashSet<_>>();
    if missing_set.len() != missing_markers.len()
        || missing_markers
            .iter()
            .any(|marker| marker.trim().is_empty() || category_set.contains(marker))
    {
        issues.push(issue(
            "observed.missing_markers.invalid",
            Some(id.clone()),
            "Missing markers must be non-empty, unique, and distinct from categories",
        ));
    }

    let mut step_ids = HashSet::new();
    let mut outputs = HashSet::new();
    for step in transformation_lineage {
        if step.id.trim().is_empty()
            || !step_ids.insert(step.id.as_str())
            || step.input_columns.is_empty()
            || step
                .input_columns
                .iter()
                .any(|column| column.trim().is_empty())
            || step.output_column.trim().is_empty()
            || !outputs.insert(step.output_column.as_str())
        {
            issues.push(issue(
                "observed.transformation.step_invalid",
                Some(id.clone()),
                "Transformation steps require unique ids and outputs plus non-empty inputs",
            ));
        }
        match &step.operation {
            ObservedTransformationOperationV4::Recode { mappings, .. }
                if mappings.is_empty()
                    || mappings
                        .iter()
                        .any(|(from, to)| from.trim().is_empty() || to.trim().is_empty()) =>
            {
                issues.push(issue(
                    "observed.transformation.recode_invalid",
                    Some(step.id.clone()),
                    "Recode mappings must be non-empty and contain non-empty values",
                ));
            }
            ObservedTransformationOperationV4::Log { base, offset }
                if !base.is_finite()
                    || *base <= 0.0
                    || (*base - 1.0).abs() <= f64::EPSILON
                    || !offset.is_finite() =>
            {
                issues.push(issue(
                    "observed.transformation.log_invalid",
                    Some(step.id.clone()),
                    "Log transformations require a finite positive base other than one and a finite offset",
                ));
            }
            ObservedTransformationOperationV4::Formula { expression }
                if expression.trim().is_empty() =>
            {
                issues.push(issue(
                    "observed.transformation.formula_empty",
                    Some(step.id.clone()),
                    "Formula transformations require an expression",
                ));
            }
            _ => {}
        }
    }
    if transformation_lineage
        .last()
        .is_some_and(|step| step.output_column != *source_column)
    {
        issues.push(issue(
            "observed.transformation.output_mismatch",
            Some(id.clone()),
            "The final transformation output must match source_column",
        ));
    }
}

fn validate_endpoint(
    endpoint: &SemEndpointV4,
    relation_id: &str,
    variables: &HashMap<&str, &SemVariableV4>,
    issues: &mut Vec<SemModelV4Issue>,
) {
    let Some(variable) = variables.get(endpoint.variable_id()) else {
        issues.push(issue(
            "endpoint.variable.unknown",
            Some(relation_id.to_string()),
            format!("Unknown endpoint variable {}", endpoint.variable_id()),
        ));
        return;
    };
    match endpoint {
        SemEndpointV4::ResidualOf(_) if !matches!(variable, SemVariableV4::Observed { .. }) => {
            issues.push(issue(
                "endpoint.residual.invalid",
                Some(relation_id.to_string()),
                "Residual endpoint must reference an observed variable",
            ));
        }
        SemEndpointV4::DisturbanceOf(_)
            if !matches!(
                variable,
                SemVariableV4::CommonFactor { .. }
                    | SemVariableV4::Composite { .. }
                    | SemVariableV4::Derived { .. }
            ) =>
        {
            issues.push(issue(
                "endpoint.disturbance.invalid",
                Some(relation_id.to_string()),
                "Disturbance endpoint must reference a latent, composite, or derived variable",
            ));
        }
        _ => {}
    }
}

fn validate_parameter(
    parameter: &SemParameterV4,
    variables: &HashMap<&str, &SemVariableV4>,
    group: &SemGroupV4,
    issues: &mut Vec<SemModelV4Issue>,
) {
    if let SemParameterV4::Free {
        id,
        start,
        lower,
        upper,
        equality_label,
        ..
    } = parameter
    {
        for (name, value) in [("start", start), ("lower", lower), ("upper", upper)] {
            if value.is_some_and(|value| !value.is_finite()) {
                issues.push(issue(
                    "parameter.value.non_finite",
                    Some(id.clone()),
                    format!("Parameter {name} value must be finite"),
                ));
            }
        }
        if lower
            .zip(*upper)
            .is_some_and(|(lower, upper)| lower > upper)
        {
            issues.push(issue(
                "parameter.bounds.invalid",
                Some(id.clone()),
                "Parameter lower bound cannot exceed upper bound",
            ));
        }
        if start.is_some_and(|start| {
            lower.is_some_and(|lower| start < lower) || upper.is_some_and(|upper| start > upper)
        }) {
            issues.push(issue(
                "parameter.start.outside_bounds",
                Some(id.clone()),
                "Parameter start must lie within its declared bounds",
            ));
        }
        if equality_label
            .as_ref()
            .is_some_and(|label| label.trim().is_empty())
        {
            issues.push(issue(
                "parameter.equality_label.empty",
                Some(id.clone()),
                "Parameter equality labels cannot be empty or whitespace",
            ));
        }
    }
    if let SemParameterV4::Fixed { id, value, .. } = parameter {
        if !value.is_finite() {
            issues.push(issue(
                "parameter.value.non_finite",
                Some(id.clone()),
                "Fixed parameter value must be finite",
            ));
        }
    }
    let valid_groups = match group {
        SemGroupV4::SingleGroup => None,
        SemGroupV4::ObservedGroups { levels, .. } => Some(
            levels
                .iter()
                .map(|level| level.id.as_str())
                .collect::<HashSet<_>>(),
        ),
    };
    let mut overridden_groups = HashSet::new();
    for override_spec in parameter.group_overrides() {
        if override_spec.group.trim().is_empty()
            || !overridden_groups.insert(override_spec.group.as_str())
            || valid_groups
                .as_ref()
                .is_none_or(|groups| !groups.contains(override_spec.group.as_str()))
        {
            issues.push(issue(
                "parameter.group_override.group_invalid",
                Some(parameter.id().to_string()),
                "Parameter group overrides must uniquely reference declared group levels",
            ));
        }
        match &override_spec.specification {
            SemParameterGroupOverrideSpecV4::Free {
                start,
                lower,
                upper,
            } => {
                if [start, lower, upper]
                    .into_iter()
                    .flatten()
                    .any(|value| !value.is_finite())
                    || lower
                        .zip(*upper)
                        .is_some_and(|(lower, upper)| lower > upper)
                    || start.is_some_and(|start| {
                        lower.is_some_and(|lower| start < lower)
                            || upper.is_some_and(|upper| start > upper)
                    })
                {
                    issues.push(issue(
                        "parameter.group_override.value_invalid",
                        Some(parameter.id().to_string()),
                        "Free group overrides require finite, ordered bounds and a start within them",
                    ));
                }
            }
            SemParameterGroupOverrideSpecV4::Fixed { value } if !value.is_finite() => {
                issues.push(issue(
                    "parameter.group_override.value_invalid",
                    Some(parameter.id().to_string()),
                    "Fixed group override values must be finite",
                ));
            }
            SemParameterGroupOverrideSpecV4::Fixed { .. } => {}
        }
    }
    match parameter.target() {
        SemParameterTargetV4::Loading {
            construct,
            indicator,
        } => {
            if !matches!(
                variables.get(construct.as_str()),
                Some(SemVariableV4::CommonFactor { .. } | SemVariableV4::Composite { .. })
            ) || !matches!(
                variables.get(indicator.as_str()),
                Some(SemVariableV4::Observed { .. })
            ) {
                issues.push(issue(
                    "parameter.loading.target_invalid",
                    Some(parameter.id().to_string()),
                    "Loading target must reference a construct and observed indicator",
                ));
            }
        }
        SemParameterTargetV4::Weight {
            indicator,
            composite,
        } => {
            if !matches!(
                variables.get(indicator.as_str()),
                Some(SemVariableV4::Observed { .. })
            ) || !matches!(
                variables.get(composite.as_str()),
                Some(SemVariableV4::Composite { .. })
            ) {
                issues.push(issue(
                    "parameter.weight.target_invalid",
                    Some(parameter.id().to_string()),
                    "Weight target must reference an observed indicator and composite",
                ));
            }
        }
        SemParameterTargetV4::Regression { source, target } => {
            if !variables.contains_key(source.as_str()) || !variables.contains_key(target.as_str())
            {
                issues.push(issue(
                    "parameter.regression.target_invalid",
                    Some(parameter.id().to_string()),
                    "Regression parameter references an unknown variable",
                ));
            }
        }
        SemParameterTargetV4::Variance { endpoint } => {
            validate_endpoint(endpoint, parameter.id(), variables, issues);
        }
        SemParameterTargetV4::Covariance { left, right } => {
            validate_endpoint(left, parameter.id(), variables, issues);
            validate_endpoint(right, parameter.id(), variables, issues);
        }
        SemParameterTargetV4::Intercept { variable }
        | SemParameterTargetV4::Mean { variable }
        | SemParameterTargetV4::Threshold { variable, .. } => {
            if !variables.contains_key(variable.as_str()) {
                issues.push(issue(
                    "parameter.location.target_invalid",
                    Some(parameter.id().to_string()),
                    "Location parameter references an unknown variable",
                ));
            }
        }
    }
}

fn validate_constraints(
    constraints: &[SemConstraintV4],
    parameters: &HashMap<&str, &SemParameterV4>,
    issues: &mut Vec<SemModelV4Issue>,
) {
    for constraint in constraints {
        match constraint {
            SemConstraintV4::Equality {
                id,
                parameters: ids,
            } => {
                let unique = ids.iter().collect::<HashSet<_>>();
                if unique.len() < 2 || unique.len() != ids.len() {
                    issues.push(issue(
                        "constraint.equality.invalid",
                        Some(id.clone()),
                        "Equality constraint requires at least two unique parameters",
                    ));
                }
                for parameter in ids {
                    if !parameters.contains_key(parameter.as_str()) {
                        issues.push(issue(
                            "constraint.parameter.unknown",
                            Some(id.clone()),
                            format!("Unknown constrained parameter {parameter}"),
                        ));
                    }
                }
            }
            SemConstraintV4::Bound {
                id,
                parameter,
                lower,
                upper,
            } => {
                if !parameters.contains_key(parameter.as_str()) {
                    issues.push(issue(
                        "constraint.parameter.unknown",
                        Some(id.clone()),
                        format!("Unknown constrained parameter {parameter}"),
                    ));
                }
                if lower.is_none() && upper.is_none() {
                    issues.push(issue(
                        "constraint.bound.empty",
                        Some(id.clone()),
                        "Bound constraint requires a lower or upper limit",
                    ));
                }
                if lower
                    .zip(*upper)
                    .is_some_and(|(lower, upper)| lower > upper)
                {
                    issues.push(issue(
                        "constraint.bound.invalid",
                        Some(id.clone()),
                        "Constraint lower bound cannot exceed upper bound",
                    ));
                }
            }
            SemConstraintV4::Linear { id, terms, value } => {
                if terms.is_empty() || !value.is_finite() {
                    issues.push(issue(
                        "constraint.linear.invalid",
                        Some(id.clone()),
                        "Linear constraint requires finite terms and target value",
                    ));
                }
                let mut seen = HashSet::new();
                for term in terms {
                    if !parameters.contains_key(term.parameter.as_str()) {
                        issues.push(issue(
                            "constraint.parameter.unknown",
                            Some(id.clone()),
                            format!("Unknown constrained parameter {}", term.parameter),
                        ));
                    }
                    if !term.coefficient.is_finite() || !seen.insert(term.parameter.as_str()) {
                        issues.push(issue(
                            "constraint.linear.term_invalid",
                            Some(id.clone()),
                            "Linear constraint terms must be finite and unique",
                        ));
                    }
                }
            }
        }
    }
}

fn validate_derived_terms(
    terms: &[SemDerivedTermV4],
    variables: &HashMap<&str, &SemVariableV4>,
    relations: &[SemRelationV4],
    issues: &mut Vec<SemModelV4Issue>,
) {
    let relations_by_id = relations
        .iter()
        .map(|relation| (relation.id(), relation))
        .collect::<HashMap<_, _>>();
    let mut outputs = HashSet::new();
    for term in terms {
        if !outputs.insert(term.output()) {
            issues.push(issue(
                "derived.output.duplicate",
                Some(term.output().to_string()),
                "A derived variable can have only one defining term",
            ));
        }
        if !matches!(
            variables.get(term.output()),
            Some(SemVariableV4::Derived { .. })
        ) {
            issues.push(issue(
                "derived.output.invalid",
                Some(term.id().to_string()),
                "Derived term output must reference a derived variable",
            ));
        }
        match term {
            SemDerivedTermV4::Interaction {
                id,
                output,
                predictor,
                moderator,
                focal_relation,
                method,
                product_indicator,
            } => {
                if predictor == moderator || predictor == output || moderator == output {
                    issues.push(issue(
                        "derived.interaction.roles_invalid",
                        Some(id.clone()),
                        "Interaction predictor, moderator, and output must be distinct",
                    ));
                }
                for input in [predictor, moderator] {
                    if !variables.contains_key(input.as_str()) {
                        issues.push(issue(
                            "derived.input.unknown",
                            Some(id.clone()),
                            format!("Unknown interaction input {input}"),
                        ));
                    }
                }
                match (method, product_indicator) {
                    (InteractionMethodV4::ProductIndicator, None) => issues.push(issue(
                        "derived.interaction.product_indicator_spec_required",
                        Some(id.clone()),
                        "Product-indicator interactions require explicit construction settings",
                    )),
                    (InteractionMethodV4::ProductIndicator, Some(_)) => {}
                    (_, Some(_)) => issues.push(issue(
                        "derived.interaction.product_indicator_spec_forbidden",
                        Some(id.clone()),
                        "Product-indicator construction settings are valid only for the product-indicator method",
                    )),
                    (_, None) => {}
                }
                match relations_by_id.get(focal_relation.as_str()) {
                    Some(SemRelationV4::Structural { source, target, .. })
                        if source == predictor =>
                    {
                        if !relations.iter().any(|relation| {
                            matches!(relation, SemRelationV4::Structural { source, target: interaction_target, .. } if source == output && interaction_target == target)
                        }) {
                            issues.push(issue(
                                "derived.interaction.effect_path_missing",
                                Some(id.clone()),
                                "The interaction output must have a structural path to the focal relation outcome",
                            ));
                        }
                    }
                    Some(_) => issues.push(issue(
                        "derived.interaction.focal_relation_invalid",
                        Some(id.clone()),
                        "The focal relation must be a structural path sourced by the predictor",
                    )),
                    None => issues.push(issue(
                        "derived.interaction.focal_relation_unknown",
                        Some(id.clone()),
                        format!("Unknown focal relation {focal_relation}"),
                    )),
                }
            }
            SemDerivedTermV4::InteractionV2 {
                id,
                output,
                operands,
                focal_relation,
                method,
                hierarchy_policy,
                product_indicator,
            } => {
                let unique_operands = operands.iter().collect::<HashSet<_>>();
                if operands.len() < 2
                    || unique_operands.len() != operands.len()
                    || operands.contains(output)
                {
                    issues.push(issue(
                        "derived.interaction_v2.operands_invalid",
                        Some(id.clone()),
                        "interaction_v2 requires at least two unique, non-output operands in stable authored order",
                    ));
                }
                for input in operands {
                    if !variables.contains_key(input.as_str()) {
                        issues.push(issue(
                            "derived.input.unknown",
                            Some(id.clone()),
                            format!("Unknown interaction input {input}"),
                        ));
                    }
                }
                match (method, product_indicator) {
                    (InteractionMethodV4::ProductIndicator, None) => issues.push(issue(
                        "derived.interaction.product_indicator_spec_required",
                        Some(id.clone()),
                        "Product-indicator interactions require explicit construction settings",
                    )),
                    (InteractionMethodV4::ProductIndicator, Some(_)) => {}
                    (_, Some(_)) => issues.push(issue(
                        "derived.interaction.product_indicator_spec_forbidden",
                        Some(id.clone()),
                        "Product-indicator construction settings are valid only for the product-indicator method",
                    )),
                    (_, None) => {}
                }

                let focal_target = match (
                    operands.first(),
                    relations_by_id.get(focal_relation.as_str()),
                ) {
                    (
                        Some(predictor),
                        Some(SemRelationV4::Structural {
                            source,
                            target,
                            role: StructuralRelationRoleV4::Structural,
                            ..
                        }),
                    ) if source.as_str() == predictor.as_str() => Some(target.as_str()),
                    (_, Some(_)) => {
                        issues.push(issue(
                            "derived.interaction.focal_relation_invalid",
                            Some(id.clone()),
                            "The focal relation must be a structural-effect path sourced by operands[0]",
                        ));
                        None
                    }
                    (_, None) => {
                        issues.push(issue(
                            "derived.interaction.focal_relation_unknown",
                            Some(id.clone()),
                            format!("Unknown focal relation {focal_relation}"),
                        ));
                        None
                    }
                };

                if let Some(target) = focal_target {
                    let has_effect_path = relations.iter().any(|relation| {
                        matches!(relation, SemRelationV4::Structural {
                            source,
                            target: interaction_target,
                            role: StructuralRelationRoleV4::Structural,
                            ..
                        } if source == output && interaction_target == target)
                    });
                    if !has_effect_path {
                        issues.push(issue(
                            "derived.interaction.effect_path_missing",
                            Some(id.clone()),
                            "The interaction output must have a structural-effect path to the focal relation outcome",
                        ));
                    }

                    if matches!(
                        hierarchy_policy,
                        InteractionHierarchyPolicyV2::Strong | InteractionHierarchyPolicyV2::Weak
                    ) {
                        for operand in operands {
                            if !relations.iter().any(|relation| {
                                matches!(relation, SemRelationV4::Structural {
                                    source,
                                    target: main_target,
                                    role: StructuralRelationRoleV4::Structural,
                                    ..
                                } if source == operand && main_target == target)
                            }) {
                                issues.push(issue(
                                    "derived.interaction_v2.main_effect_missing",
                                    Some(id.clone()),
                                    format!(
                                        "Hierarchy policy requires a main-effect path from {operand} to {target}"
                                    ),
                                ));
                            }
                        }
                    }

                    if *hierarchy_policy == InteractionHierarchyPolicyV2::Strong
                        && operands.len() > 2
                        && unique_operands.len() == operands.len()
                    {
                        for omitted in 0..operands.len() {
                            let required = operands
                                .iter()
                                .enumerate()
                                .filter_map(|(index, operand)| {
                                    (index != omitted).then_some(operand.as_str())
                                })
                                .collect::<HashSet<_>>();
                            let lower_order_present = terms.iter().any(|candidate| {
                                if candidate.id() == id {
                                    return false;
                                }
                                let Some(candidate_operands) = interaction_operands(candidate)
                                else {
                                    return false;
                                };
                                if candidate_operands.len() != required.len()
                                    || candidate_operands.iter().copied().collect::<HashSet<_>>()
                                        != required
                                {
                                    return false;
                                }
                                if candidate_operands.len() > 2
                                    && !matches!(
                                        candidate,
                                        SemDerivedTermV4::InteractionV2 {
                                            hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
                                            ..
                                        }
                                    )
                                {
                                    return false;
                                }
                                relations.iter().any(|relation| {
                                    matches!(relation, SemRelationV4::Structural {
                                        source,
                                        target: lower_target,
                                        role: StructuralRelationRoleV4::Structural,
                                        ..
                                    } if source == candidate.output() && lower_target == target)
                                })
                            });
                            if !lower_order_present {
                                let mut required = required.into_iter().collect::<Vec<_>>();
                                required.sort_unstable();
                                issues.push(issue(
                                    "derived.interaction_v2.lower_order_missing",
                                    Some(id.clone()),
                                    format!(
                                        "Strong hierarchy requires an interaction for operands [{}] targeting {target}",
                                        required.join(", ")
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
            SemDerivedTermV4::HigherOrder {
                id,
                output,
                components,
                ..
            } => {
                let unique = components.iter().collect::<HashSet<_>>();
                if unique.len() < 2
                    || unique.len() != components.len()
                    || components.contains(output)
                {
                    issues.push(issue(
                        "derived.higher_order.components_invalid",
                        Some(id.clone()),
                        "Higher-order term requires at least two unique non-self components",
                    ));
                }
                for component in components {
                    if !variables.contains_key(component.as_str()) {
                        issues.push(issue(
                            "derived.input.unknown",
                            Some(id.clone()),
                            format!("Unknown higher-order component {component}"),
                        ));
                    }
                }
            }
            SemDerivedTermV4::Polynomial {
                id,
                output,
                source,
                degree,
            } => {
                if *degree < 2 || source == output || !variables.contains_key(source.as_str()) {
                    issues.push(issue(
                        "derived.polynomial.invalid",
                        Some(id.clone()),
                        "Polynomial term requires a known non-output source and degree of at least two",
                    ));
                }
            }
        }
    }
    for variable in variables.values() {
        if matches!(variable, SemVariableV4::Derived { .. }) && !outputs.contains(variable.id()) {
            issues.push(issue(
                "derived.definition.missing",
                Some(variable.id().to_string()),
                "Derived variable requires exactly one defining term",
            ));
        }
    }
}

fn interaction_operands(term: &SemDerivedTermV4) -> Option<Vec<&str>> {
    match term {
        SemDerivedTermV4::Interaction {
            predictor,
            moderator,
            ..
        } => Some(vec![predictor.as_str(), moderator.as_str()]),
        SemDerivedTermV4::InteractionV2 { operands, .. } => {
            Some(operands.iter().map(String::as_str).collect())
        }
        SemDerivedTermV4::HigherOrder { .. } | SemDerivedTermV4::Polynomial { .. } => None,
    }
}

fn validate_group(
    group: &SemGroupV4,
    variables: &HashMap<&str, &SemVariableV4>,
    issues: &mut Vec<SemModelV4Issue>,
) {
    if let SemGroupV4::ObservedGroups {
        grouping_variable,
        levels,
    } = group
    {
        if !matches!(
            variables.get(grouping_variable.as_str()),
            Some(SemVariableV4::Observed { .. })
        ) {
            issues.push(issue(
                "group.variable.invalid",
                Some(grouping_variable.clone()),
                "Grouping variable must be observed",
            ));
        }
        if levels.len() < 2 {
            issues.push(issue(
                "group.levels.insufficient",
                Some(grouping_variable.clone()),
                "Observed group analysis requires at least two levels",
            ));
        }
        let mut ids = HashSet::new();
        let mut values = HashSet::new();
        for level in levels {
            if level.id.trim().is_empty()
                || level.value.trim().is_empty()
                || !ids.insert(level.id.as_str())
                || !values.insert(level.value.as_str())
            {
                issues.push(issue(
                    "group.level.duplicate_or_empty",
                    Some(level.id.clone()),
                    "Group level ids and values must be non-empty and unique",
                ));
            }
        }
    }
}

fn validate_data_binding(
    binding: &SemDataBindingV4,
    variables: &HashMap<&str, &SemVariableV4>,
    group: &SemGroupV4,
    issues: &mut Vec<SemModelV4Issue>,
) {
    match binding {
        SemDataBindingV4::Raw {
            dataset_id,
            missing_data,
            weight,
            cluster_variable,
            strata_variable,
        } => {
            if dataset_id.trim().is_empty() {
                issues.push(issue(
                    "data.dataset.empty",
                    None,
                    "Dataset id cannot be empty",
                ));
            }
            if matches!(missing_data, MissingDataPolicyV4::MultipleImputation { imputations } if *imputations < 2)
            {
                issues.push(issue(
                    "data.imputation.count_invalid",
                    None,
                    "Multiple imputation requires at least two imputations",
                ));
            }
            if let Some(weight) = weight {
                let id = weight.variable();
                if !matches!(
                    variables.get(id),
                    Some(SemVariableV4::Observed {
                        scale: ObservedScaleV4::Continuous,
                        role: ObservedRoleV4::Control,
                        ..
                    })
                ) {
                    issues.push(issue(
                        "data.weight.variable_invalid",
                        Some(id.to_string()),
                        "Case, frequency, and sampling weights require a continuous observed control variable",
                    ));
                }
            }
            for (role, id) in [("cluster", cluster_variable), ("strata", strata_variable)] {
                if let Some(id) = id {
                    if !matches!(
                        variables.get(id.as_str()),
                        Some(SemVariableV4::Observed { .. })
                    ) {
                        issues.push(issue(
                            "data.binding.variable_invalid",
                            Some(id.clone()),
                            format!("{role} variable must reference an observed variable"),
                        ));
                    }
                }
            }
        }
        SemDataBindingV4::Covariance {
            dataset_id,
            variables: matrix_variables,
            means,
            standard_deviations,
            sample,
        }
        | SemDataBindingV4::Correlation {
            dataset_id,
            variables: matrix_variables,
            means,
            standard_deviations,
            sample,
        } => {
            if dataset_id.trim().is_empty() || sample.sample_size < 2 {
                issues.push(issue(
                    "data.matrix.invalid",
                    Some(dataset_id.clone()),
                    "Matrix input requires a dataset id and sample size of at least two",
                ));
            }
            let matrix_set = matrix_variables.iter().collect::<HashSet<_>>();
            if matrix_variables.len() < 2
                || matrix_set.len() != matrix_variables.len()
                || matrix_variables.iter().any(|id| {
                    !matches!(
                        variables.get(id.as_str()),
                        Some(SemVariableV4::Observed { .. })
                    )
                })
            {
                issues.push(issue(
                    "data.matrix.variables_invalid",
                    Some(dataset_id.clone()),
                    "Matrix variables must contain at least two unique observed-variable ids",
                ));
            }
            for (name, values, require_positive) in [
                ("means", means, false),
                ("standard_deviations", standard_deviations, true),
            ] {
                if let Some(values) = values {
                    let keys = values.keys().collect::<HashSet<_>>();
                    if keys != matrix_set
                        || values
                            .values()
                            .any(|value| !value.is_finite() || (require_positive && *value <= 0.0))
                    {
                        issues.push(issue(
                            "data.matrix.moments_invalid",
                            Some(dataset_id.clone()),
                            format!(
                                "Matrix {name} must provide one finite{} value per matrix variable",
                                if require_positive { " positive" } else { "" }
                            ),
                        ));
                    }
                }
            }
            if sample
                .effective_sample_size
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
                || sample.degrees_of_freedom.is_some_and(|value| value == 0)
            {
                issues.push(issue(
                    "data.matrix.sample_metadata_invalid",
                    Some(dataset_id.clone()),
                    "Effective sample size and degrees of freedom must be positive",
                ));
            }
            let declared_groups = match group {
                SemGroupV4::SingleGroup => None,
                SemGroupV4::ObservedGroups { levels, .. } => Some(
                    levels
                        .iter()
                        .map(|level| level.id.as_str())
                        .collect::<HashSet<_>>(),
                ),
            };
            if (!sample.group_sample_sizes.is_empty()
                && declared_groups.as_ref().is_none_or(|groups| {
                    sample
                        .group_sample_sizes
                        .keys()
                        .any(|id| !groups.contains(id.as_str()))
                }))
                || sample.group_sample_sizes.values().any(|size| *size == 0)
                || (!sample.group_sample_sizes.is_empty()
                    && sample.group_sample_sizes.values().sum::<usize>() != sample.sample_size)
            {
                issues.push(issue(
                    "data.matrix.group_sample_sizes_invalid",
                    Some(dataset_id.clone()),
                    "Group sample sizes must reference declared groups, be positive, and sum to sample_size",
                ));
            }
        }
    }
}

fn validate_annotations(
    annotations: &[SemAnnotationV4],
    variables: &HashMap<&str, &SemVariableV4>,
    issues: &mut Vec<SemModelV4Issue>,
) {
    for annotation in annotations {
        if let SemAnnotationV4::DisplayOnlyCovariance {
            id, left, right, ..
        } = annotation
        {
            if left == right
                || !variables.contains_key(left.as_str())
                || !variables.contains_key(right.as_str())
            {
                issues.push(issue(
                    "annotation.covariance.invalid",
                    Some(id.clone()),
                    "Display-only covariance must reference two different known variables",
                ));
            }
        }
    }
}

fn validate_presentation(
    presentation: &SemPresentationV4,
    variables: &HashMap<&str, &SemVariableV4>,
    relations: &[SemRelationV4],
    issues: &mut Vec<SemModelV4Issue>,
) {
    if let SemPresentationV4::Canvas {
        nodes,
        edges,
        shapes,
        images,
        lines,
        zoom,
        pan_x,
        pan_y,
    } = presentation
    {
        let relation_ids = relations
            .iter()
            .map(SemRelationV4::id)
            .collect::<HashSet<_>>();
        let mut node_ids = HashSet::new();
        for node in nodes {
            if !variables.contains_key(node.variable.as_str())
                || !node_ids.insert(node.variable.as_str())
            {
                issues.push(issue(
                    "presentation.node.invalid",
                    Some(node.variable.clone()),
                    "Canvas nodes must reference unique known variables",
                ));
            }
            if !node.x.is_finite() || !node.y.is_finite() {
                issues.push(issue(
                    "presentation.coordinate.non_finite",
                    Some(node.variable.clone()),
                    "Canvas coordinates must be finite",
                ));
            }
        }
        let mut edge_ids = HashSet::new();
        for edge in edges {
            if !relation_ids.contains(edge.relation.as_str())
                || !edge_ids.insert(edge.relation.as_str())
            {
                issues.push(issue(
                    "presentation.edge.invalid",
                    Some(edge.relation.clone()),
                    "Canvas edges must reference unique known relations",
                ));
            }
        }
        let mut decoration_ids = HashSet::new();
        for shape in shapes {
            if shape.id.trim().is_empty()
                || !decoration_ids.insert(shape.id.as_str())
                || [shape.x, shape.y, shape.width, shape.height]
                    .into_iter()
                    .any(|value| !value.is_finite())
                || shape.width <= 0.0
                || shape.height <= 0.0
            {
                issues.push(issue(
                    "presentation.shape.invalid",
                    Some(shape.id.clone()),
                    "Canvas shapes require unique ids, finite coordinates, and positive size",
                ));
            }
        }
        for image in images {
            if image.id.trim().is_empty()
                || !decoration_ids.insert(image.id.as_str())
                || image.asset_ref.trim().is_empty()
                || image.alt_text.trim().is_empty()
                || [image.x, image.y, image.width, image.height]
                    .into_iter()
                    .any(|value| !value.is_finite())
                || image.width <= 0.0
                || image.height <= 0.0
            {
                issues.push(issue(
                    "presentation.image.invalid",
                    Some(image.id.clone()),
                    "Canvas images require unique ids, an asset reference, alt text, finite coordinates, and positive size",
                ));
            }
        }
        for line in lines {
            if line.id.trim().is_empty()
                || !decoration_ids.insert(line.id.as_str())
                || [line.x1, line.y1, line.x2, line.y2]
                    .into_iter()
                    .any(|value| !value.is_finite())
                || (line.x1 == line.x2 && line.y1 == line.y2)
            {
                issues.push(issue(
                    "presentation.line.invalid",
                    Some(line.id.clone()),
                    "Canvas lines require unique ids, finite coordinates, and distinct endpoints",
                ));
            }
        }
        if [zoom, pan_x, pan_y]
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            issues.push(issue(
                "presentation.viewport.non_finite",
                None,
                "Canvas viewport values must be finite",
            ));
        }
    }
}

fn is_authoring_integrity_policy_issue(issue: &SemModelV4Issue) -> bool {
    matches!(
        issue.code.as_str(),
        "identification.factor.causal_measurement"
            | "factor.mean_policy.parameter_invalid"
            | "factor.mean_policy.group_overrides_invalid"
            | "factor.mean_policy.reference_group_invalid"
            | "factor.disturbance_policy.invalid"
            | "identification.composite.custom_weights_invalid"
    )
}

fn validate_identification(
    model: &SemModelV4,
    parameters: &HashMap<&str, &SemParameterV4>,
    issues: &mut Vec<SemModelV4Issue>,
) {
    for variable in &model.variables {
        match variable {
            SemVariableV4::CommonFactor {
                id,
                identification,
                mean_policy,
                disturbance_policy,
                ..
            } => {
                let effects = model
                    .relations
                    .iter()
                    .filter_map(|relation| match relation {
                        SemRelationV4::MeasurementEffect {
                            construct,
                            indicator,
                            parameter,
                            ..
                        } if construct == id => Some((indicator, parameter)),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if effects.len() < 2 {
                    issues.push(issue(
                        "identification.factor.indicators_insufficient",
                        Some(id.clone()),
                        "A common factor requires at least two effect indicators",
                    ));
                }
                if model.relations.iter().any(|relation| {
                    matches!(relation, SemRelationV4::MeasurementCausal { composite, .. } if composite == id)
                }) {
                    issues.push(issue(
                        "identification.factor.causal_measurement",
                        Some(id.clone()),
                        "A common factor cannot use causal-indicator measurement relations",
                    ));
                }
                match identification {
                    FactorIdentificationV4::MarkerLoading { indicator } => {
                        let marker = effects
                            .iter()
                            .find(|(candidate, _)| candidate.as_str() == indicator.as_str());
                        if marker.is_none_or(|(_, parameter)| {
                            parameters
                                .get(parameter.as_str())
                                .and_then(|parameter| parameter.fixed_value())
                                .is_none_or(|value| (value - 1.0).abs() > 1e-12)
                        }) {
                            issues.push(issue(
                                "identification.marker.invalid",
                                Some(id.clone()),
                                "Marker identification requires the selected loading fixed to one",
                            ));
                        }
                    }
                    FactorIdentificationV4::FixedVariance => {
                        let fixed = model.parameters.iter().any(|parameter| {
                            parameter.target()
                                == &SemParameterTargetV4::Variance {
                                    endpoint: SemEndpointV4::Variable(id.clone()),
                                }
                                && parameter
                                    .fixed_value()
                                    .is_some_and(|value| (value - 1.0).abs() <= 1e-12)
                        });
                        if !fixed {
                            issues.push(issue(
                                "identification.fixed_variance.missing",
                                Some(id.clone()),
                                "Fixed-variance identification requires factor variance fixed to one",
                            ));
                        }
                    }
                    FactorIdentificationV4::EffectsCoding => {
                        let loading_ids = effects
                            .iter()
                            .map(|(_, parameter)| parameter.as_str())
                            .collect::<BTreeSet<_>>();
                        let expected_value = loading_ids.len() as f64;
                        let found = model.constraints.iter().any(|constraint| match constraint {
                            SemConstraintV4::Linear { terms, value, .. } => {
                                let term_ids = terms
                                    .iter()
                                    .filter(|term| (term.coefficient - 1.0).abs() <= 1e-12)
                                    .map(|term| term.parameter.as_str())
                                    .collect::<BTreeSet<_>>();
                                term_ids == loading_ids
                                    && terms.len() == loading_ids.len()
                                    && (*value - expected_value).abs() <= 1e-12
                            }
                            _ => false,
                        });
                        if effects.len() < 3 || !found {
                            issues.push(issue(
                                "identification.effects_coding.invalid",
                                Some(id.clone()),
                                "Effects coding requires at least three loadings and an explicit sum-to-indicator-count constraint",
                            ));
                        }
                    }
                }
                match mean_policy {
                    FactorMeanPolicyV4::FixedZero => {}
                    FactorMeanPolicyV4::Estimated { parameter } => {
                        if !matches!(
                            parameters.get(parameter.as_str()),
                            Some(SemParameterV4::Free { target, .. })
                                if target == &SemParameterTargetV4::Mean { variable: id.clone() }
                        ) {
                            issues.push(issue(
                                "factor.mean_policy.parameter_invalid",
                                Some(id.clone()),
                                "Estimated factor means require a matching free mean parameter",
                            ));
                        }
                    }
                    FactorMeanPolicyV4::ReferenceGroup {
                        reference_group,
                        parameter,
                    } => {
                        let parameter_spec = parameters.get(parameter.as_str());
                        if parameter_spec.is_none_or(|parameter| {
                            parameter.target()
                                != &SemParameterTargetV4::Mean {
                                    variable: id.clone(),
                                }
                        }) {
                            issues.push(issue(
                                "factor.mean_policy.parameter_invalid",
                                Some(id.clone()),
                                "Reference-group means require a matching mean parameter",
                            ));
                        }
                        if let (SemGroupV4::ObservedGroups { levels, .. }, Some(parameter_spec)) =
                            (&model.group, parameter_spec)
                        {
                            let overrides = parameter_spec
                                .group_overrides()
                                .iter()
                                .map(|value| (value.group.as_str(), &value.specification))
                                .collect::<HashMap<_, _>>();
                            let reference_fixed_zero = matches!(
                                overrides.get(reference_group.as_str()),
                                Some(SemParameterGroupOverrideSpecV4::Fixed { value })
                                    if value.abs() <= 1e-12
                            );
                            if overrides.len() != levels.len()
                                || levels
                                    .iter()
                                    .any(|level| !overrides.contains_key(level.id.as_str()))
                                || !reference_fixed_zero
                            {
                                issues.push(issue(
                                    "factor.mean_policy.group_overrides_invalid",
                                    Some(id.clone()),
                                    "Reference-group means require an explicit override for every group and zero fixed in the reference group",
                                ));
                            }
                        }
                    }
                }
                if let FactorMeanPolicyV4::ReferenceGroup {
                    reference_group, ..
                } = mean_policy
                {
                    let valid = matches!(
                        &model.group,
                        SemGroupV4::ObservedGroups { levels, .. }
                            if levels.iter().any(|level| level.id == *reference_group)
                    );
                    if !valid {
                        issues.push(issue(
                            "factor.mean_policy.reference_group_invalid",
                            Some(id.clone()),
                            "Reference-group mean policies require a declared group level",
                        ));
                    }
                }

                let endogenous = model.relations.iter().any(|relation| {
                    matches!(relation, SemRelationV4::Structural { target, .. } if target == id)
                });
                let (disturbance_parameter, expected_endpoint, must_be_zero) =
                    match disturbance_policy {
                        FactorDisturbancePolicyV4::ExogenousVariance { parameter } => {
                            (parameter, SemEndpointV4::Variable(id.clone()), false)
                        }
                        FactorDisturbancePolicyV4::EndogenousDisturbance { parameter } => {
                            (parameter, SemEndpointV4::DisturbanceOf(id.clone()), false)
                        }
                        FactorDisturbancePolicyV4::FixedZero { parameter } => (
                            parameter,
                            if endogenous {
                                SemEndpointV4::DisturbanceOf(id.clone())
                            } else {
                                SemEndpointV4::Variable(id.clone())
                            },
                            true,
                        ),
                    };
                let disturbance_valid =
                    parameters
                        .get(disturbance_parameter.as_str())
                        .is_some_and(|parameter| {
                            parameter.target()
                                == &SemParameterTargetV4::Variance {
                                    endpoint: expected_endpoint,
                                }
                                && (!must_be_zero
                                    || parameter
                                        .fixed_value()
                                        .is_some_and(|value| value.abs() <= 1e-12))
                        });
                if !disturbance_valid
                    || matches!(
                        disturbance_policy,
                        FactorDisturbancePolicyV4::ExogenousVariance { .. }
                    ) && endogenous
                    || matches!(
                        disturbance_policy,
                        FactorDisturbancePolicyV4::EndogenousDisturbance { .. }
                    ) && !endogenous
                {
                    issues.push(issue(
                        "factor.disturbance_policy.invalid",
                        Some(id.clone()),
                        "Factor disturbance policy must match structural endogeneity and an explicit variance parameter",
                    ));
                }
            }
            SemVariableV4::Composite { id, weighting, .. } => {
                let effect_count = model
                    .relations
                    .iter()
                    .filter(|relation| matches!(relation, SemRelationV4::MeasurementEffect { construct, .. } if construct == id))
                    .count();
                let causal_count = model
                    .relations
                    .iter()
                    .filter(|relation| matches!(relation, SemRelationV4::MeasurementCausal { composite, .. } if composite == id))
                    .count();
                let valid = match weighting {
                    CompositeWeightingV4::ModeA => effect_count > 0 && causal_count == 0,
                    CompositeWeightingV4::ModeB
                    | CompositeWeightingV4::Unit { .. }
                    | CompositeWeightingV4::Custom { .. } => causal_count > 0 && effect_count == 0,
                };
                if !valid {
                    issues.push(issue(
                        "identification.composite.measurement_invalid",
                        Some(id.clone()),
                        "Mode A composites require effect indicators; Mode B, unit, and custom weighting require causal indicators",
                    ));
                }
                if let CompositeWeightingV4::Custom { weights, .. } = weighting {
                    let indicators = model
                        .relations
                        .iter()
                        .filter_map(|relation| match relation {
                            SemRelationV4::MeasurementCausal {
                                indicator,
                                composite,
                                ..
                            } if composite == id => Some(indicator),
                            _ => None,
                        })
                        .collect::<BTreeSet<_>>();
                    let weight_ids = weights.keys().collect::<BTreeSet<_>>();
                    if indicators != weight_ids
                        || weights.values().any(|weight| !weight.is_finite())
                        || weights.values().all(|weight| weight.abs() <= f64::EPSILON)
                    {
                        issues.push(issue(
                            "identification.composite.custom_weights_invalid",
                            Some(id.clone()),
                            "Custom weights must provide one finite value per causal indicator and cannot all be zero",
                        ));
                    }
                }
            }
            SemVariableV4::Observed { .. } | SemVariableV4::Derived { .. } => {}
        }
    }
}

fn canonical_endpoint_pair(
    left: SemEndpointV4,
    right: SemEndpointV4,
) -> (SemEndpointV4, SemEndpointV4) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn canonical_parameter_target(target: SemParameterTargetV4) -> SemParameterTargetV4 {
    match target {
        SemParameterTargetV4::Covariance { left, right } => {
            let pair = canonical_endpoint_pair(left, right);
            SemParameterTargetV4::Covariance {
                left: pair.0,
                right: pair.1,
            }
        }
        other => other,
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LegacyBasicModelInterpretationV4 {
    Unspecified,
    PlsComposite,
    CbsemCommonFactor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LegacyDisplayCovarianceV4 {
    pub id: String,
    pub left_construct: String,
    pub right_construct: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum LegacyBasicModelConversionErrorV4 {
    #[error(
        "legacy basic-model conversion requires an explicit PLS-composite or CB-SEM-factor interpretation"
    )]
    InterpretationRequired,
    #[error(
        "legacy basic-model conversion does not accept controls, interactions, or higher-order constructs"
    )]
    AdvancedSemantics,
    #[error("CB-SEM common-factor conversion cannot reinterpret formative construct {0}")]
    FormativeCbsem(String),
    #[error(
        "legacy indicator {indicator} is owned by both {first_construct} and {second_construct}"
    )]
    DuplicateLegacyIndicator {
        indicator: String,
        first_construct: String,
        second_construct: String,
    },
    #[error("legacy structural path references unknown construct {0}")]
    UnknownStructuralConstruct(String),
    #[error("legacy display covariance references unknown construct {0}")]
    UnknownDisplayCovarianceConstruct(String),
    #[error(transparent)]
    InvalidModel(#[from] SemModelV4ValidationError),
}

pub fn convert_legacy_basic_model_v4(
    legacy: &ModelSpec,
    interpretation: LegacyBasicModelInterpretationV4,
    display_covariances: &[LegacyDisplayCovarianceV4],
) -> Result<SemModelV4, LegacyBasicModelConversionErrorV4> {
    if interpretation == LegacyBasicModelInterpretationV4::Unspecified {
        return Err(LegacyBasicModelConversionErrorV4::InterpretationRequired);
    }
    if !legacy.controls.is_empty()
        || !legacy.interactions.is_empty()
        || !legacy.higher_order_constructs.is_empty()
    {
        return Err(LegacyBasicModelConversionErrorV4::AdvancedSemantics);
    }

    let mut legacy_indicator_owners = HashMap::<&str, &str>::new();
    for construct in &legacy.constructs {
        for indicator in &construct.indicators {
            if let Some(first_construct) =
                legacy_indicator_owners.insert(indicator.as_str(), construct.id.as_str())
            {
                return Err(
                    LegacyBasicModelConversionErrorV4::DuplicateLegacyIndicator {
                        indicator: indicator.clone(),
                        first_construct: first_construct.into(),
                        second_construct: construct.id.clone(),
                    },
                );
            }
        }
    }

    let construct_ids = legacy
        .constructs
        .iter()
        .map(|construct| (construct.id.as_str(), format!("construct:{}", construct.id)))
        .collect::<HashMap<_, _>>();
    let endogenous_constructs = legacy
        .paths
        .iter()
        .map(|path| path.target.as_str())
        .collect::<HashSet<_>>();
    let indicator_names = legacy
        .constructs
        .iter()
        .flat_map(|construct| construct.indicators.iter())
        .collect::<BTreeSet<_>>();
    let indicator_ids = indicator_names
        .iter()
        .map(|indicator| {
            (
                indicator.as_str(),
                format!("observed:{}", indicator.as_str()),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut variables = indicator_names
        .iter()
        .map(|indicator| SemVariableV4::Observed {
            id: indicator_ids[indicator.as_str()].clone(),
            label: (*indicator).clone(),
            source_column: (*indicator).clone(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Indicator,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut relations = Vec::new();
    let mut parameters = Vec::new();

    for construct in &legacy.constructs {
        let construct_id = construct_ids[construct.id.as_str()].clone();
        match interpretation {
            LegacyBasicModelInterpretationV4::PlsComposite => {
                let weighting = match construct.mode {
                    MeasurementMode::Reflective => CompositeWeightingV4::ModeA,
                    MeasurementMode::Formative => CompositeWeightingV4::ModeB,
                };
                variables.push(SemVariableV4::Composite {
                    id: construct_id.clone(),
                    label: construct.name.clone(),
                    weighting,
                });
                for indicator in &construct.indicators {
                    let indicator_id = indicator_ids[indicator.as_str()].clone();
                    let relation_id =
                        stable_sem_id("measurement", &[construct.id.as_str(), indicator.as_str()]);
                    let parameter_id =
                        stable_sem_id("parameter", &[construct.id.as_str(), indicator.as_str()]);
                    match construct.mode {
                        MeasurementMode::Reflective => {
                            relations.push(SemRelationV4::MeasurementEffect {
                                id: relation_id,
                                construct: construct_id.clone(),
                                indicator: indicator_id.clone(),
                                parameter: parameter_id.clone(),
                            });
                            parameters.push(SemParameterV4::Free {
                                id: parameter_id,
                                label: format!("{} -> {indicator}", construct.short_name),
                                target: SemParameterTargetV4::Loading {
                                    construct: construct_id.clone(),
                                    indicator: indicator_id,
                                },
                                start: None,
                                lower: None,
                                upper: None,
                                equality_label: None,
                                group_overrides: Vec::new(),
                            });
                        }
                        MeasurementMode::Formative => {
                            relations.push(SemRelationV4::MeasurementCausal {
                                id: relation_id,
                                indicator: indicator_id.clone(),
                                composite: construct_id.clone(),
                                parameter: parameter_id.clone(),
                            });
                            parameters.push(SemParameterV4::Free {
                                id: parameter_id,
                                label: format!("{indicator} -> {}", construct.short_name),
                                target: SemParameterTargetV4::Weight {
                                    indicator: indicator_id,
                                    composite: construct_id.clone(),
                                },
                                start: None,
                                lower: None,
                                upper: None,
                                equality_label: None,
                                group_overrides: Vec::new(),
                            });
                        }
                    }
                }
            }
            LegacyBasicModelInterpretationV4::CbsemCommonFactor => {
                if construct.mode == MeasurementMode::Formative {
                    return Err(LegacyBasicModelConversionErrorV4::FormativeCbsem(
                        construct.id.clone(),
                    ));
                }
                let marker = construct
                    .indicators
                    .first()
                    .map(|indicator| indicator_ids[indicator.as_str()].clone())
                    .unwrap_or_else(|| "missing-marker".into());
                let variance_id = stable_sem_id("variance", &[construct.id.as_str()]);
                let endogenous = endogenous_constructs.contains(construct.id.as_str());
                variables.push(SemVariableV4::CommonFactor {
                    id: construct_id.clone(),
                    label: construct.name.clone(),
                    identification: FactorIdentificationV4::MarkerLoading {
                        indicator: marker.clone(),
                    },
                    mean_policy: FactorMeanPolicyV4::FixedZero,
                    disturbance_policy: if endogenous {
                        FactorDisturbancePolicyV4::EndogenousDisturbance {
                            parameter: variance_id.clone(),
                        }
                    } else {
                        FactorDisturbancePolicyV4::ExogenousVariance {
                            parameter: variance_id.clone(),
                        }
                    },
                });
                for (index, indicator) in construct.indicators.iter().enumerate() {
                    let indicator_id = indicator_ids[indicator.as_str()].clone();
                    let relation_id =
                        stable_sem_id("measurement", &[construct.id.as_str(), indicator.as_str()]);
                    let parameter_id =
                        stable_sem_id("parameter", &[construct.id.as_str(), indicator.as_str()]);
                    relations.push(SemRelationV4::MeasurementEffect {
                        id: relation_id,
                        construct: construct_id.clone(),
                        indicator: indicator_id.clone(),
                        parameter: parameter_id.clone(),
                    });
                    let target = SemParameterTargetV4::Loading {
                        construct: construct_id.clone(),
                        indicator: indicator_id,
                    };
                    if index == 0 {
                        parameters.push(SemParameterV4::Fixed {
                            id: parameter_id,
                            label: format!("{} -> {indicator}", construct.short_name),
                            target,
                            value: 1.0,
                            group_overrides: Vec::new(),
                        });
                    } else {
                        parameters.push(SemParameterV4::Free {
                            id: parameter_id,
                            label: format!("{} -> {indicator}", construct.short_name),
                            target,
                            start: Some(0.7),
                            lower: None,
                            upper: None,
                            equality_label: None,
                            group_overrides: Vec::new(),
                        });
                    }
                }
                parameters.push(SemParameterV4::Free {
                    id: variance_id,
                    label: format!("Var({})", construct.short_name),
                    target: SemParameterTargetV4::Variance {
                        endpoint: if endogenous {
                            SemEndpointV4::DisturbanceOf(construct_id.clone())
                        } else {
                            SemEndpointV4::Variable(construct_id.clone())
                        },
                    },
                    start: Some(1.0),
                    lower: Some(0.0),
                    upper: None,
                    equality_label: None,
                    group_overrides: Vec::new(),
                });
            }
            LegacyBasicModelInterpretationV4::Unspecified => unreachable!(),
        }
    }

    if interpretation == LegacyBasicModelInterpretationV4::CbsemCommonFactor {
        for indicator in indicator_names {
            let indicator_id = indicator_ids[indicator.as_str()].clone();
            parameters.push(SemParameterV4::Free {
                id: stable_sem_id("residual_variance", &[indicator.as_str()]),
                label: format!("Residual variance({indicator})"),
                target: SemParameterTargetV4::Variance {
                    endpoint: SemEndpointV4::ResidualOf(indicator_id),
                },
                start: Some(0.5),
                lower: Some(0.0),
                upper: None,
                equality_label: None,
                group_overrides: Vec::new(),
            });
        }
    }

    for path in &legacy.paths {
        let source = construct_ids.get(path.source.as_str()).ok_or_else(|| {
            LegacyBasicModelConversionErrorV4::UnknownStructuralConstruct(path.source.clone())
        })?;
        let target = construct_ids.get(path.target.as_str()).ok_or_else(|| {
            LegacyBasicModelConversionErrorV4::UnknownStructuralConstruct(path.target.clone())
        })?;
        let relation_id =
            stable_sem_id("structural", &[path.source.as_str(), path.target.as_str()]);
        let parameter_id =
            stable_sem_id("regression", &[path.source.as_str(), path.target.as_str()]);
        relations.push(SemRelationV4::Structural {
            id: relation_id,
            source: source.clone(),
            target: target.clone(),
            parameter: parameter_id.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        parameters.push(SemParameterV4::Free {
            id: parameter_id,
            label: format!("{} -> {}", path.source, path.target),
            target: SemParameterTargetV4::Regression {
                source: source.clone(),
                target: target.clone(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
    }

    let mut annotations = Vec::new();
    for covariance in display_covariances {
        let left = construct_ids
            .get(covariance.left_construct.as_str())
            .ok_or_else(|| {
                LegacyBasicModelConversionErrorV4::UnknownDisplayCovarianceConstruct(
                    covariance.left_construct.clone(),
                )
            })?;
        let right = construct_ids
            .get(covariance.right_construct.as_str())
            .ok_or_else(|| {
                LegacyBasicModelConversionErrorV4::UnknownDisplayCovarianceConstruct(
                    covariance.right_construct.clone(),
                )
            })?;
        annotations.push(SemAnnotationV4::DisplayOnlyCovariance {
            id: covariance.id.clone(),
            left: left.clone(),
            right: right.clone(),
            label: covariance.label.clone(),
        });
    }

    let model = SemModelV4 {
        schema_version: SEM_MODEL_V4_SCHEMA_VERSION,
        id: legacy.id.to_string(),
        name: legacy.name.clone(),
        variables,
        relations,
        parameters,
        constraints: Vec::new(),
        derived_terms: Vec::new(),
        group: SemGroupV4::SingleGroup,
        data_binding: SemDataBindingV4::Raw {
            dataset_id: "legacy-unbound".into(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        },
        annotations,
        presentation: SemPresentationV4::None,
    }
    .canonicalized();
    model.ensure_valid()?;
    Ok(model)
}

fn stable_sem_id(prefix: &str, parts: &[&str]) -> String {
    let encoded = parts
        .iter()
        .map(|part| hex_bytes(part.as_bytes()))
        .collect::<Vec<_>>()
        .join("_");
    format!("{prefix}_{encoded}")
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Construct, StructuralPath};
    use uuid::Uuid;

    fn legacy_model() -> ModelSpec {
        ModelSpec {
            id: Uuid::nil(),
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
            controls: vec![],
            higher_order_constructs: vec![],
            interactions: vec![],
        }
    }

    fn add_free_structural_relation(model: &mut SemModelV4, id: &str, source: &str, target: &str) {
        let parameter = format!("parameter:{id}");
        model.relations.push(SemRelationV4::Structural {
            id: id.into(),
            source: source.into(),
            target: target.into(),
            parameter: parameter.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: parameter,
            label: format!("{source} -> {target}"),
            target: SemParameterTargetV4::Regression {
                source: source.into(),
                target: target.into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
    }

    fn add_interaction_v2(
        model: &mut SemModelV4,
        id: &str,
        output: &str,
        operands: &[&str],
        focal_relation: &str,
        hierarchy_policy: InteractionHierarchyPolicyV2,
    ) {
        model.variables.push(SemVariableV4::Derived {
            id: output.into(),
            label: operands.join(" x "),
        });
        add_free_structural_relation(
            model,
            &format!("relation:{id}:effect"),
            output,
            "construct:y",
        );
        model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
            id: id.into(),
            output: output.into(),
            operands: operands.iter().map(|operand| (*operand).into()).collect(),
            focal_relation: focal_relation.into(),
            method: InteractionMethodV4::TwoStage,
            hierarchy_policy,
            product_indicator: None,
        });
    }

    #[test]
    fn roundtrip_and_order_determinism_are_stable() {
        let model = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        let encoded = serde_json::to_vec(&model).unwrap();
        let decoded: SemModelV4 = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, model);

        let mut reordered = model.clone();
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        assert_eq!(
            model.scientific_hash_input().unwrap(),
            reordered.scientific_hash_input().unwrap()
        );
        assert_eq!(
            model.model_document_hash_input().unwrap(),
            reordered.model_document_hash_input().unwrap()
        );
    }

    #[test]
    fn structural_relation_role_default_preserves_existing_wire_and_digest() {
        let model = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        let wire = serde_json::to_value(&model).unwrap();
        let structural = wire["relations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|relation| relation["kind"] == "structural")
            .unwrap();
        assert!(structural.get("role").is_none());
        assert_eq!(
            model.scientific_sha256().unwrap(),
            "115e88da1579ad3fcf7216f734fa5a2a1046acc47026e1dabbe2bc55a19e354f"
        );

        let mut explicit_default = wire;
        let structural = explicit_default["relations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|relation| relation["kind"] == "structural")
            .unwrap();
        structural["role"] = serde_json::json!("structural");
        let decoded: SemModelV4 = serde_json::from_value(explicit_default).unwrap();
        assert_eq!(decoded, model);
        assert_eq!(
            decoded.scientific_hash_input().unwrap(),
            model.scientific_hash_input().unwrap()
        );
        let reencoded = serde_json::to_value(&decoded).unwrap();
        let structural = reencoded["relations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|relation| relation["kind"] == "structural")
            .unwrap();
        assert!(structural.get("role").is_none());
    }

    #[test]
    fn latent_control_relation_roundtrips_and_changes_scientific_identity() {
        let mut model = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        let structural = model
            .relations
            .iter_mut()
            .find(|relation| matches!(relation, SemRelationV4::Structural { .. }))
            .unwrap();
        let relation_id = structural.id().to_owned();
        let SemRelationV4::Structural { role, .. } = structural else {
            unreachable!()
        };
        *role = StructuralRelationRoleV4::Control;
        assert!(model.validate().is_empty());

        let control_hash = model.scientific_sha256().unwrap();
        let wire = serde_json::to_vec(&model).unwrap();
        assert!(String::from_utf8_lossy(&wire).contains("\"role\":\"control\""));
        let decoded: SemModelV4 = serde_json::from_slice(&wire).unwrap();
        assert_eq!(decoded, model);
        let mut default_role = model.clone();
        let SemRelationV4::Structural { role, .. } = default_role
            .relations
            .iter_mut()
            .find(|relation| relation.id() == relation_id)
            .unwrap()
        else {
            unreachable!()
        };
        *role = StructuralRelationRoleV4::Structural;
        assert_ne!(control_hash, default_role.scientific_sha256().unwrap());

        let compiled = crate::compile_cbsem_plan_v2(&model).unwrap();
        assert_eq!(
            compiled
                .regressions()
                .iter()
                .find(|regression| regression.relation_id() == relation_id)
                .unwrap()
                .role(),
            StructuralRelationRoleV4::Control
        );
        assert_eq!(
            compiled
                .to_scientific_sem_model_v4()
                .scientific_sha256()
                .unwrap(),
            control_hash
        );
    }

    #[test]
    fn presentation_and_display_annotations_do_not_change_scientific_identity() {
        let base = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        let mut decorated = base.clone();
        decorated.presentation = SemPresentationV4::Canvas {
            nodes: vec![SemCanvasNodeV4 {
                variable: "construct:x".into(),
                x: 100.0,
                y: 200.0,
                style: BTreeMap::new(),
            }],
            edges: vec![],
            shapes: vec![SemCanvasShapeV4 {
                id: "shape-background".into(),
                shape: SemCanvasShapeKindV4::RoundedRectangle,
                x: 50.0,
                y: 75.0,
                width: 300.0,
                height: 200.0,
                label: Some("Measurement model".into()),
                style: BTreeMap::new(),
            }],
            images: vec![SemCanvasImageV4 {
                id: "image-logo".into(),
                asset_ref: "project-asset:logo".into(),
                alt_text: "Project logo".into(),
                x: 25.0,
                y: 25.0,
                width: 64.0,
                height: 64.0,
                style: BTreeMap::new(),
            }],
            lines: vec![SemCanvasLineV4 {
                id: "line-divider".into(),
                x1: 0.0,
                y1: 50.0,
                x2: 500.0,
                y2: 50.0,
                label: None,
                start_marker: None,
                end_marker: None,
                style: BTreeMap::new(),
            }],
            zoom: Some(1.5),
            pan_x: Some(10.0),
            pan_y: Some(20.0),
        };
        decorated
            .annotations
            .push(SemAnnotationV4::DisplayOnlyCovariance {
                id: "display-cov".into(),
                left: "construct:x".into(),
                right: "construct:y".into(),
                label: None,
            });
        assert_eq!(
            base.scientific_sha256().unwrap(),
            decorated.scientific_sha256().unwrap()
        );
        assert_ne!(
            base.model_document_sha256().unwrap(),
            decorated.model_document_sha256().unwrap()
        );
        let mut invalid_presentation = decorated;
        if let SemPresentationV4::Canvas { shapes, .. } = &mut invalid_presentation.presentation {
            shapes[0].width = -1.0;
        }
        assert!(
            invalid_presentation
                .validate()
                .iter()
                .any(|issue| issue.code == "presentation.shape.invalid")
        );
    }

    #[test]
    fn authoring_integrity_allows_underidentified_draft_but_rejects_dangling_references() {
        let ready = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        let mut dangling_marker = ready.clone();
        let marker = dangling_marker
            .variables
            .iter_mut()
            .find_map(|variable| match variable {
                SemVariableV4::CommonFactor {
                    identification: FactorIdentificationV4::MarkerLoading { indicator },
                    ..
                } => Some(indicator),
                _ => None,
            })
            .unwrap();
        *marker = "observed:missing-marker".into();
        assert!(
            dangling_marker
                .validate_authoring_integrity()
                .iter()
                .any(|issue| { issue.code == "authoring.marker.reference_invalid" })
        );

        let mut draft = ready;
        let factor = draft
            .variables
            .iter_mut()
            .find_map(|variable| match variable {
                SemVariableV4::CommonFactor { identification, .. } => Some(identification),
                _ => None,
            })
            .unwrap();
        *factor = FactorIdentificationV4::FixedVariance;

        draft.ensure_authoring_integrity().unwrap();
        assert!(
            draft
                .validate()
                .iter()
                .any(|issue| { issue.code == "identification.fixed_variance.missing" })
        );
        assert_eq!(
            draft.model_document_sha256().unwrap(),
            draft.model_document_sha256().unwrap()
        );

        let parameter = draft
            .relations
            .iter_mut()
            .find_map(|relation| match relation {
                SemRelationV4::MeasurementEffect { parameter, .. } => Some(parameter),
                _ => None,
            })
            .expect("converted common-factor fixture must contain a measurement effect");
        *parameter = "missing-parameter".into();
        assert!(
            draft
                .validate_authoring_integrity()
                .iter()
                .any(|issue| { issue.code == "relation.parameter.unknown" })
        );
        assert!(draft.model_document_sha256().is_err());
    }

    #[test]
    fn duplicate_ids_and_invalid_constraints_fail_validation() {
        let mut model = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        let duplicate = model.variables[0].clone();
        model.variables.push(duplicate);
        model.constraints.push(SemConstraintV4::Equality {
            id: "bad-equality".into(),
            parameters: vec!["missing".into(), "missing".into()],
        });
        let codes = model
            .validate()
            .into_iter()
            .map(|value| value.code)
            .collect::<HashSet<_>>();
        assert!(codes.contains("object.id.duplicate"));
        assert!(codes.contains("constraint.equality.invalid"));
        assert!(codes.contains("constraint.parameter.unknown"));
    }

    #[test]
    fn migration_requires_explicit_estimand_and_preserves_display_covariance_only() {
        assert_eq!(
            convert_legacy_basic_model_v4(
                &legacy_model(),
                LegacyBasicModelInterpretationV4::Unspecified,
                &[],
            ),
            Err(LegacyBasicModelConversionErrorV4::InterpretationRequired)
        );
        let model = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[LegacyDisplayCovarianceV4 {
                id: "legacy-display".into(),
                left_construct: "x".into(),
                right_construct: "y".into(),
                label: Some("visual".into()),
            }],
        )
        .unwrap();
        assert_eq!(model.annotations.len(), 1);
        assert!(
            !model
                .relations
                .iter()
                .any(|relation| matches!(relation, SemRelationV4::Covariance { .. }))
        );
    }

    #[test]
    fn structural_feedback_is_representable_without_being_a_schema_error() {
        let mut model = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.relations.push(SemRelationV4::Structural {
            id: "feedback".into(),
            source: "construct:y".into(),
            target: "construct:x".into(),
            parameter: "feedback-parameter".into(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: "feedback-parameter".into(),
            label: "Y -> X".into(),
            target: SemParameterTargetV4::Regression {
                source: "construct:y".into(),
                target: "construct:x".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        let x_variance_parameter = match model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:x")
            .unwrap()
        {
            SemVariableV4::CommonFactor {
                disturbance_policy, ..
            } => match disturbance_policy {
                FactorDisturbancePolicyV4::ExogenousVariance { parameter } => {
                    let parameter = parameter.clone();
                    *disturbance_policy = FactorDisturbancePolicyV4::EndogenousDisturbance {
                        parameter: parameter.clone(),
                    };
                    parameter
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };
        match model
            .parameters
            .iter_mut()
            .find(|parameter| parameter.id() == x_variance_parameter)
            .unwrap()
        {
            SemParameterV4::Free { target, .. } => {
                *target = SemParameterTargetV4::Variance {
                    endpoint: SemEndpointV4::DisturbanceOf("construct:x".into()),
                };
            }
            _ => unreachable!(),
        }
        assert!(model.validate().is_empty());
        assert!(model.has_structural_feedback());
    }

    #[test]
    fn observed_metadata_control_identifier_and_strict_serde_roundtrip() {
        let mut model = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.variables.push(SemVariableV4::Observed {
            id: "observed:segment".into(),
            label: "Segment".into(),
            source_column: "segment_code".into(),
            scale: ObservedScaleV4::Nominal,
            role: ObservedRoleV4::Control,
            categories: vec!["A".into(), "B".into()],
            value_labels: BTreeMap::from([
                ("A".into(), "Enterprise".into()),
                ("B".into(), "Consumer".into()),
            ]),
            missing_markers: vec!["-9".into()],
            transformation_lineage: vec![ObservedTransformationStepV4 {
                id: "recode-segment".into(),
                input_columns: vec!["segment_raw".into()],
                output_column: "segment_code".into(),
                operation: ObservedTransformationOperationV4::Recode {
                    mappings: BTreeMap::from([
                        ("enterprise".into(), "A".into()),
                        ("consumer".into(), "B".into()),
                    ]),
                    unmapped: RecodeUnmappedPolicyV4::SetMissing,
                },
            }],
        });
        model.variables.push(SemVariableV4::Observed {
            id: "observed:case_id".into(),
            label: "Case id".into(),
            source_column: "case_id".into(),
            scale: ObservedScaleV4::Identifier,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        assert!(model.validate().is_empty(), "{:?}", model.validate());
        let encoded = serde_json::to_value(&model).unwrap();
        assert_eq!(
            serde_json::from_value::<SemModelV4>(encoded.clone()).unwrap(),
            model
        );

        let mut unknown_top = encoded.clone();
        unknown_top
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::json!(true));
        assert!(serde_json::from_value::<SemModelV4>(unknown_top).is_err());

        let mut unknown_nested = encoded;
        unknown_nested["variables"][0]
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::json!(true));
        assert!(serde_json::from_value::<SemModelV4>(unknown_nested).is_err());

        let mut invalid_use = model;
        invalid_use.relations.push(SemRelationV4::Structural {
            id: "identifier-outcome".into(),
            source: "construct:x".into(),
            target: "observed:case_id".into(),
            parameter: "identifier-outcome-p".into(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        invalid_use.parameters.push(SemParameterV4::Free {
            id: "identifier-outcome-p".into(),
            label: "X -> case id".into(),
            target: SemParameterTargetV4::Regression {
                source: "construct:x".into(),
                target: "observed:case_id".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        let codes = invalid_use
            .validate()
            .into_iter()
            .map(|issue| issue.code)
            .collect::<HashSet<_>>();
        assert!(codes.contains("observed.role.usage_invalid"));
        assert!(codes.contains("observed.identifier.model_use_invalid"));
    }

    #[test]
    fn custom_composite_weighting_is_preserved_by_the_basic_pls_compiler() {
        let mut legacy = legacy_model();
        legacy.constructs[0].mode = MeasurementMode::Formative;
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let weights = BTreeMap::from([("observed:x1".into(), 0.4), ("observed:x2".into(), 0.6)]);
        match model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:x")
            .unwrap()
        {
            SemVariableV4::Composite { weighting, .. } => {
                *weighting = CompositeWeightingV4::Custom {
                    weights,
                    normalization: CompositeWeightNormalizationV4::SumToOne,
                };
            }
            _ => unreachable!(),
        }
        assert!(model.validate().is_empty(), "{:?}", model.validate());
        let plan = crate::compile_pls_plan_v2(&model).unwrap();
        let block = plan
            .blocks()
            .iter()
            .find(|block| block.construct_id() == "construct:x")
            .unwrap();
        assert_eq!(
            block.fixed_scoring(),
            Some(&crate::CompiledPlsFixedScoringV2::Custom {
                weights: BTreeMap::from(
                    [("observed:x1".into(), 0.4), ("observed:x2".into(), 0.6),]
                ),
                normalization: CompositeWeightNormalizationV4::SumToOne,
            })
        );
        let mut invalid = model;
        if let SemVariableV4::Composite {
            weighting: CompositeWeightingV4::Custom { weights, .. },
            ..
        } = invalid
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:x")
            .unwrap()
        {
            weights.remove("observed:x2");
        }
        assert!(
            invalid
                .validate()
                .iter()
                .any(|issue| { issue.code == "identification.composite.custom_weights_invalid" })
        );
    }

    #[test]
    fn factor_policies_and_group_overrides_are_explicit_and_validated() {
        let mut model = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.parameters.push(SemParameterV4::Free {
            id: "factor-x-mean".into(),
            label: "Mean(X)".into(),
            target: SemParameterTargetV4::Mean {
                variable: "construct:x".into(),
            },
            start: Some(0.0),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        match model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:x")
            .unwrap()
        {
            SemVariableV4::CommonFactor { mean_policy, .. } => {
                *mean_policy = FactorMeanPolicyV4::Estimated {
                    parameter: "factor-x-mean".into(),
                };
            }
            _ => unreachable!(),
        }
        assert!(model.validate().is_empty(), "{:?}", model.validate());
        let plan = crate::compile_cbsem_plan_v2(&model).unwrap();
        assert!(
            crate::validate_cbsem_ml_v1_estimator_capability_v2(&plan)
                .iter()
                .any(|issue| issue.code == "factor_mean_or_disturbance_policy")
        );

        let mut grouped = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
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
        grouped.parameters.push(SemParameterV4::Free {
            id: "group-factor-x-mean".into(),
            label: "Group mean(X)".into(),
            target: SemParameterTargetV4::Mean {
                variable: "construct:x".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: vec![
                SemParameterGroupOverrideV4 {
                    group: "a".into(),
                    specification: SemParameterGroupOverrideSpecV4::Fixed { value: 0.0 },
                },
                SemParameterGroupOverrideV4 {
                    group: "b".into(),
                    specification: SemParameterGroupOverrideSpecV4::Free {
                        start: Some(0.0),
                        lower: None,
                        upper: None,
                    },
                },
            ],
        });
        if let SemVariableV4::CommonFactor { mean_policy, .. } = grouped
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:x")
            .unwrap()
        {
            *mean_policy = FactorMeanPolicyV4::ReferenceGroup {
                reference_group: "a".into(),
                parameter: "group-factor-x-mean".into(),
            };
        }
        assert!(grouped.validate().is_empty(), "{:?}", grouped.validate());
        let mut invalid = grouped.clone();
        if let SemParameterV4::Free {
            group_overrides, ..
        } = invalid
            .parameters
            .iter_mut()
            .find(|parameter| parameter.id() == "group-factor-x-mean")
            .unwrap()
        {
            group_overrides[0].group = "missing".into();
        }
        assert!(
            invalid
                .validate()
                .iter()
                .any(|issue| issue.code == "parameter.group_override.group_invalid")
        );
    }

    #[test]
    fn raw_weight_kinds_and_matrix_metadata_are_preserved_or_rejected_explicitly() {
        let mut pls = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        pls.variables.push(SemVariableV4::Observed {
            id: "observed:weight".into(),
            label: "Survey weight".into(),
            source_column: "survey_weight".into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        let weights = [
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
        for weight in weights {
            let mut weighted = pls.clone();
            if let SemDataBindingV4::Raw {
                weight: configured, ..
            } = &mut weighted.data_binding
            {
                *configured = Some(weight);
            }
            assert!(weighted.validate().is_empty(), "{:?}", weighted.validate());
            assert!(matches!(
                crate::compile_pls_plan_v2(&weighted),
                Err(crate::CompiledPlsPlanV2Error::UnsupportedWeight(issue))
                    if issue.declaration.as_ref().is_some_and(|declaration|
                        declaration.binding().variable_id() == "observed:weight"
                    )
            ));
        }

        let mut cbsem = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        let variables = vec![
            "observed:x1".into(),
            "observed:x2".into(),
            "observed:y1".into(),
            "observed:y2".into(),
        ];
        cbsem.data_binding = SemDataBindingV4::Covariance {
            dataset_id: "covariance-input".into(),
            variables: variables.clone(),
            means: Some(BTreeMap::from([
                ("observed:x1".into(), 1.0),
                ("observed:x2".into(), 2.0),
                ("observed:y1".into(), 3.0),
                ("observed:y2".into(), 4.0),
            ])),
            standard_deviations: Some(BTreeMap::from([
                ("observed:x1".into(), 1.1),
                ("observed:x2".into(), 1.2),
                ("observed:y1".into(), 1.3),
                ("observed:y2".into(), 1.4),
            ])),
            sample: SemMatrixSampleMetadataV4 {
                sample_size: 200,
                covariance_denominator: SemCovarianceDenominatorV4::SampleNMinusOne,
                effective_sample_size: Some(184.5),
                degrees_of_freedom: Some(199),
                group_sample_sizes: BTreeMap::new(),
            },
        };
        assert!(cbsem.validate().is_empty(), "{:?}", cbsem.validate());
        let plan = crate::compile_cbsem_plan_v2(&cbsem).unwrap();
        let issue_codes = crate::validate_cbsem_ml_v1_estimator_capability_v2(&plan)
            .into_iter()
            .map(|issue| issue.code)
            .collect::<HashSet<_>>();
        assert!(issue_codes.contains("matrix_means_unsupported"));
        assert!(issue_codes.contains("covariance_scale_metadata_unsupported"));
        assert!(issue_codes.contains("matrix_effective_sample_size_unsupported"));
        assert!(issue_codes.contains("matrix_degrees_of_freedom_unsupported"));

        cbsem.data_binding = SemDataBindingV4::Correlation {
            dataset_id: "correlation-input".into(),
            variables: variables.clone(),
            means: None,
            standard_deviations: None,
            sample: SemMatrixSampleMetadataV4 {
                sample_size: 200,
                covariance_denominator: SemCovarianceDenominatorV4::SampleNMinusOne,
                effective_sample_size: None,
                degrees_of_freedom: None,
                group_sample_sizes: BTreeMap::new(),
            },
        };
        let plan = crate::compile_cbsem_plan_v2(&cbsem).unwrap();
        assert!(
            crate::validate_cbsem_ml_v1_estimator_capability_v2(&plan)
                .iter()
                .any(|issue| issue.code == "correlation_scale_metadata_required")
        );
        assert!(matches!(
            plan.input(),
            crate::CompiledCbsemInputV2::Correlation {
                sample,
                variables: compiled_variables,
                ..
            } if sample.sample_size == 200 && compiled_variables == &variables
        ));
    }

    #[test]
    fn moderation_focal_path_and_all_hoc_approaches_are_explicit() {
        let mut legacy = legacy_model();
        legacy.constructs.push(Construct {
            id: "m".into(),
            name: "Moderator".into(),
            short_name: "M".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["m1".into(), "m2".into()],
        });
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let focal_relation = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == "construct:x" && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        model.variables.push(SemVariableV4::Derived {
            id: "derived:x-by-m".into(),
            label: "X x M".into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: "interaction-effect".into(),
            source: "derived:x-by-m".into(),
            target: "construct:y".into(),
            parameter: "interaction-effect-p".into(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: "interaction-effect-p".into(),
            label: "X x M -> Y".into(),
            target: SemParameterTargetV4::Regression {
                source: "derived:x-by-m".into(),
                target: "construct:y".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.derived_terms.push(SemDerivedTermV4::Interaction {
            id: "interaction-x-m".into(),
            output: "derived:x-by-m".into(),
            predictor: "construct:x".into(),
            moderator: "construct:m".into(),
            focal_relation: focal_relation.clone(),
            method: InteractionMethodV4::TwoStage,
            product_indicator: None,
        });
        assert!(model.validate().is_empty(), "{:?}", model.validate());
        let mut invalid = model.clone();
        if let SemDerivedTermV4::Interaction { focal_relation, .. } = &mut invalid.derived_terms[0]
        {
            *focal_relation = "missing".into();
        }
        assert!(
            invalid
                .validate()
                .iter()
                .any(|issue| issue.code == "derived.interaction.focal_relation_unknown")
        );

        let approaches = [
            HigherOrderConstructionApproachV4::RepeatedIndicators,
            HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators,
            HigherOrderConstructionApproachV4::EmbeddedTwoStage,
            HigherOrderConstructionApproachV4::DisjointTwoStage,
            HigherOrderConstructionApproachV4::Hybrid,
        ];
        for approach in approaches {
            let encoded = serde_json::to_string(&approach).unwrap();
            assert_eq!(
                serde_json::from_str::<HigherOrderConstructionApproachV4>(&encoded).unwrap(),
                approach
            );
        }
        let higher_order = SemDerivedTermV4::HigherOrder {
            id: "hoc".into(),
            output: "derived:hoc".into(),
            components: vec!["construct:x".into(), "construct:y".into()],
            approach: HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators,
            measurement_type: HigherOrderMeasurementTypeV4::ReflectiveFormative,
        };
        assert!(
            serde_json::to_string(&higher_order)
                .unwrap()
                .contains("extended_repeated_indicators")
        );
    }

    #[test]
    fn interaction_v2_preserves_operand_order_and_validates_strong_three_way_hierarchy() {
        let mut legacy = legacy_model();
        for (id, name) in [("m", "Moderator M"), ("w", "Moderator W")] {
            legacy.constructs.push(Construct {
                id: id.into(),
                name: name.into(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}1"), format!("{id}2")],
            });
        }
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let focal_x = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == "construct:x" && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        add_free_structural_relation(
            &mut model,
            "relation:main:m:y",
            "construct:m",
            "construct:y",
        );
        add_free_structural_relation(
            &mut model,
            "relation:main:w:y",
            "construct:w",
            "construct:y",
        );
        add_interaction_v2(
            &mut model,
            "term:x:m",
            "derived:x:m",
            &["construct:x", "construct:m"],
            &focal_x,
            InteractionHierarchyPolicyV2::Strong,
        );
        add_interaction_v2(
            &mut model,
            "term:x:w",
            "derived:x:w",
            &["construct:x", "construct:w"],
            &focal_x,
            InteractionHierarchyPolicyV2::Strong,
        );
        add_interaction_v2(
            &mut model,
            "term:m:w",
            "derived:m:w",
            &["construct:m", "construct:w"],
            "relation:main:m:y",
            InteractionHierarchyPolicyV2::Strong,
        );
        add_interaction_v2(
            &mut model,
            "term:x:m:w",
            "derived:x:m:w",
            &["construct:x", "construct:m", "construct:w"],
            &focal_x,
            InteractionHierarchyPolicyV2::Strong,
        );
        model.ensure_valid().unwrap();

        let encoded = serde_json::to_value(&model).unwrap();
        let top = encoded["derived_terms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|term| term["id"] == "term:x:m:w")
            .unwrap();
        assert_eq!(top["kind"], "interaction_v2");
        assert_eq!(
            top["operands"],
            serde_json::json!(["construct:x", "construct:m", "construct:w"])
        );
        let decoded: SemModelV4 = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, model);

        let original_digest = model.scientific_sha256().unwrap();
        let mut reordered_operands = model.clone();
        let SemDerivedTermV4::InteractionV2 { operands, .. } = reordered_operands
            .derived_terms
            .iter_mut()
            .find(|term| term.id() == "term:x:m:w")
            .unwrap()
        else {
            unreachable!()
        };
        operands.swap(1, 2);
        assert_ne!(
            reordered_operands.scientific_sha256().unwrap(),
            original_digest
        );

        let mut missing_lower_order = model.clone();
        missing_lower_order
            .derived_terms
            .retain(|term| term.id() != "term:m:w");
        missing_lower_order
            .variables
            .retain(|variable| variable.id() != "derived:m:w");
        missing_lower_order.relations.retain(|relation| {
            !matches!(relation, SemRelationV4::Structural { source, .. } if source == "derived:m:w")
        });
        missing_lower_order.parameters.retain(|parameter| {
            !matches!(parameter.target(), SemParameterTargetV4::Regression { source, .. } if source == "derived:m:w")
        });
        assert!(missing_lower_order.validate().iter().any(|issue| {
            issue.code == "derived.interaction_v2.lower_order_missing"
                && issue.subject.as_deref() == Some("term:x:m:w")
        }));

        let SemDerivedTermV4::InteractionV2 {
            hierarchy_policy, ..
        } = missing_lower_order
            .derived_terms
            .iter_mut()
            .find(|term| term.id() == "term:x:m:w")
            .unwrap()
        else {
            unreachable!()
        };
        *hierarchy_policy = InteractionHierarchyPolicyV2::Weak;
        missing_lower_order.ensure_valid().unwrap();

        let mut duplicate = model;
        let SemDerivedTermV4::InteractionV2 { operands, .. } = duplicate
            .derived_terms
            .iter_mut()
            .find(|term| term.id() == "term:x:m:w")
            .unwrap()
        else {
            unreachable!()
        };
        operands[2] = operands[1].clone();
        assert!(
            duplicate
                .validate()
                .iter()
                .any(|issue| issue.code == "derived.interaction_v2.operands_invalid")
        );
    }

    #[test]
    fn free_parameter_start_and_equality_label_validation_is_typed() {
        let mut model = convert_legacy_basic_model_v4(
            &legacy_model(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        let parameter = model
            .parameters
            .iter_mut()
            .find(|parameter| matches!(parameter, SemParameterV4::Free { .. }))
            .unwrap();
        let SemParameterV4::Free {
            start,
            lower,
            upper,
            equality_label,
            ..
        } = parameter
        else {
            unreachable!()
        };
        *start = Some(2.0);
        *lower = Some(-1.0);
        *upper = Some(1.0);
        *equality_label = Some("   ".into());
        let issues = model.validate();
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "parameter.start.outside_bounds")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "parameter.equality_label.empty")
        );
    }
}
