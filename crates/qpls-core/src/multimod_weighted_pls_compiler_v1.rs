//! Additive compiled point authority for MultiMod weighted PLS.
//!
//! The public Recipe V4 PLS compiler intentionally continues to reject
//! weights.  MultiMod obtains a separately typed authority: the original
//! weighted recipe/model and weight semantics are hashed into this wrapper,
//! while its embedded ordinary `CompiledAnalysisRecipeV4` is compiled from the
//! exact interaction-free, weight-free score model.  Execution must validate
//! the wrapper and then apply the bound row masses in every estimator stage.

use crate::{
    AnalysisMethod, AnalysisRecipeModelBindingV4, AnalysisRecipeV4, AnalysisWeightBindingV1,
    CompiledAnalysisRecipeV4, CompiledRecipePlanV4, MethodConfig, RecipeV4CompilationError,
    RecipeV4CompilerTarget, ResolvedWeightBindingV1, SemDataBindingV4, SemDerivedTermV4,
    SemModelV4, SemRelationV4, compile_analysis_recipe_v4, compile_pls_stage_one_projection_v3,
    recipe_v4_analytical_sha256, resolve_weight_declaration_v1, sha256_serialized,
};
use serde::{Deserialize, Serialize};

pub const MULTIMOD_WEIGHTED_PLS_COMPILER_VERSION_V1: &str = "multimod_weighted_pls_compiler_v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultimodCompiledWeightSemanticsV1 {
    PositiveCase,
    PositiveIntegerFrequencyCountSpace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultimodWeightedPlsCompilationReceiptV1 {
    compiler_version: String,
    source_recipe_analytical_sha256: String,
    source_model_scientific_sha256: String,
    source_dataset_fingerprint: String,
    weight_semantics: MultimodCompiledWeightSemanticsV1,
    weight_variable_id: String,
    weight_source_column: String,
    point_recipe_sha256: String,
    point_model_scientific_sha256: String,
    point_artifact_sha256: String,
}

impl MultimodWeightedPlsCompilationReceiptV1 {
    pub fn compiler_version(&self) -> &str {
        &self.compiler_version
    }

    pub fn source_recipe_analytical_sha256(&self) -> &str {
        &self.source_recipe_analytical_sha256
    }

    pub fn source_model_scientific_sha256(&self) -> &str {
        &self.source_model_scientific_sha256
    }

    pub fn source_dataset_fingerprint(&self) -> &str {
        &self.source_dataset_fingerprint
    }

    pub fn weight_semantics(&self) -> MultimodCompiledWeightSemanticsV1 {
        self.weight_semantics
    }

    pub fn weight_variable_id(&self) -> &str {
        &self.weight_variable_id
    }

    pub fn weight_source_column(&self) -> &str {
        &self.weight_source_column
    }

    pub fn point_recipe_sha256(&self) -> &str {
        &self.point_recipe_sha256
    }

    pub fn point_model_scientific_sha256(&self) -> &str {
        &self.point_model_scientific_sha256
    }

    pub fn point_artifact_sha256(&self) -> &str {
        &self.point_artifact_sha256
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultimodCompiledWeightedPlsRecipeV1 {
    point_recipe: AnalysisRecipeV4,
    point_model: SemModelV4,
    point_artifact: CompiledAnalysisRecipeV4,
    receipt: MultimodWeightedPlsCompilationReceiptV1,
}

impl MultimodCompiledWeightedPlsRecipeV1 {
    pub fn point_recipe(&self) -> &AnalysisRecipeV4 {
        &self.point_recipe
    }

    pub fn point_model(&self) -> &SemModelV4 {
        &self.point_model
    }

    pub fn point_artifact(&self) -> &CompiledAnalysisRecipeV4 {
        &self.point_artifact
    }

    pub fn receipt(&self) -> &MultimodWeightedPlsCompilationReceiptV1 {
        &self.receipt
    }

    pub fn plan(&self) -> &crate::CompiledPlsPlanV2 {
        let CompiledRecipePlanV4::PlsPlanV2 { plan } = self.point_artifact.plan() else {
            unreachable!("the weighted authority compiler always emits a PLS plan")
        };
        plan
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum MultimodWeightedPlsCompilationErrorV1 {
    #[error(transparent)]
    Recipe(#[from] RecipeV4CompilationError),
    #[error(transparent)]
    RecipeValidation(#[from] crate::AnalysisRecipeV4Error),
    #[error(transparent)]
    ModelValidation(#[from] crate::SemModelV4ValidationError),
    #[error(transparent)]
    Weight(#[from] crate::WeightDeclarationResolutionErrorV1),
    #[error("MultiMod weighted PLS requires one typed case or frequency declaration")]
    MissingWeight,
    #[error("requested MultiMod weight semantics differ from SemModelV4")]
    WeightSemanticsMismatch,
    #[error("requested MultiMod weight column differs from SemModelV4")]
    WeightColumnMismatch,
    #[error("the weight control variable participates in the scientific model")]
    ScientificWeightVariable,
    #[error("weighted PLS point projection failed: {0}")]
    StageOneProjection(String),
    #[error("compiled weighted PLS authority differs from deterministic recompilation")]
    AuthorityMismatch,
}

/// Compile a MultiMod-only weighted point authority without changing the
/// ordinary Recipe V4 compiler's public rejection of weighted PLS.
pub fn compile_multimod_weighted_pls_recipe_v4_v1(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    requested_weight: &AnalysisWeightBindingV1,
) -> Result<MultimodCompiledWeightedPlsRecipeV1, MultimodWeightedPlsCompilationErrorV1> {
    recipe.ensure_valid()?;
    model.ensure_valid()?;
    let declaration = resolve_weight_declaration_v1(model)?
        .ok_or(MultimodWeightedPlsCompilationErrorV1::MissingWeight)?;
    let (semantics, variable_id, source_column) = match declaration.binding() {
        ResolvedWeightBindingV1::Case {
            variable_id,
            source_column,
        } => (
            MultimodCompiledWeightSemanticsV1::PositiveCase,
            variable_id.clone(),
            source_column.clone(),
        ),
        ResolvedWeightBindingV1::Frequency {
            variable_id,
            source_column,
        } => (
            MultimodCompiledWeightSemanticsV1::PositiveIntegerFrequencyCountSpace,
            variable_id.clone(),
            source_column.clone(),
        ),
        ResolvedWeightBindingV1::Sampling { .. } => {
            return Err(MultimodWeightedPlsCompilationErrorV1::WeightSemanticsMismatch);
        }
    };
    let (requested_semantics, requested_column) = match requested_weight {
        AnalysisWeightBindingV1::Case { column } => {
            (MultimodCompiledWeightSemanticsV1::PositiveCase, column)
        }
        AnalysisWeightBindingV1::Frequency { column } => (
            MultimodCompiledWeightSemanticsV1::PositiveIntegerFrequencyCountSpace,
            column,
        ),
    };
    if requested_semantics != semantics {
        return Err(MultimodWeightedPlsCompilationErrorV1::WeightSemanticsMismatch);
    }
    if requested_column != &source_column && requested_column != &variable_id {
        return Err(MultimodWeightedPlsCompilationErrorV1::WeightColumnMismatch);
    }
    if scientific_variable_reference_v1(model, &variable_id) {
        return Err(MultimodWeightedPlsCompilationErrorV1::ScientificWeightVariable);
    }

    let projection = compile_pls_stage_one_projection_v3(model).map_err(|error| {
        MultimodWeightedPlsCompilationErrorV1::StageOneProjection(error.to_string())
    })?;
    let mut point_model = projection.projected_model().clone();
    if let SemDataBindingV4::Raw { weight, .. } = &mut point_model.data_binding {
        *weight = None;
    }
    point_model
        .variables
        .retain(|variable| variable.id() != variable_id);
    point_model.annotations.clear();
    point_model.presentation = Default::default();
    point_model = point_model.canonicalized();
    point_model.ensure_valid()?;

    let point_scientific_sha256 = point_model.scientific_sha256()?;
    let mut point_recipe = recipe.clone();
    point_recipe.settings.method = AnalysisMethod::PlsPm;
    point_recipe.settings.bootstrap_samples = 0;
    point_recipe.settings.permutation_samples = 0;
    point_recipe.settings.studentized_inner_samples = 0;
    point_recipe.settings.case_weight_column = None;
    point_recipe.method_config = Some(MethodConfig::PlsAlgorithm);
    point_recipe.general_sem_config = None;
    point_recipe.mga_multigroup = None;
    point_recipe.pls_heterogeneity = None;
    point_recipe.general_sem_conditional_process = None;
    point_recipe.interventional_causal_mediation = None;
    point_recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        scientific_sha256: point_scientific_sha256.clone(),
        model: point_model.clone(),
    };
    point_recipe.ensure_valid()?;
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let point_artifact = compile_analysis_recipe_v4(
        &point_recipe,
        Some(&point_model),
        target,
        target.capability_cell_for_recipe(&point_recipe),
    )?;
    let receipt = MultimodWeightedPlsCompilationReceiptV1 {
        compiler_version: MULTIMOD_WEIGHTED_PLS_COMPILER_VERSION_V1.into(),
        source_recipe_analytical_sha256: recipe_v4_analytical_sha256(recipe, model)?,
        source_model_scientific_sha256: model.scientific_sha256()?,
        source_dataset_fingerprint: recipe.dataset_fingerprint.clone(),
        weight_semantics: semantics,
        weight_variable_id: variable_id,
        weight_source_column: source_column,
        point_recipe_sha256: sha256_serialized(&point_recipe),
        point_model_scientific_sha256: point_scientific_sha256,
        point_artifact_sha256: sha256_serialized(&point_artifact),
    };
    Ok(MultimodCompiledWeightedPlsRecipeV1 {
        point_recipe,
        point_model,
        point_artifact,
        receipt,
    })
}

pub fn validate_multimod_weighted_pls_recipe_v4_v1(
    authority: &MultimodCompiledWeightedPlsRecipeV1,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    requested_weight: &AnalysisWeightBindingV1,
) -> Result<(), MultimodWeightedPlsCompilationErrorV1> {
    let expected = compile_multimod_weighted_pls_recipe_v4_v1(recipe, model, requested_weight)?;
    if expected != *authority {
        return Err(MultimodWeightedPlsCompilationErrorV1::AuthorityMismatch);
    }
    Ok(())
}

fn scientific_variable_reference_v1(model: &SemModelV4, variable_id: &str) -> bool {
    model.relations.iter().any(|relation| match relation {
        SemRelationV4::MeasurementEffect {
            construct,
            indicator,
            ..
        }
        | SemRelationV4::MeasurementCausal {
            indicator,
            composite: construct,
            ..
        } => construct == variable_id || indicator == variable_id,
        SemRelationV4::Structural { source, target, .. } => {
            source == variable_id || target == variable_id
        }
        SemRelationV4::Covariance { left, right, .. } => {
            left.variable_id() == variable_id || right.variable_id() == variable_id
        }
    }) || model.derived_terms.iter().any(|term| match term {
        SemDerivedTermV4::Interaction {
            predictor,
            moderator,
            output,
            ..
        } => predictor == variable_id || moderator == variable_id || output == variable_id,
        SemDerivedTermV4::InteractionV2 {
            operands, output, ..
        } => operands.iter().any(|operand| operand == variable_id) || output == variable_id,
        SemDerivedTermV4::HigherOrder {
            components, output, ..
        } => components.iter().any(|component| component == variable_id) || output == variable_id,
        SemDerivedTermV4::Polynomial { source, output, .. } => {
            source == variable_id || output == variable_id
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_version_is_additive_and_stable() {
        assert_eq!(
            MULTIMOD_WEIGHTED_PLS_COMPILER_VERSION_V1,
            "multimod_weighted_pls_compiler_v1"
        );
    }
}
