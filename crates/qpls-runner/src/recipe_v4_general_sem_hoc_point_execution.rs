use crate::{
    RecipeV4PlsExecutionError, RecipeV4PlsExecutionResultV1, RunnerProgress,
    run_compiled_pls_recipe_v4_allowing_isolated,
};
use qpls_assessment::{AssessmentError, variance_inflation_factor_with_control};
use qpls_core::{
    AnalysisRecipeModelBindingV4, AnalysisRecipeV4, CompiledAnalysisRecipeV4,
    CompiledPlsHocComponentRelationInterpretationV1, CompiledPlsHocExecutionProjectionV1,
    CompiledPlsHocStageRoleV1, CompiledPlsPlanV3, CompiledRecipePlanV4,
    HigherOrderConstructionApproachV4, HigherOrderMeasurementTypeV4, RecipeV4CompilationError,
    RecipeV4CompilerTarget, SemModelV4, StructuralRelationRoleV4, compile_analysis_recipe_v4,
    compile_pls_higher_order_repeated_stage_projection_v1,
    compile_pls_higher_order_score_stage_projection_v1, project_general_sem_pls_base_recipe_v1,
    sha256_serialized,
};
use qpls_data::Dataset;
use qpls_estimation::{
    GENERAL_SEM_PLS_DISJOINT_HOC_SCORE_DATASET_RECEIPT_VERSION_V1,
    GeneralSemPlsDisjointHocScoreDatasetErrorV1, GeneralSemPlsDisjointHocScoreDatasetReceiptV1,
    PlsAliasColumnSpecV1, PlsResult, append_pls_alias_columns_v1,
    prepare_general_sem_pls_disjoint_hoc_score_dataset_v1,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const GENERAL_SEM_PLS_HIGHER_ORDER_POINT_METHOD_VERSION_V1: &str =
    qpls_core::PLS_GENERAL_HIGHER_ORDER_POINT_CAPABILITY_VERSION_V1;
pub const GENERAL_SEM_PLS_DISJOINT_HIGHER_ORDER_POINT_METHOD_VERSION_V1: &str =
    GENERAL_SEM_PLS_HIGHER_ORDER_POINT_METHOD_VERSION_V1;
pub const GENERAL_SEM_PLS_HIGHER_ORDER_POINT_STAGE_RECEIPT_VERSION_V1: &str =
    "general_sem_pls_higher_order_point_stage_receipt_v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemPlsHocPointRelationKindV1 {
    ComponentLoading,
    ComponentWeight,
    AuthoredStructural,
    AuthoredControl,
    TechnicalStructural,
    ExtendedIndirectEffect,
    ExtendedTotalEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsHocPointRelationEstimateV1 {
    relation_id: String,
    parameter_id: String,
    source_id: String,
    target_id: String,
    kind: GeneralSemPlsHocPointRelationKindV1,
    estimate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    generated_score_variable_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    collinearity_vif: Option<f64>,
}

impl GeneralSemPlsHocPointRelationEstimateV1 {
    pub fn relation_id(&self) -> &str {
        &self.relation_id
    }

    pub fn parameter_id(&self) -> &str {
        &self.parameter_id
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn kind(&self) -> GeneralSemPlsHocPointRelationKindV1 {
        self.kind
    }

    pub fn estimate(&self) -> f64 {
        self.estimate
    }

    pub fn generated_score_variable_id(&self) -> Option<&str> {
        self.generated_score_variable_id.as_deref()
    }

    pub fn collinearity_vif(&self) -> Option<f64> {
        self.collinearity_vif
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsHocPointStageReceiptV1 {
    receipt_version: String,
    stage_number: u8,
    role: CompiledPlsHocStageRoleV1,
    projection_identity_sha256: String,
    model_scientific_sha256: String,
    compiled_plan_sha256: String,
    dataset_fingerprint: String,
    used_observations: usize,
    omitted_observations: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    generated_score_dataset: Option<GeneralSemPlsDisjointHocScoreDatasetReceiptV1>,
}

impl GeneralSemPlsHocPointStageReceiptV1 {
    pub fn receipt_version(&self) -> &str {
        &self.receipt_version
    }

    pub fn stage_number(&self) -> u8 {
        self.stage_number
    }

    pub fn role(&self) -> CompiledPlsHocStageRoleV1 {
        self.role
    }

    pub fn projection_identity_sha256(&self) -> &str {
        &self.projection_identity_sha256
    }

    pub fn model_scientific_sha256(&self) -> &str {
        &self.model_scientific_sha256
    }

    pub fn compiled_plan_sha256(&self) -> &str {
        &self.compiled_plan_sha256
    }

    pub fn dataset_fingerprint(&self) -> &str {
        &self.dataset_fingerprint
    }

    pub fn used_observations(&self) -> usize {
        self.used_observations
    }

    pub fn omitted_observations(&self) -> usize {
        self.omitted_observations
    }

    pub fn generated_score_dataset(
        &self,
    ) -> Option<&GeneralSemPlsDisjointHocScoreDatasetReceiptV1> {
        self.generated_score_dataset.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsHocPointStageResultV1 {
    stage_id: String,
    input_construct_ids: Vec<String>,
    output_variable_ids: Vec<String>,
    relation_estimates: Vec<GeneralSemPlsHocPointRelationEstimateV1>,
    receipt: GeneralSemPlsHocPointStageReceiptV1,
}

impl GeneralSemPlsHocPointStageResultV1 {
    pub fn stage_id(&self) -> &str {
        &self.stage_id
    }

    pub fn input_construct_ids(&self) -> &[String] {
        &self.input_construct_ids
    }

    pub fn output_variable_ids(&self) -> &[String] {
        &self.output_variable_ids
    }

    pub fn relation_estimates(&self) -> &[GeneralSemPlsHocPointRelationEstimateV1] {
        &self.relation_estimates
    }

    pub fn receipt(&self) -> &GeneralSemPlsHocPointStageReceiptV1 {
        &self.receipt
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsHigherOrderPointResultV1 {
    method_version: String,
    hoc_stage_plan_sha256: String,
    authored_term_id: String,
    output_variable_id: String,
    approach: HigherOrderConstructionApproachV4,
    measurement_type: HigherOrderMeasurementTypeV4,
    stages: Vec<GeneralSemPlsHocPointStageResultV1>,
}

impl GeneralSemPlsHigherOrderPointResultV1 {
    pub fn method_version(&self) -> &str {
        &self.method_version
    }

    pub fn hoc_stage_plan_sha256(&self) -> &str {
        &self.hoc_stage_plan_sha256
    }

    pub fn authored_term_id(&self) -> &str {
        &self.authored_term_id
    }

    pub fn output_variable_id(&self) -> &str {
        &self.output_variable_id
    }

    pub fn approach(&self) -> &HigherOrderConstructionApproachV4 {
        &self.approach
    }

    pub fn measurement_type(&self) -> &HigherOrderMeasurementTypeV4 {
        &self.measurement_type
    }

    pub fn stages(&self) -> &[GeneralSemPlsHocPointStageResultV1] {
        &self.stages
    }

    pub fn structural_relation_estimates(
        &self,
    ) -> impl Iterator<Item = &GeneralSemPlsHocPointRelationEstimateV1> {
        self.stages.iter().flat_map(|stage| {
            stage.relation_estimates.iter().filter(|relation| {
                matches!(
                    relation.kind,
                    GeneralSemPlsHocPointRelationKindV1::AuthoredStructural
                        | GeneralSemPlsHocPointRelationKindV1::AuthoredControl
                )
            })
        })
    }

    pub fn ensure_valid_against_plan_v1(
        &self,
        plan: &CompiledPlsPlanV3,
    ) -> Result<(), GeneralSemPlsHigherOrderPointErrorV1> {
        let [hoc] = plan.higher_order_stage_plans() else {
            return Err(invalid_result("compiled plan must contain exactly one HOC"));
        };
        if self.method_version != GENERAL_SEM_PLS_HIGHER_ORDER_POINT_METHOD_VERSION_V1
            || self.hoc_stage_plan_sha256 != sha256_serialized(hoc)
            || self.authored_term_id != hoc.authored_term_id()
            || self.output_variable_id != hoc.output_variable_id()
            || &self.approach != hoc.approach()
            || &self.measurement_type != hoc.measurement_type()
            || self.stages.len() != hoc.stage_projections().len()
            || self.stages.is_empty()
        {
            return Err(invalid_result(
                "higher-order result identity differs from its compiled plan",
            ));
        }
        for (stage, projection) in self.stages.iter().zip(hoc.stage_projections()) {
            if stage.stage_id
                != format!(
                    "qpls_hoc_stage_v1_{}",
                    projection.projection_identity_sha256()
                )
                || stage.receipt.receipt_version
                    != GENERAL_SEM_PLS_HIGHER_ORDER_POINT_STAGE_RECEIPT_VERSION_V1
                || stage.receipt.stage_number != projection.stage_number()
                || stage.receipt.role != projection.role()
                || stage.receipt.projection_identity_sha256
                    != projection.projection_identity_sha256()
                || stage.receipt.dataset_fingerprint != self.stages[0].receipt.dataset_fingerprint
                || stage.receipt.used_observations != self.stages[0].receipt.used_observations
                || stage.receipt.omitted_observations != self.stages[0].receipt.omitted_observations
                || stage.input_construct_ids != hoc.component_ids()
                || stage
                    .relation_estimates
                    .iter()
                    .any(|row| !row.estimate.is_finite())
            {
                return Err(invalid_result(
                    "higher-order stage receipt differs from its compiled projection",
                ));
            }
        }
        let final_stage = self.stages.last().expect("nonempty checked above");
        let requires_score_stage = matches!(
            hoc.approach(),
            HigherOrderConstructionApproachV4::EmbeddedTwoStage
                | HigherOrderConstructionApproachV4::DisjointTwoStage
        );
        if final_stage.output_variable_ids != [hoc.output_variable_id()]
            || self.stages[..self.stages.len() - 1]
                .iter()
                .any(|stage| !stage.relation_estimates.is_empty())
            || self.stages[..self.stages.len() - 1].iter().any(|stage| {
                stage.output_variable_ids
                    != if hoc.approach() == &HigherOrderConstructionApproachV4::DisjointTwoStage {
                        hoc.component_ids()
                    } else {
                        std::slice::from_ref(&self.output_variable_id)
                    }
            })
            || (hoc.approach() == &HigherOrderConstructionApproachV4::DisjointTwoStage
                && (self.stages[0].receipt.model_scientific_sha256
                    != plan.base_plan().scientific_hash()
                    || self.stages[0].receipt.compiled_plan_sha256
                        != sha256_serialized(plan.base_plan())))
        {
            return Err(invalid_result(
                "higher-order stage contents differ from the compiled approach contract",
            ));
        }
        if self.stages[..self.stages.len() - 1]
            .iter()
            .any(|stage| stage.receipt.generated_score_dataset.is_some())
            || final_stage.receipt.generated_score_dataset.is_some() != requires_score_stage
        {
            return Err(invalid_result(
                "generated-score receipt does not match the compiled HOC approach",
            ));
        }
        if let Some(generated_score_receipt) = final_stage.receipt.generated_score_dataset.as_ref()
        {
            if generated_score_receipt.receipt_version()
                != GENERAL_SEM_PLS_DISJOINT_HOC_SCORE_DATASET_RECEIPT_VERSION_V1
                || generated_score_receipt.source_dataset_fingerprint()
                    != self.stages[0].receipt.dataset_fingerprint
                || generated_score_receipt.complete_case_row_count()
                    != self.stages[0].receipt.used_observations
                || generated_score_receipt.omitted_row_count()
                    != self.stages[0].receipt.omitted_observations
                || generated_score_receipt.generated_score_columns().len()
                    != hoc.component_mappings().len()
                || generated_score_receipt
                    .generated_score_columns()
                    .iter()
                    .zip(hoc.component_mappings())
                    .any(|(score, mapping)| {
                        score.component_id() != mapping.component_id()
                            || score.generated_score_variable_id()
                                != mapping.generated_score_variable_id()
                            || score.observation_count() != self.stages[0].receipt.used_observations
                    })
            {
                return Err(invalid_result(
                    "generated-score receipt differs from the compiled component mapping",
                ));
            }
        }
        let expected_component_ids = hoc
            .component_mappings()
            .iter()
            .map(|mapping| mapping.generated_component_relation_id())
            .collect::<BTreeSet<_>>();
        let actual_component_ids = final_stage
            .relation_estimates
            .iter()
            .filter(|relation| {
                matches!(
                    relation.kind,
                    GeneralSemPlsHocPointRelationKindV1::ComponentLoading
                        | GeneralSemPlsHocPointRelationKindV1::ComponentWeight
                )
            })
            .map(|relation| relation.relation_id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_structural_ids = plan
            .topology()
            .structural_relations()
            .iter()
            .map(|relation| relation.relation_id())
            .collect::<BTreeSet<_>>();
        let actual_structural_ids = final_stage
            .relation_estimates
            .iter()
            .filter(|relation| {
                matches!(
                    relation.kind,
                    GeneralSemPlsHocPointRelationKindV1::AuthoredStructural
                        | GeneralSemPlsHocPointRelationKindV1::AuthoredControl
                )
            })
            .map(|relation| relation.relation_id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_technical_ids = hoc
            .technical_paths()
            .iter()
            .map(|path| path.generated_relation_id())
            .collect::<BTreeSet<_>>();
        let actual_technical_ids = final_stage
            .relation_estimates
            .iter()
            .filter(|relation| {
                relation.kind == GeneralSemPlsHocPointRelationKindV1::TechnicalStructural
            })
            .map(|relation| relation.relation_id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_extended_effect_count =
            if hoc.approach() == &HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators {
                hoc.technical_paths()
                    .iter()
                    .map(|path| path.authored_antecedent_relation_id())
                    .collect::<BTreeSet<_>>()
                    .len()
                    * 2
            } else {
                0
            };
        let expected_relation_count = expected_component_ids.len()
            + expected_structural_ids.len()
            + expected_technical_ids.len()
            + expected_extended_effect_count;
        if expected_component_ids != actual_component_ids
            || expected_structural_ids != actual_structural_ids
            || expected_technical_ids != actual_technical_ids
            || final_stage.relation_estimates.len() != expected_relation_count
            || final_stage
                .relation_estimates
                .iter()
                .any(|relation| !relation.estimate.is_finite())
        {
            return Err(invalid_result(
                "higher-order relation ledger is incomplete or non-finite",
            ));
        }
        for mapping in hoc.component_mappings() {
            let expected_kind = match mapping.relation_interpretation() {
                CompiledPlsHocComponentRelationInterpretationV1::Loading => {
                    GeneralSemPlsHocPointRelationKindV1::ComponentLoading
                }
                CompiledPlsHocComponentRelationInterpretationV1::WeightAndCollinearity => {
                    GeneralSemPlsHocPointRelationKindV1::ComponentWeight
                }
            };
            let Some(actual) = final_stage
                .relation_estimates
                .iter()
                .find(|relation| relation.relation_id == mapping.generated_component_relation_id())
            else {
                return Err(invalid_result(
                    "compiled HOC component relation is absent from the point ledger",
                ));
            };
            if actual.parameter_id != mapping.generated_component_parameter_id()
                || actual.source_id != mapping.component_relation_source_id()
                || actual.target_id != mapping.component_relation_target_id()
                || actual.generated_score_variable_id.as_deref()
                    != Some(mapping.generated_score_variable_id())
                || actual.kind != expected_kind
                || match expected_kind {
                    GeneralSemPlsHocPointRelationKindV1::ComponentLoading => {
                        actual.collinearity_vif.is_some()
                    }
                    GeneralSemPlsHocPointRelationKindV1::ComponentWeight => actual
                        .collinearity_vif
                        .is_none_or(|value| !value.is_finite() || value <= 0.0),
                    GeneralSemPlsHocPointRelationKindV1::AuthoredStructural
                    | GeneralSemPlsHocPointRelationKindV1::AuthoredControl
                    | GeneralSemPlsHocPointRelationKindV1::TechnicalStructural
                    | GeneralSemPlsHocPointRelationKindV1::ExtendedIndirectEffect
                    | GeneralSemPlsHocPointRelationKindV1::ExtendedTotalEffect => true,
                }
            {
                return Err(invalid_result(
                    "HOC component estimate identity differs from the compiled mapping",
                ));
            }
        }
        for expected in plan.topology().structural_relations() {
            let Some(actual) = final_stage
                .relation_estimates
                .iter()
                .find(|relation| relation.relation_id == expected.relation_id())
            else {
                return Err(invalid_result(
                    "authored structural relation is absent from the HOC point ledger",
                ));
            };
            let expected_kind = match expected.role() {
                StructuralRelationRoleV4::Structural => {
                    GeneralSemPlsHocPointRelationKindV1::AuthoredStructural
                }
                StructuralRelationRoleV4::Control => {
                    GeneralSemPlsHocPointRelationKindV1::AuthoredControl
                }
            };
            if actual.parameter_id != expected.parameter_id()
                || actual.source_id != expected.source()
                || actual.target_id != expected.target()
                || actual.kind != expected_kind
                || actual.generated_score_variable_id.is_some()
                || actual.collinearity_vif.is_some()
            {
                return Err(invalid_result(
                    "authored relation estimate identity differs from the compiled topology",
                ));
            }
        }
        for technical in hoc.technical_paths() {
            let Some(actual) = final_stage
                .relation_estimates
                .iter()
                .find(|relation| relation.relation_id == technical.generated_relation_id())
            else {
                return Err(invalid_result("compiled HOC technical path is absent"));
            };
            if actual.parameter_id != technical.generated_parameter_id()
                || actual.source_id != technical.source_id()
                || actual.target_id != technical.component_id()
                || actual.kind != GeneralSemPlsHocPointRelationKindV1::TechnicalStructural
                || actual.generated_score_variable_id.is_some()
                || actual.collinearity_vif.is_some()
            {
                return Err(invalid_result(
                    "HOC technical path differs from the compiled mapping",
                ));
            }
        }
        if hoc.approach() == &HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators {
            for authored_id in hoc
                .technical_paths()
                .iter()
                .map(|path| path.authored_antecedent_relation_id())
                .collect::<BTreeSet<_>>()
            {
                for (kind_name, kind) in [
                    (
                        "indirect",
                        GeneralSemPlsHocPointRelationKindV1::ExtendedIndirectEffect,
                    ),
                    (
                        "total",
                        GeneralSemPlsHocPointRelationKindV1::ExtendedTotalEffect,
                    ),
                ] {
                    let id = extended_effect_identity_v1(kind_name, hoc, authored_id);
                    let Some(actual) = final_stage
                        .relation_estimates
                        .iter()
                        .find(|relation| relation.relation_id == id)
                    else {
                        return Err(invalid_result("extended repeated effect is absent"));
                    };
                    if actual.kind != kind || actual.target_id != hoc.output_variable_id() {
                        return Err(invalid_result(
                            "extended repeated effect identity is invalid",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct GeneralSemPlsHocCompiledStageV1 {
    projection: CompiledPlsHocExecutionProjectionV1,
    recipe: AnalysisRecipeV4,
    artifact: CompiledAnalysisRecipeV4,
}

#[derive(Debug, Clone)]
pub(crate) struct GeneralSemPlsHocPointExecutionContextV1 {
    repeated_stage: Option<GeneralSemPlsHocCompiledStageV1>,
    score_stage: Option<GeneralSemPlsHocCompiledStageV1>,
}

impl GeneralSemPlsHocPointExecutionContextV1 {
    pub(crate) fn final_projection(&self) -> &CompiledPlsHocExecutionProjectionV1 {
        &self
            .score_stage
            .as_ref()
            .or(self.repeated_stage.as_ref())
            .expect("a compiled HOC context always contains an execution stage")
            .projection
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GeneralSemPlsDisjointHocPointExecutionArtifactsV1 {
    result: GeneralSemPlsHigherOrderPointResultV1,
    stage_construct_scores: Vec<BTreeMap<String, Vec<f64>>>,
}

impl GeneralSemPlsDisjointHocPointExecutionArtifactsV1 {
    pub(crate) fn result(&self) -> &GeneralSemPlsHigherOrderPointResultV1 {
        &self.result
    }

    pub(crate) fn stage_construct_scores(&self) -> &[BTreeMap<String, Vec<f64>>] {
        &self.stage_construct_scores
    }

    fn into_result(self) -> GeneralSemPlsHigherOrderPointResultV1 {
        self.result
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneralSemPlsHocScoreAlignmentReferenceV1<'a> {
    original_scores: &'a BTreeMap<String, Vec<f64>>,
    sampled_positions: &'a [usize],
}

impl<'a> GeneralSemPlsHocScoreAlignmentReferenceV1<'a> {
    pub(crate) fn new(
        original_scores: &'a BTreeMap<String, Vec<f64>>,
        sampled_positions: &'a [usize],
    ) -> Self {
        Self {
            original_scores,
            sampled_positions,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneralSemPlsHocStageAlignmentReferencesV1<'a> {
    original_stage_scores: &'a [BTreeMap<String, Vec<f64>>],
    sampled_positions: &'a [usize],
}

impl<'a> GeneralSemPlsHocStageAlignmentReferencesV1<'a> {
    pub(crate) fn new(
        original_stage_scores: &'a [BTreeMap<String, Vec<f64>>],
        sampled_positions: &'a [usize],
    ) -> Self {
        Self {
            original_stage_scores,
            sampled_positions,
        }
    }

    fn for_stage(
        self,
        stage_index: usize,
    ) -> Option<GeneralSemPlsHocScoreAlignmentReferenceV1<'a>> {
        self.original_stage_scores.get(stage_index).map(|scores| {
            GeneralSemPlsHocScoreAlignmentReferenceV1::new(scores, self.sampled_positions)
        })
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum GeneralSemPlsHocScoreAlignmentErrorV1 {
    #[error("HOC score sign alignment was cancelled")]
    Cancelled,
    #[error("replicate and original HOC construct-score domains differ")]
    ConstructDomainMismatch,
    #[error("original HOC construct score is missing for {construct_id}")]
    MissingOriginalScore { construct_id: String },
    #[error(
        "sampled position {sampled_position} is outside original HOC construct score {construct_id}"
    )]
    SampledPositionOutOfBounds {
        construct_id: String,
        sampled_position: usize,
    },
    #[error(
        "replicate HOC construct score {construct_id} has {actual} observations; expected {expected}"
    )]
    ScoreLengthMismatch {
        construct_id: String,
        expected: usize,
        actual: usize,
    },
    #[error("HOC construct score {construct_id} contains a non-finite alignment value")]
    NonFiniteScore { construct_id: String },
    #[error("HOC construct score sign is indeterminate for {construct_id}")]
    IndeterminateSign { construct_id: String },
}

pub(crate) fn align_general_sem_pls_hoc_result_signs_v1(
    estimate: &mut PlsResult,
    reference: GeneralSemPlsHocScoreAlignmentReferenceV1<'_>,
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<(), GeneralSemPlsHocScoreAlignmentErrorV1> {
    if estimate.construct_scores.keys().collect::<Vec<_>>()
        != reference.original_scores.keys().collect::<Vec<_>>()
    {
        return Err(GeneralSemPlsHocScoreAlignmentErrorV1::ConstructDomainMismatch);
    }
    let mut signs = BTreeMap::new();
    for (construct_id, replicate) in &estimate.construct_scores {
        if is_cancelled() {
            return Err(GeneralSemPlsHocScoreAlignmentErrorV1::Cancelled);
        }
        let original = reference.original_scores.get(construct_id).ok_or_else(|| {
            GeneralSemPlsHocScoreAlignmentErrorV1::MissingOriginalScore {
                construct_id: construct_id.clone(),
            }
        })?;
        let aligned_reference = reference
            .sampled_positions
            .iter()
            .map(|position| {
                original.get(*position).copied().ok_or_else(|| {
                    GeneralSemPlsHocScoreAlignmentErrorV1::SampledPositionOutOfBounds {
                        construct_id: construct_id.clone(),
                        sampled_position: *position,
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if replicate.len() != aligned_reference.len() {
            return Err(GeneralSemPlsHocScoreAlignmentErrorV1::ScoreLengthMismatch {
                construct_id: construct_id.clone(),
                expected: aligned_reference.len(),
                actual: replicate.len(),
            });
        }
        if replicate
            .iter()
            .chain(&aligned_reference)
            .any(|value| !value.is_finite())
        {
            return Err(GeneralSemPlsHocScoreAlignmentErrorV1::NonFiniteScore {
                construct_id: construct_id.clone(),
            });
        }
        let (covariance, absolute_cross_product_sum) =
            hoc_score_alignment_covariance_v1(&aligned_reference, replicate);
        let tolerance =
            64.0 * f64::EPSILON * absolute_cross_product_sum.max(covariance.abs()).max(1.0);
        let sign = if covariance > tolerance {
            1.0
        } else if covariance < -tolerance {
            -1.0
        } else {
            return Err(GeneralSemPlsHocScoreAlignmentErrorV1::IndeterminateSign {
                construct_id: construct_id.clone(),
            });
        };
        signs.insert(construct_id.clone(), sign);
    }

    for (construct_id, scores) in &mut estimate.construct_scores {
        let sign = signs[construct_id];
        if sign < 0.0 {
            for score in scores {
                if is_cancelled() {
                    return Err(GeneralSemPlsHocScoreAlignmentErrorV1::Cancelled);
                }
                *score = -*score;
            }
        }
    }
    for outer in &mut estimate.outer_estimates {
        let sign = signs[&outer.construct];
        outer.weight *= sign;
        outer.loading *= sign;
    }
    for path in &mut estimate.paths {
        path.coefficient *= signs[&path.source] * signs[&path.target];
    }
    for control in &mut estimate.control_estimates {
        control.coefficient *= signs[&control.source] * signs[&control.target];
    }
    for effect in &mut estimate.effects {
        let sign = signs[&effect.source] * signs[&effect.target];
        effect.direct *= sign;
        effect.indirect *= sign;
        effect.total *= sign;
    }
    Ok(())
}

fn hoc_score_alignment_covariance_v1(left: &[f64], right: &[f64]) -> (f64, f64) {
    let left_mean = hoc_stable_sum_v1(left) / left.len() as f64;
    let right_mean = hoc_stable_sum_v1(right) / right.len() as f64;
    let cross_products = left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - left_mean) * (right - right_mean))
        .collect::<Vec<_>>();
    (
        hoc_stable_sum_v1(&cross_products),
        hoc_stable_sum_v1(
            &cross_products
                .iter()
                .map(|value| value.abs())
                .collect::<Vec<_>>(),
        ),
    )
}

fn hoc_stable_sum_v1(values: &[f64]) -> f64 {
    let mut sum = 0.0_f64;
    let mut compensation = 0.0_f64;
    for value in values {
        let updated = sum + value;
        if sum.abs() >= value.abs() {
            compensation += (sum - updated) + value;
        } else {
            compensation += (value - updated) + sum;
        }
        sum = updated;
    }
    sum + compensation
}

#[derive(Debug, thiserror::Error)]
pub enum GeneralSemPlsHigherOrderPointErrorV1 {
    #[error("higher-order point execution was cancelled")]
    Cancelled,
    #[error(transparent)]
    Projection(#[from] qpls_core::CompiledPlsHigherOrderV1Error),
    #[error(transparent)]
    ScoreDataset(#[from] GeneralSemPlsDisjointHocScoreDatasetErrorV1),
    #[error(transparent)]
    BaseRecipe(#[from] qpls_core::GeneralSemPlsRecipeCompilationErrorV1),
    #[error(transparent)]
    StageTwoCompilation(#[from] RecipeV4CompilationError),
    #[error(transparent)]
    PointEstimation(#[from] RecipeV4PlsExecutionError),
    #[error(transparent)]
    StageTwoScoreAlignment(#[from] GeneralSemPlsHocScoreAlignmentErrorV1),
    #[error("stage-one point result is not bound to the compiled lower-order plan and dataset")]
    StageOneBindingMismatch,
    #[error("stage-two recipe compilation differs from the typed HOC projection")]
    StageTwoPlanMismatch,
    #[error(
        "HOC component relation {relation_id} has no unique final outer estimate for generated score {generated_score_variable_id}"
    )]
    ComponentEstimateCardinality {
        relation_id: String,
        generated_score_variable_id: String,
    },
    #[error("formative HOC component {component_id} has undefined collinearity")]
    ComponentCollinearityUndefined { component_id: String },
    #[error("formative HOC component {component_id} collinearity failed: {reason}")]
    ComponentCollinearity {
        component_id: String,
        reason: String,
    },
    #[error(
        "authored structural relation {relation_id} ({source_id} -> {target_id}) has no unique HOC stage-two estimate"
    )]
    StructuralEstimateCardinality {
        relation_id: String,
        source_id: String,
        target_id: String,
    },
    #[error("higher-order point result contract is invalid: {0}")]
    InvalidResultContract(String),
}

fn compile_hoc_runtime_stage_v1(
    recipe: &AnalysisRecipeV4,
    projection: CompiledPlsHocExecutionProjectionV1,
) -> Result<GeneralSemPlsHocCompiledStageV1, GeneralSemPlsHigherOrderPointErrorV1> {
    let mut projected_recipe = project_general_sem_pls_base_recipe_v1(recipe)?;
    projected_recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        scientific_sha256: projection.projected_scientific_sha256().to_string(),
        model: projection.projected_model().clone(),
    };
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let artifact = compile_analysis_recipe_v4(
        &projected_recipe,
        Some(projection.projected_model()),
        target,
        target.capability_cell_for_method(recipe.settings.method),
    )?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
        return Err(GeneralSemPlsHigherOrderPointErrorV1::StageTwoPlanMismatch);
    };
    if plan != projection.projected_plan() {
        return Err(GeneralSemPlsHigherOrderPointErrorV1::StageTwoPlanMismatch);
    }
    Ok(GeneralSemPlsHocCompiledStageV1 {
        projection,
        recipe: projected_recipe,
        artifact,
    })
}

pub(crate) fn compile_general_sem_pls_hoc_point_context_v1(
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    plan: &CompiledPlsPlanV3,
) -> Result<GeneralSemPlsHocPointExecutionContextV1, GeneralSemPlsHigherOrderPointErrorV1> {
    let [hoc] = plan.higher_order_stage_plans() else {
        return Err(invalid_result("compiled plan must contain exactly one HOC"));
    };
    let repeated_stage = matches!(
        hoc.approach(),
        HigherOrderConstructionApproachV4::RepeatedIndicators
            | HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators
            | HigherOrderConstructionApproachV4::EmbeddedTwoStage
    )
    .then(|| {
        compile_pls_higher_order_repeated_stage_projection_v1(resolved_model, plan)
            .map_err(GeneralSemPlsHigherOrderPointErrorV1::Projection)
            .and_then(|projection| compile_hoc_runtime_stage_v1(recipe, projection))
    })
    .transpose()?;
    let score_stage = matches!(
        hoc.approach(),
        HigherOrderConstructionApproachV4::EmbeddedTwoStage
            | HigherOrderConstructionApproachV4::DisjointTwoStage
    )
    .then(|| {
        compile_pls_higher_order_score_stage_projection_v1(resolved_model, plan)
            .map_err(GeneralSemPlsHigherOrderPointErrorV1::Projection)
            .and_then(|projection| compile_hoc_runtime_stage_v1(recipe, projection))
    })
    .transpose()?;
    if repeated_stage.is_none() && score_stage.is_none() {
        return Err(invalid_result("compiled HOC approach has no runtime stage"));
    }
    Ok(GeneralSemPlsHocPointExecutionContextV1 {
        repeated_stage,
        score_stage,
    })
}

pub fn run_compiled_general_sem_pls_higher_order_point_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    plan: &CompiledPlsPlanV3,
    stage_one: &RecipeV4PlsExecutionResultV1,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<GeneralSemPlsHigherOrderPointResultV1, GeneralSemPlsHigherOrderPointErrorV1> {
    if should_cancel() {
        return Err(GeneralSemPlsHigherOrderPointErrorV1::Cancelled);
    }
    let context = compile_general_sem_pls_hoc_point_context_v1(recipe, resolved_model, plan)?;
    run_compiled_general_sem_pls_higher_order_point_with_context_v1(
        dataset,
        plan,
        stage_one,
        &context,
        None,
        should_cancel,
        progress,
    )
    .map(GeneralSemPlsDisjointHocPointExecutionArtifactsV1::into_result)
}

pub fn run_compiled_general_sem_pls_disjoint_higher_order_point_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    plan: &CompiledPlsPlanV3,
    stage_one: &RecipeV4PlsExecutionResultV1,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<GeneralSemPlsHigherOrderPointResultV1, GeneralSemPlsHigherOrderPointErrorV1> {
    let [hoc] = plan.higher_order_stage_plans() else {
        return Err(invalid_result("compiled plan must contain exactly one HOC"));
    };
    if hoc.approach() != &HigherOrderConstructionApproachV4::DisjointTwoStage {
        return Err(invalid_result(
            "disjoint wrapper received another HOC approach",
        ));
    }
    run_compiled_general_sem_pls_higher_order_point_v1(
        dataset,
        recipe,
        resolved_model,
        plan,
        stage_one,
        should_cancel,
        progress,
    )
}

pub(crate) fn run_compiled_general_sem_pls_higher_order_point_with_context_v1(
    dataset: &Dataset,
    plan: &CompiledPlsPlanV3,
    stage_one: &RecipeV4PlsExecutionResultV1,
    context: &GeneralSemPlsHocPointExecutionContextV1,
    stage_alignments: Option<GeneralSemPlsHocStageAlignmentReferencesV1<'_>>,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<GeneralSemPlsDisjointHocPointExecutionArtifactsV1, GeneralSemPlsHigherOrderPointErrorV1>
{
    if should_cancel() {
        return Err(GeneralSemPlsHigherOrderPointErrorV1::Cancelled);
    }
    let [hoc] = plan.higher_order_stage_plans() else {
        return Err(invalid_result("compiled plan must contain exactly one HOC"));
    };
    let stage_one_receipt = stage_one.provenance().compilation_receipt();
    let stage_one_plan = CompiledRecipePlanV4::PlsPlanV2 {
        plan: plan.base_plan().clone(),
    };
    if stage_one_receipt.compiler_target() != RecipeV4CompilerTarget::PlsPlanV2
        || stage_one_receipt.model_scientific_sha256() != plan.base_plan().scientific_hash()
        || stage_one_receipt.plan_sha256() != sha256_serialized(&stage_one_plan)
        || stage_one_receipt.dataset_fingerprint() != dataset.fingerprint.0.as_str()
    {
        return Err(GeneralSemPlsHigherOrderPointErrorV1::StageOneBindingMismatch);
    }
    for compiled_stage in context
        .repeated_stage
        .iter()
        .chain(context.score_stage.iter())
    {
        if compiled_stage.projection.source_scientific_sha256() != plan.scientific_hash()
            || compiled_stage.projection.hoc_stage_plan_sha256() != sha256_serialized(hoc)
            || compiled_stage.projection.projected_plan()
                != match compiled_stage.artifact.plan() {
                    CompiledRecipePlanV4::PlsPlanV2 { plan } => plan,
                    _ => {
                        return Err(GeneralSemPlsHigherOrderPointErrorV1::StageTwoPlanMismatch);
                    }
                }
        {
            return Err(GeneralSemPlsHigherOrderPointErrorV1::StageTwoPlanMismatch);
        }
    }

    let mut repeated_estimation = None;
    if let Some(compiled_stage) = &context.repeated_stage {
        let aliases = hoc
            .component_mappings()
            .iter()
            .flat_map(|mapping| {
                mapping
                    .virtual_indicators()
                    .iter()
                    .map(move |indicator| PlsAliasColumnSpecV1 {
                        source_column_id: indicator.source_column().to_string(),
                        generated_column_id: indicator.generated_source_column_id().to_string(),
                        label: format!(
                            "Repeated HOC indicator: {} <- {}",
                            hoc.output_variable_id(),
                            mapping.component_id()
                        ),
                    })
            })
            .collect::<Vec<_>>();
        let repeated_dataset = append_pls_alias_columns_v1(dataset, &aliases)?;
        let mut estimation = run_hoc_compiled_stage_v1(
            &repeated_dataset,
            compiled_stage,
            "general_sem_hoc_repeated_stage",
            &should_cancel,
            &progress,
        )?;
        if let Some(reference) = stage_alignments.and_then(|value| value.for_stage(0)) {
            align_general_sem_pls_hoc_result_signs_v1(
                estimation.estimation_mut(),
                reference,
                &should_cancel,
            )
            .map_err(map_hoc_alignment_error_v1)?;
        }
        estimation.retain_source_row_accounting(
            stage_one.estimation().used_observations,
            stage_one.estimation().omitted_observations,
        );
        repeated_estimation = Some(estimation);
    }

    let component_estimation = repeated_estimation.as_ref().unwrap_or(stage_one);
    let mut generated_score_dataset = None;
    let mut score_estimation = None;
    if let Some(compiled_stage) = &context.score_stage {
        progress(RunnerProgress {
            phase: "general_sem_hoc_prepare_score_stage".into(),
            completed_units: 0,
            total_units: 1,
        });
        let prepared = prepare_general_sem_pls_disjoint_hoc_score_dataset_v1(
            dataset,
            plan,
            component_estimation.estimation(),
            || !should_cancel(),
        )
        .map_err(map_hoc_score_dataset_error_v1)?;
        progress(RunnerProgress {
            phase: "general_sem_hoc_prepare_score_stage".into(),
            completed_units: 1,
            total_units: 1,
        });
        let mut estimation = run_hoc_compiled_stage_v1(
            prepared.dataset(),
            compiled_stage,
            "general_sem_hoc_score_stage",
            &should_cancel,
            &progress,
        )?;
        let final_stage_index = hoc.stage_projections().len() - 1;
        if let Some(reference) =
            stage_alignments.and_then(|value| value.for_stage(final_stage_index))
        {
            align_general_sem_pls_hoc_result_signs_v1(
                estimation.estimation_mut(),
                reference,
                &should_cancel,
            )
            .map_err(map_hoc_alignment_error_v1)?;
        }
        estimation.retain_source_row_accounting(
            stage_one.estimation().used_observations,
            stage_one.estimation().omitted_observations,
        );
        let (_, receipt) = prepared.into_parts();
        generated_score_dataset = Some(receipt);
        score_estimation = Some(estimation);
    }
    let final_estimation = score_estimation
        .as_ref()
        .or(repeated_estimation.as_ref())
        .ok_or_else(|| invalid_result("HOC execution produced no final stage"))?;

    let mut relation_estimates = Vec::new();
    let repeated_component_coefficients = context
        .repeated_stage
        .as_ref()
        .map(|_| repeated_hoc_component_coefficients_v1(final_estimation.estimation(), hoc))
        .transpose()?;
    for mapping in hoc.component_mappings() {
        let score_stage_outer = score_estimation.as_ref().and_then(|estimation| {
            let matches = estimation
                .estimation()
                .outer_estimates
                .iter()
                .filter(|estimate| {
                    estimate.construct == hoc.output_variable_id()
                        && estimate.indicator == mapping.generated_score_variable_id()
                })
                .collect::<Vec<_>>();
            matches
                .as_slice()
                .first()
                .copied()
                .filter(|_| matches.len() == 1)
        });
        let (kind, value, collinearity_vif) = match mapping.relation_interpretation() {
            CompiledPlsHocComponentRelationInterpretationV1::Loading => (
                GeneralSemPlsHocPointRelationKindV1::ComponentLoading,
                score_stage_outer
                    .map(|estimate| estimate.loading)
                    .or_else(|| {
                        repeated_component_coefficients
                            .as_ref()
                            .and_then(|values| values.get(mapping.component_id()).copied())
                    })
                    .ok_or_else(|| {
                        GeneralSemPlsHigherOrderPointErrorV1::ComponentEstimateCardinality {
                            relation_id: mapping.generated_component_relation_id().to_string(),
                            generated_score_variable_id: mapping
                                .generated_score_variable_id()
                                .to_string(),
                        }
                    })?,
                None,
            ),
            CompiledPlsHocComponentRelationInterpretationV1::WeightAndCollinearity => {
                let target =
                    &component_estimation.estimation().construct_scores[mapping.component_id()];
                let remaining = hoc
                    .component_mappings()
                    .iter()
                    .filter(|candidate| candidate.component_id() != mapping.component_id())
                    .map(|candidate| {
                        component_estimation.estimation().construct_scores[candidate.component_id()]
                            .as_slice()
                    })
                    .collect::<Vec<_>>();
                let vif = variance_inflation_factor_with_control(target, &remaining, |_, _| {
                    !should_cancel()
                })
                .map_err(|error| match error {
                    AssessmentError::Cancelled => GeneralSemPlsHigherOrderPointErrorV1::Cancelled,
                    other => GeneralSemPlsHigherOrderPointErrorV1::ComponentCollinearity {
                        component_id: mapping.component_id().to_string(),
                        reason: other.to_string(),
                    },
                })?
                .ok_or_else(|| {
                    GeneralSemPlsHigherOrderPointErrorV1::ComponentCollinearityUndefined {
                        component_id: mapping.component_id().to_string(),
                    }
                })?;
                (
                    GeneralSemPlsHocPointRelationKindV1::ComponentWeight,
                    score_stage_outer
                        .map(|estimate| estimate.weight)
                        .or_else(|| {
                            repeated_component_coefficients
                                .as_ref()
                                .and_then(|values| values.get(mapping.component_id()).copied())
                        })
                        .ok_or_else(|| {
                            GeneralSemPlsHigherOrderPointErrorV1::ComponentEstimateCardinality {
                                relation_id: mapping.generated_component_relation_id().to_string(),
                                generated_score_variable_id: mapping
                                    .generated_score_variable_id()
                                    .to_string(),
                            }
                        })?,
                    Some(vif),
                )
            }
        };
        relation_estimates.push(GeneralSemPlsHocPointRelationEstimateV1 {
            relation_id: mapping.generated_component_relation_id().to_string(),
            parameter_id: mapping.generated_component_parameter_id().to_string(),
            source_id: mapping.component_relation_source_id().to_string(),
            target_id: mapping.component_relation_target_id().to_string(),
            kind,
            estimate: value,
            generated_score_variable_id: Some(mapping.generated_score_variable_id().to_string()),
            collinearity_vif,
        });
    }
    for technical in hoc.technical_paths() {
        let coefficients = final_estimation
            .estimation()
            .paths
            .iter()
            .filter(|estimate| {
                estimate.source == technical.source_id()
                    && estimate.target == technical.component_id()
            })
            .map(|estimate| estimate.coefficient)
            .collect::<Vec<_>>();
        let [estimate] = coefficients.as_slice() else {
            return Err(
                GeneralSemPlsHigherOrderPointErrorV1::StructuralEstimateCardinality {
                    relation_id: technical.generated_relation_id().to_string(),
                    source_id: technical.source_id().to_string(),
                    target_id: technical.component_id().to_string(),
                },
            );
        };
        relation_estimates.push(GeneralSemPlsHocPointRelationEstimateV1 {
            relation_id: technical.generated_relation_id().to_string(),
            parameter_id: technical.generated_parameter_id().to_string(),
            source_id: technical.source_id().to_string(),
            target_id: technical.component_id().to_string(),
            kind: GeneralSemPlsHocPointRelationKindV1::TechnicalStructural,
            estimate: *estimate,
            generated_score_variable_id: None,
            collinearity_vif: None,
        });
    }
    for relation in plan.topology().structural_relations() {
        let relation_id = relation.relation_id();
        let coefficients = match relation.role() {
            StructuralRelationRoleV4::Structural => final_estimation
                .estimation()
                .paths
                .iter()
                .filter(|estimate| {
                    estimate.source == relation.source() && estimate.target == relation.target()
                })
                .map(|estimate| estimate.coefficient)
                .collect::<Vec<_>>(),
            StructuralRelationRoleV4::Control => final_estimation
                .estimation()
                .control_estimates
                .iter()
                .filter(|estimate| {
                    estimate.source == relation.source() && estimate.target == relation.target()
                })
                .map(|estimate| estimate.coefficient)
                .collect::<Vec<_>>(),
        };
        let [estimate] = coefficients.as_slice() else {
            return Err(
                GeneralSemPlsHigherOrderPointErrorV1::StructuralEstimateCardinality {
                    relation_id: relation_id.to_string(),
                    source_id: relation.source().to_string(),
                    target_id: relation.target().to_string(),
                },
            );
        };
        relation_estimates.push(GeneralSemPlsHocPointRelationEstimateV1 {
            relation_id: relation_id.to_string(),
            parameter_id: relation.parameter_id().to_string(),
            source_id: relation.source().to_string(),
            target_id: relation.target().to_string(),
            kind: match relation.role() {
                StructuralRelationRoleV4::Structural => {
                    GeneralSemPlsHocPointRelationKindV1::AuthoredStructural
                }
                StructuralRelationRoleV4::Control => {
                    GeneralSemPlsHocPointRelationKindV1::AuthoredControl
                }
            },
            estimate: *estimate,
            generated_score_variable_id: None,
            collinearity_vif: None,
        });
    }
    append_extended_repeated_effects_v1(hoc, &mut relation_estimates)?;
    relation_estimates.sort_by(|left, right| left.relation_id.cmp(&right.relation_id));

    let final_projection = context.final_projection();
    let final_stage_index = hoc.stage_projections().len() - 1;
    let mut stages = Vec::with_capacity(hoc.stage_projections().len());
    for (stage_index, projection) in hoc.stage_projections().iter().enumerate() {
        let is_final = stage_index == final_stage_index;
        let (model_sha256, plan_sha256, used, omitted, score_receipt) = if is_final {
            (
                final_projection.projected_scientific_sha256().to_string(),
                sha256_serialized(final_projection.projected_plan()),
                final_estimation.estimation().used_observations,
                final_estimation.estimation().omitted_observations,
                generated_score_dataset.clone(),
            )
        } else if context.repeated_stage.is_some() {
            let repeated = context.repeated_stage.as_ref().unwrap();
            let estimation = repeated_estimation.as_ref().unwrap();
            (
                repeated
                    .projection
                    .projected_scientific_sha256()
                    .to_string(),
                sha256_serialized(repeated.projection.projected_plan()),
                estimation.estimation().used_observations,
                estimation.estimation().omitted_observations,
                None,
            )
        } else {
            (
                plan.base_plan().scientific_hash().to_string(),
                sha256_serialized(plan.base_plan()),
                stage_one.estimation().used_observations,
                stage_one.estimation().omitted_observations,
                None,
            )
        };
        stages.push(GeneralSemPlsHocPointStageResultV1 {
            stage_id: format!(
                "qpls_hoc_stage_v1_{}",
                projection.projection_identity_sha256()
            ),
            input_construct_ids: hoc.component_ids().to_vec(),
            output_variable_ids: if is_final || context.repeated_stage.is_some() {
                vec![hoc.output_variable_id().to_string()]
            } else {
                hoc.component_ids().to_vec()
            },
            relation_estimates: if is_final {
                relation_estimates.clone()
            } else {
                Vec::new()
            },
            receipt: GeneralSemPlsHocPointStageReceiptV1 {
                receipt_version: GENERAL_SEM_PLS_HIGHER_ORDER_POINT_STAGE_RECEIPT_VERSION_V1.into(),
                stage_number: projection.stage_number(),
                role: projection.role(),
                projection_identity_sha256: projection.projection_identity_sha256().to_string(),
                model_scientific_sha256: model_sha256,
                compiled_plan_sha256: plan_sha256,
                dataset_fingerprint: dataset.fingerprint.0.clone(),
                used_observations: used,
                omitted_observations: omitted,
                generated_score_dataset: score_receipt,
            },
        });
    }
    let result = GeneralSemPlsHigherOrderPointResultV1 {
        method_version: GENERAL_SEM_PLS_HIGHER_ORDER_POINT_METHOD_VERSION_V1.into(),
        hoc_stage_plan_sha256: sha256_serialized(hoc),
        authored_term_id: hoc.authored_term_id().to_string(),
        output_variable_id: hoc.output_variable_id().to_string(),
        approach: hoc.approach().clone(),
        measurement_type: hoc.measurement_type().clone(),
        stages,
    };
    result.ensure_valid_against_plan_v1(plan)?;
    let stage_construct_scores = match hoc.approach() {
        HigherOrderConstructionApproachV4::RepeatedIndicators
        | HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators => {
            vec![final_estimation.estimation().construct_scores.clone()]
        }
        HigherOrderConstructionApproachV4::EmbeddedTwoStage => vec![
            repeated_estimation
                .as_ref()
                .unwrap()
                .estimation()
                .construct_scores
                .clone(),
            final_estimation.estimation().construct_scores.clone(),
        ],
        HigherOrderConstructionApproachV4::DisjointTwoStage => vec![
            stage_one.estimation().construct_scores.clone(),
            final_estimation.estimation().construct_scores.clone(),
        ],
        HigherOrderConstructionApproachV4::Hybrid => unreachable!("hybrid is compiler-blocked"),
    };
    Ok(GeneralSemPlsDisjointHocPointExecutionArtifactsV1 {
        result,
        stage_construct_scores,
    })
}

fn run_hoc_compiled_stage_v1(
    dataset: &Dataset,
    stage: &GeneralSemPlsHocCompiledStageV1,
    phase_prefix: &str,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(RunnerProgress) + Sync),
) -> Result<RecipeV4PlsExecutionResultV1, GeneralSemPlsHigherOrderPointErrorV1> {
    run_compiled_pls_recipe_v4_allowing_isolated(
        dataset,
        &stage.recipe,
        stage.projection.projected_model(),
        &stage.artifact,
        should_cancel,
        |update| {
            progress(RunnerProgress {
                phase: format!("{phase_prefix}_{}", update.phase),
                completed_units: update.completed_units,
                total_units: update.total_units,
            });
        },
    )
    .map_err(|error| match error {
        RecipeV4PlsExecutionError::Cancelled => GeneralSemPlsHigherOrderPointErrorV1::Cancelled,
        other => GeneralSemPlsHigherOrderPointErrorV1::PointEstimation(other),
    })
}

fn map_hoc_score_dataset_error_v1(
    error: GeneralSemPlsDisjointHocScoreDatasetErrorV1,
) -> GeneralSemPlsHigherOrderPointErrorV1 {
    match error {
        GeneralSemPlsDisjointHocScoreDatasetErrorV1::Cancelled => {
            GeneralSemPlsHigherOrderPointErrorV1::Cancelled
        }
        other => GeneralSemPlsHigherOrderPointErrorV1::ScoreDataset(other),
    }
}

fn map_hoc_alignment_error_v1(
    error: GeneralSemPlsHocScoreAlignmentErrorV1,
) -> GeneralSemPlsHigherOrderPointErrorV1 {
    match error {
        GeneralSemPlsHocScoreAlignmentErrorV1::Cancelled => {
            GeneralSemPlsHigherOrderPointErrorV1::Cancelled
        }
        other => GeneralSemPlsHigherOrderPointErrorV1::StageTwoScoreAlignment(other),
    }
}

fn repeated_hoc_component_coefficients_v1(
    estimation: &PlsResult,
    hoc: &qpls_core::CompiledPlsHigherOrderStagePlanV1,
) -> Result<BTreeMap<String, f64>, GeneralSemPlsHigherOrderPointErrorV1> {
    let output = estimation
        .construct_scores
        .get(hoc.output_variable_id())
        .ok_or_else(|| invalid_result("repeated HOC output score is absent"))?;
    let component_scores = hoc
        .component_mappings()
        .iter()
        .map(|mapping| {
            estimation
                .construct_scores
                .get(mapping.component_id())
                .map(|values| (mapping.component_id(), values.as_slice()))
                .ok_or_else(|| {
                    invalid_result(format!(
                        "repeated HOC component score is absent for {}",
                        mapping.component_id()
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    match hoc.component_relation_interpretation() {
        CompiledPlsHocComponentRelationInterpretationV1::Loading => component_scores
            .into_iter()
            .map(|(component_id, values)| {
                Ok((
                    component_id.to_string(),
                    hoc_score_correlation_v1(values, output)?,
                ))
            })
            .collect(),
        CompiledPlsHocComponentRelationInterpretationV1::WeightAndCollinearity => {
            let standardized_predictors = component_scores
                .iter()
                .map(|(component_id, values)| {
                    standardize_hoc_score_v1(values).ok_or_else(|| {
                        invalid_result(format!(
                            "formative repeated HOC component {component_id} is constant or non-finite"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let standardized_output = standardize_hoc_score_v1(output)
                .ok_or_else(|| invalid_result("formative repeated HOC output is constant"))?;
            let coefficients =
                solve_hoc_least_squares_qr_v1(&standardized_predictors, &standardized_output)?;
            Ok(component_scores
                .iter()
                .zip(coefficients)
                .map(|((component_id, _), coefficient)| ((*component_id).to_string(), coefficient))
                .collect())
        }
    }
}

fn hoc_score_correlation_v1(
    left: &[f64],
    right: &[f64],
) -> Result<f64, GeneralSemPlsHigherOrderPointErrorV1> {
    let left = standardize_hoc_score_v1(left)
        .ok_or_else(|| invalid_result("HOC component score is constant or non-finite"))?;
    let right = standardize_hoc_score_v1(right)
        .ok_or_else(|| invalid_result("HOC output score is constant or non-finite"))?;
    if left.len() != right.len() || left.len() < 3 {
        return Err(invalid_result("HOC score domains differ"));
    }
    Ok(left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f64>()
        / (left.len() - 1) as f64)
}

fn standardize_hoc_score_v1(values: &[f64]) -> Option<Vec<f64>> {
    if values.len() < 3 || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mean = hoc_stable_sum_v1(values) / values.len() as f64;
    let centered = values.iter().map(|value| value - mean).collect::<Vec<_>>();
    let variance = hoc_stable_sum_v1(
        &centered
            .iter()
            .map(|value| value * value)
            .collect::<Vec<_>>(),
    ) / (values.len() - 1) as f64;
    let standard_deviation = variance.sqrt();
    (standard_deviation.is_finite() && standard_deviation > f64::EPSILON).then(|| {
        centered
            .into_iter()
            .map(|value| value / standard_deviation)
            .collect()
    })
}

fn solve_hoc_least_squares_qr_v1(
    predictors: &[Vec<f64>],
    outcome: &[f64],
) -> Result<Vec<f64>, GeneralSemPlsHigherOrderPointErrorV1> {
    let mut q_columns = Vec::<Vec<f64>>::with_capacity(predictors.len());
    let mut r = vec![vec![0.0; predictors.len()]; predictors.len()];
    let rank_tolerance = (outcome.len() as f64).sqrt() * 1.0e-11;
    for (column_index, predictor) in predictors.iter().enumerate() {
        if predictor.len() != outcome.len() {
            return Err(invalid_result("formative HOC score domains differ"));
        }
        let mut residual = predictor.clone();
        for previous in 0..column_index {
            let projection = hoc_dot_v1(&q_columns[previous], &residual);
            r[previous][column_index] += projection;
            hoc_subtract_scaled_v1(&mut residual, &q_columns[previous], projection);
        }
        for previous in 0..column_index {
            let correction = hoc_dot_v1(&q_columns[previous], &residual);
            r[previous][column_index] += correction;
            hoc_subtract_scaled_v1(&mut residual, &q_columns[previous], correction);
        }
        let norm = hoc_dot_v1(&residual, &residual).sqrt();
        if !norm.is_finite() || norm <= rank_tolerance {
            return Err(invalid_result(
                "formative HOC component scores are rank deficient",
            ));
        }
        r[column_index][column_index] = norm;
        residual.iter_mut().for_each(|value| *value /= norm);
        q_columns.push(residual);
    }
    let mut coefficients = q_columns
        .iter()
        .map(|column| hoc_dot_v1(column, outcome))
        .collect::<Vec<_>>();
    for row in (0..coefficients.len()).rev() {
        for column in row + 1..coefficients.len() {
            coefficients[row] -= r[row][column] * coefficients[column];
        }
        coefficients[row] /= r[row][row];
    }
    Ok(coefficients)
}

fn hoc_dot_v1(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn hoc_subtract_scaled_v1(target: &mut [f64], source: &[f64], scale: f64) {
    for (target, source) in target.iter_mut().zip(source) {
        *target -= scale * source;
    }
}

pub(crate) fn extended_effect_identity_v1(
    kind: &str,
    hoc: &qpls_core::CompiledPlsHigherOrderStagePlanV1,
    authored_relation_id: &str,
) -> String {
    format!(
        "qpls_hoc_extended_{kind}_v1_{}",
        sha256_serialized(&(
            "general_sem_pls_extended_repeated_effect_v1",
            hoc.authored_term_id(),
            hoc.output_variable_id(),
            authored_relation_id,
            kind,
        ))
    )
}

fn append_extended_repeated_effects_v1(
    hoc: &qpls_core::CompiledPlsHigherOrderStagePlanV1,
    estimates: &mut Vec<GeneralSemPlsHocPointRelationEstimateV1>,
) -> Result<(), GeneralSemPlsHigherOrderPointErrorV1> {
    if hoc.approach() != &HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators {
        return Ok(());
    }
    let component_relations = hoc
        .component_mappings()
        .iter()
        .map(|mapping| {
            (
                mapping.component_id(),
                mapping.generated_component_relation_id(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let authored_ids = hoc
        .technical_paths()
        .iter()
        .map(|path| path.authored_antecedent_relation_id())
        .collect::<BTreeSet<_>>();
    for authored_id in authored_ids {
        let direct = estimates
            .iter()
            .find(|estimate| estimate.relation_id() == authored_id)
            .cloned()
            .ok_or_else(|| invalid_result("extended repeated direct effect is absent"))?;
        let mut indirect = 0.0;
        for path in hoc
            .technical_paths()
            .iter()
            .filter(|path| path.authored_antecedent_relation_id() == authored_id)
        {
            let technical = estimates
                .iter()
                .find(|estimate| estimate.relation_id() == path.generated_relation_id())
                .ok_or_else(|| invalid_result("extended repeated technical path is absent"))?;
            let component_relation_id = component_relations[path.component_id()];
            let component = estimates
                .iter()
                .find(|estimate| estimate.relation_id() == component_relation_id)
                .ok_or_else(|| invalid_result("extended repeated component weight is absent"))?;
            indirect += technical.estimate() * component.estimate();
        }
        let indirect_id = extended_effect_identity_v1("indirect", hoc, authored_id);
        let total_id = extended_effect_identity_v1("total", hoc, authored_id);
        estimates.push(GeneralSemPlsHocPointRelationEstimateV1 {
            relation_id: indirect_id.clone(),
            parameter_id: format!("{indirect_id}:estimand"),
            source_id: direct.source_id().to_string(),
            target_id: direct.target_id().to_string(),
            kind: GeneralSemPlsHocPointRelationKindV1::ExtendedIndirectEffect,
            estimate: indirect,
            generated_score_variable_id: None,
            collinearity_vif: None,
        });
        estimates.push(GeneralSemPlsHocPointRelationEstimateV1 {
            relation_id: total_id.clone(),
            parameter_id: format!("{total_id}:estimand"),
            source_id: direct.source_id().to_string(),
            target_id: direct.target_id().to_string(),
            kind: GeneralSemPlsHocPointRelationKindV1::ExtendedTotalEffect,
            estimate: direct.estimate() + indirect,
            generated_score_variable_id: None,
            collinearity_vif: None,
        });
    }
    Ok(())
}

fn invalid_result(message: impl Into<String>) -> GeneralSemPlsHigherOrderPointErrorV1 {
    GeneralSemPlsHigherOrderPointErrorV1::InvalidResultContract(message.into())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe, AnalysisSettings,
        Construct, GeneralSemConfigV1, HigherOrderConstruct, HigherOrderMethod,
        LegacyBasicModelInterpretationV4, MeasurementMode, MethodConfig, ModelSpec,
        SemDataBindingV4, SemDerivedTermV4, SemParameterTargetV4, SemParameterV4, SemRelationV4,
        SemVariableV4, StructuralPath, StructuralRelationRoleV4, compile_pls_plan_v3,
        confirm_legacy_recipe_estimand_v4, migrate_analysis_recipe_to_v4_pending,
        project_general_sem_pls_stage_one_recipe_v1,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use qpls_estimation::estimate_pls;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn source_dataset() -> Dataset {
        let mut csv = String::from("a1,a2,x1,x2,z1,z2,y1,y2\n");
        for row_index in 0..96 {
            let row = f64::from(row_index);
            let a = (row * 0.09).sin() + 0.12 * ((row - 47.5) / 15.0);
            let x = ((row - 47.5) / 15.0) + 0.25 * (row * 0.31).sin() + 0.18 * a;
            let z = (row * 0.17).cos() + 0.15 * x + 0.21 * a + 0.12 * (row * 0.07).sin();
            let y = 0.48 * x + 0.62 * z + 0.08 * (row * 0.23).cos();
            let x2 = (row_index != 17)
                .then(|| format!("{}", 0.92 * x + 0.08 * (row * 0.19).cos()))
                .unwrap_or_default();
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                a + 0.04 * (row * 0.27).cos(),
                1.02 * a + 0.05 * (row * 0.21).sin(),
                x + 0.05 * (row * 0.11).sin(),
                x2,
                z + 0.06 * (row * 0.13).cos(),
                1.04 * z + 0.05 * (row * 0.29).sin(),
                y + 0.04 * (row * 0.37).sin(),
                1.03 * y + 0.05 * (row * 0.41).cos(),
            ));
        }
        import_delimited_bytes(
            csv.as_bytes(),
            "general-sem-disjoint-hoc.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap()
    }

    fn source_model(loc_mode: MeasurementMode) -> ModelSpec {
        ModelSpec {
            id: Uuid::from_u128(0x484f_435f_504f_494e_545f_5631),
            name: "Disjoint HOC point fixture".into(),
            constructs: vec![
                Construct {
                    id: "a".into(),
                    name: "A".into(),
                    short_name: "A".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["a1".into(), "a2".into()],
                },
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: loc_mode.clone(),
                    indicators: vec!["x1".into(), "x2".into()],
                },
                Construct {
                    id: "z".into(),
                    name: "Z".into(),
                    short_name: "Z".into(),
                    mode: loc_mode,
                    indicators: vec!["z1".into(), "z2".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["y1".into(), "y2".into()],
                },
            ],
            paths: Vec::new(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        }
    }

    pub(crate) fn fixture_for(
        measurement_type: HigherOrderMeasurementTypeV4,
        approach: HigherOrderConstructionApproachV4,
        endogenous: bool,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        RecipeV4PlsExecutionResultV1,
        GeneralSemPlsHigherOrderPointResultV1,
    ) {
        let loc_mode = match &measurement_type {
            HigherOrderMeasurementTypeV4::ReflectiveReflective
            | HigherOrderMeasurementTypeV4::ReflectiveFormative => MeasurementMode::Reflective,
            HigherOrderMeasurementTypeV4::FormativeReflective
            | HigherOrderMeasurementTypeV4::FormativeFormative => MeasurementMode::Formative,
        };
        let dataset = source_dataset();
        let source_model = source_model(loc_mode);
        let source_recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x484f_435f_5245_4349_5045_5631),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: source_model.clone(),
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                workers: 1,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let pending = migrate_analysis_recipe_to_v4_pending(&source_recipe).unwrap();
        let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source_model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *dataset_id = dataset.id.to_string();
        let output = "derived:hoc".to_string();
        model.variables.push(SemVariableV4::Derived {
            id: output.clone(),
            label: "Higher order".into(),
        });
        let mut structural_relations = vec![(
            "relation:hoc_y",
            "parameter:hoc_y",
            output.as_str(),
            "construct:y",
            "HOC -> Y",
        )];
        if endogenous {
            structural_relations.insert(
                0,
                (
                    "relation:a_hoc",
                    "parameter:a_hoc",
                    "construct:a",
                    output.as_str(),
                    "A -> HOC",
                ),
            );
        }
        for (relation_id, parameter_id, source, target, label) in structural_relations {
            model.relations.push(SemRelationV4::Structural {
                id: relation_id.into(),
                source: source.into(),
                target: target.into(),
                parameter: parameter_id.into(),
                role: StructuralRelationRoleV4::Structural,
                intercept_parameter: None,
            });
            model.parameters.push(SemParameterV4::Free {
                id: parameter_id.into(),
                label: label.into(),
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
        model.derived_terms.push(SemDerivedTermV4::HigherOrder {
            id: "term:hoc".into(),
            output,
            components: vec!["construct:x".into(), "construct:z".into()],
            approach,
            measurement_type,
        });
        model.ensure_valid().unwrap();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256: model.scientific_sha256().unwrap(),
            model: model.clone(),
        };
        recipe.general_sem_config = Some(GeneralSemConfigV1::default());
        recipe.ensure_valid().unwrap();
        let plan =
            compile_pls_plan_v3(&model, recipe.general_sem_config.as_ref().unwrap()).unwrap();
        let (stage_one_recipe, stage_one_model) =
            project_general_sem_pls_stage_one_recipe_v1(&recipe, &model).unwrap();
        let target = RecipeV4CompilerTarget::PlsPlanV2;
        let stage_one_artifact = compile_analysis_recipe_v4(
            &stage_one_recipe,
            Some(&stage_one_model),
            target,
            target.capability_cell_for_method(recipe.settings.method),
        )
        .unwrap();
        let stage_one = run_compiled_pls_recipe_v4_allowing_isolated(
            &dataset,
            &stage_one_recipe,
            &stage_one_model,
            &stage_one_artifact,
            || false,
            |_| {},
        )
        .unwrap();
        let point = run_compiled_general_sem_pls_higher_order_point_v1(
            &dataset,
            &recipe,
            &model,
            &plan,
            &stage_one,
            || false,
            |_| {},
        )
        .unwrap();
        (dataset, recipe, model, stage_one, point)
    }

    pub(crate) fn fixture(
        measurement_type: HigherOrderMeasurementTypeV4,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        RecipeV4PlsExecutionResultV1,
        GeneralSemPlsHigherOrderPointResultV1,
    ) {
        fixture_for(
            measurement_type,
            HigherOrderConstructionApproachV4::DisjointTwoStage,
            true,
        )
    }

    #[test]
    fn all_four_disjoint_hcm_types_produce_typed_two_stage_receipts() {
        for measurement_type in [
            HigherOrderMeasurementTypeV4::ReflectiveReflective,
            HigherOrderMeasurementTypeV4::ReflectiveFormative,
            HigherOrderMeasurementTypeV4::FormativeReflective,
            HigherOrderMeasurementTypeV4::FormativeFormative,
        ] {
            let (_, _, _, _, point) = fixture(measurement_type.clone());
            assert_eq!(point.measurement_type(), &measurement_type);
            assert_eq!(point.stages().len(), 2);
            assert!(
                point.stages()[0]
                    .receipt()
                    .generated_score_dataset()
                    .is_none()
            );
            let score_receipt = point.stages()[1]
                .receipt()
                .generated_score_dataset()
                .unwrap();
            assert_eq!(score_receipt.generated_score_columns().len(), 2);
            assert_eq!(score_receipt.complete_case_row_count(), 95);
            assert_eq!(score_receipt.omitted_row_count(), 1);
            assert_eq!(point.structural_relation_estimates().count(), 2);
            let expected_kind = match measurement_type {
                HigherOrderMeasurementTypeV4::ReflectiveReflective
                | HigherOrderMeasurementTypeV4::FormativeReflective => {
                    GeneralSemPlsHocPointRelationKindV1::ComponentLoading
                }
                HigherOrderMeasurementTypeV4::ReflectiveFormative
                | HigherOrderMeasurementTypeV4::FormativeFormative => {
                    GeneralSemPlsHocPointRelationKindV1::ComponentWeight
                }
            };
            assert_eq!(
                point.stages()[1]
                    .relation_estimates()
                    .iter()
                    .filter(|estimate| estimate.kind() == expected_kind)
                    .count(),
                2
            );
            for estimate in point.stages()[1]
                .relation_estimates()
                .iter()
                .filter(|estimate| {
                    matches!(
                        estimate.kind(),
                        GeneralSemPlsHocPointRelationKindV1::ComponentLoading
                            | GeneralSemPlsHocPointRelationKindV1::ComponentWeight
                    )
                })
            {
                match expected_kind {
                    GeneralSemPlsHocPointRelationKindV1::ComponentLoading => {
                        assert_eq!(estimate.collinearity_vif(), None);
                    }
                    GeneralSemPlsHocPointRelationKindV1::ComponentWeight => {
                        assert!(estimate.collinearity_vif().is_some_and(f64::is_finite));
                    }
                    _ => unreachable!(),
                }
            }
            assert!(
                point.stages()[1]
                    .relation_estimates()
                    .iter()
                    .all(|estimate| estimate.estimate().is_finite())
            );
        }
    }

    #[test]
    fn repeated_extended_embedded_and_disjoint_runtime_paths_publish_exact_stage_shapes() {
        for (approach, measurement_type, endogenous, expected_stages) in [
            (
                HigherOrderConstructionApproachV4::RepeatedIndicators,
                HigherOrderMeasurementTypeV4::ReflectiveReflective,
                true,
                1,
            ),
            (
                HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators,
                HigherOrderMeasurementTypeV4::ReflectiveFormative,
                true,
                1,
            ),
            (
                HigherOrderConstructionApproachV4::EmbeddedTwoStage,
                HigherOrderMeasurementTypeV4::FormativeFormative,
                true,
                2,
            ),
            (
                HigherOrderConstructionApproachV4::DisjointTwoStage,
                HigherOrderMeasurementTypeV4::FormativeReflective,
                true,
                2,
            ),
        ] {
            let (_, _, _, _, point) = fixture_for(measurement_type, approach.clone(), endogenous);
            assert_eq!(point.approach(), &approach);
            assert_eq!(point.stages().len(), expected_stages);
            assert!(
                point
                    .stages()
                    .last()
                    .unwrap()
                    .relation_estimates()
                    .iter()
                    .all(|row| row.estimate().is_finite())
            );
            if approach == HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators {
                assert_eq!(
                    point
                        .stages()
                        .last()
                        .unwrap()
                        .relation_estimates()
                        .iter()
                        .filter(|row| {
                            row.kind() == GeneralSemPlsHocPointRelationKindV1::ExtendedTotalEffect
                        })
                        .count(),
                    1
                );
            }
        }
    }

    #[test]
    fn reflective_reflective_disjoint_point_matches_the_legacy_two_stage_fixture() {
        let (dataset, recipe, _, _, point) =
            fixture(HigherOrderMeasurementTypeV4::ReflectiveReflective);
        let mut legacy_model = source_model(MeasurementMode::Reflective);
        legacy_model.constructs.push(Construct {
            id: "derived:hoc".into(),
            name: "Higher order".into(),
            short_name: "HOC".into(),
            mode: MeasurementMode::Reflective,
            indicators: Vec::new(),
        });
        legacy_model.paths.push(StructuralPath {
            source: "a".into(),
            target: "derived:hoc".into(),
        });
        legacy_model.paths.push(StructuralPath {
            source: "derived:hoc".into(),
            target: "y".into(),
        });
        legacy_model
            .higher_order_constructs
            .push(HigherOrderConstruct {
                id: "derived:hoc".into(),
                components: vec!["x".into(), "z".into()],
                method: HigherOrderMethod::TwoStage,
                stage_one_recipe: None,
            });
        let legacy_recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x484f_435f_4c45_4741_4359_5631),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: legacy_model,
            settings: recipe.settings.clone(),
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let legacy = estimate_pls(&dataset, &legacy_recipe).unwrap();
        for (legacy_source, legacy_target, schema6_source, schema6_target) in [
            ("a", "derived:hoc", "construct:a", "derived:hoc"),
            ("derived:hoc", "y", "derived:hoc", "construct:y"),
        ] {
            let legacy_path = legacy
                .paths
                .iter()
                .find(|path| path.source == legacy_source && path.target == legacy_target)
                .unwrap()
                .coefficient;
            let schema6_path = point
                .structural_relation_estimates()
                .find(|estimate| {
                    estimate.source_id() == schema6_source && estimate.target_id() == schema6_target
                })
                .unwrap()
                .estimate();
            assert!((legacy_path - schema6_path).abs() <= 1e-12);
        }

        let legacy_loadings = [("construct:x", "x"), ("construct:z", "z")]
            .into_iter()
            .map(|(schema6_component, legacy_component)| {
                (
                    schema6_component,
                    legacy
                        .outer_estimates
                        .iter()
                        .find(|estimate| {
                            estimate.construct == "derived:hoc"
                                && estimate.indicator
                                    == format!("__qpls_hoc_derived:hoc_{legacy_component}")
                        })
                        .unwrap()
                        .loading,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let schema6_loadings = point.stages()[1]
            .relation_estimates()
            .iter()
            .filter(|estimate| {
                estimate.kind() == GeneralSemPlsHocPointRelationKindV1::ComponentLoading
            })
            .map(|estimate| (estimate.target_id(), estimate.estimate()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            legacy_loadings.keys().copied().collect::<Vec<_>>(),
            schema6_loadings.keys().copied().collect::<Vec<_>>()
        );
        for (component, legacy) in &legacy_loadings {
            assert!((*legacy - schema6_loadings[component]).abs() <= 1e-12);
        }
    }

    #[test]
    fn typed_hoc_point_result_rejects_component_identity_tampering() {
        let (_, recipe, model, _, mut point) =
            fixture(HigherOrderMeasurementTypeV4::ReflectiveReflective);
        let plan =
            compile_pls_plan_v3(&model, recipe.general_sem_config.as_ref().unwrap()).unwrap();
        let component = point.stages[1]
            .relation_estimates
            .iter_mut()
            .find(|estimate| estimate.kind == GeneralSemPlsHocPointRelationKindV1::ComponentLoading)
            .unwrap();
        component.parameter_id = "parameter:tampered".into();
        assert!(matches!(
            point.ensure_valid_against_plan_v1(&plan),
            Err(GeneralSemPlsHigherOrderPointErrorV1::InvalidResultContract(
                _
            ))
        ));
    }
}
