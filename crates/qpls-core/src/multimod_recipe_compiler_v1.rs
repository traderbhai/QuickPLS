use crate::{
    AnalysisRecipeV4, AnalysisRecipeV4Error, AnalysisWeightBindingV1, CapabilityCellReferenceV2,
    CompositeWeightingV4, ConditionalProbeScaleV2, ConditionalProcessPathV2,
    ConditionalProcessProfileV2, GeneralSemConditionalProcessConfigV2, HeterogeneityAlgorithmV2,
    HeterogeneityInteractionProfileV2, InteractionHierarchyPolicyV2, InteractionMethodV4,
    InterventionalCausalMediationConfigV1, MgaModelProfileV1, MgaMultigroupV1, MissingDataPolicyV4,
    MultiModConfigErrorV1, ObservedScaleV4, ObservedTreatmentContrastV1,
    PlsUnobservedHeterogeneityConfigV2, SemDataBindingV4, SemDerivedTermV4, SemGroupV4, SemModelV4,
    SemModelV4ValidationError, SemRelationV4, SemVariableV4, SemWeightBindingV4,
    conditional_process_target_upper_bound_v2, recipe_v4_analytical_sha256, sha256_serialized,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MULTIMOD_RECIPE_COMPILER_VERSION_V1: &str = "multimod_recipe_compiler_v1";
pub const MULTIMOD_COMPILATION_RECEIPT_SCHEMA_VERSION_V1: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultiModCompilerTargetV1 {
    MgaMultigroupV1,
    PlsHeterogeneityV2,
    GeneralSemConditionalProcessV2,
    InterventionalCausalMediationV1,
}

impl MultiModCompilerTargetV1 {
    pub const fn method_version(self) -> &'static str {
        match self {
            Self::MgaMultigroupV1 => "mga_multigroup_v1",
            Self::PlsHeterogeneityV2 => "pls_heterogeneity_v2",
            Self::GeneralSemConditionalProcessV2 => "general_sem_conditional_process_v2",
            Self::InterventionalCausalMediationV1 => "interventional_causal_mediation_v1",
        }
    }

    pub fn capability_cell(self) -> CapabilityCellReferenceV2 {
        let cell_id = match self {
            Self::MgaMultigroupV1 => "qpls.multimod.mga_multigroup_v1",
            Self::PlsHeterogeneityV2 => "qpls.multimod.pls_heterogeneity_v2",
            Self::GeneralSemConditionalProcessV2 => {
                "qpls.multimod.general_sem_conditional_process_v2"
            }
            Self::InterventionalCausalMediationV1 => {
                "qpls.multimod.interventional_causal_mediation_v1"
            }
        };
        CapabilityCellReferenceV2 {
            registry_schema_version: 2,
            capability_id: "quickpls.multimod".into(),
            cell_id: cell_id.into(),
            capability_version: self.method_version().into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledInteractionIdentityV1 {
    pub term_id: String,
    pub output_id: String,
    pub operands: Vec<String>,
    pub focal_relation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledHocIdentityV1 {
    pub term_id: String,
    pub output_id: String,
    pub components: Vec<String>,
    pub approach: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledMultiModPlanV1 {
    MgaMultigroupV1 {
        profile: MgaModelProfileV1,
        grouping_column: String,
        group_ids: Vec<String>,
        selected_parameter_ids: Vec<String>,
        interactions: Vec<CompiledInteractionIdentityV1>,
        hocs: Vec<CompiledHocIdentityV1>,
    },
    PlsHeterogeneityV2 {
        profile: HeterogeneityInteractionProfileV2,
        algorithms: Vec<HeterogeneityAlgorithmV2>,
        candidate_k: Vec<u8>,
        interactions: Vec<CompiledInteractionIdentityV1>,
    },
    GeneralSemConditionalProcessV2 {
        profile: ConditionalProcessProfileV2,
        paths: Vec<ConditionalProcessPathV2>,
        interactions: Vec<CompiledInteractionIdentityV1>,
        hocs: Vec<CompiledHocIdentityV1>,
        compiled_target_upper_bound: usize,
    },
    InterventionalCausalMediationV1 {
        treatment: String,
        outcome: String,
        mediator_ids: Vec<String>,
        path_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiModCompilationReceiptV1 {
    pub schema_version: u32,
    pub target: MultiModCompilerTargetV1,
    pub compiler_version: String,
    pub method_version: String,
    pub recipe_id: String,
    pub recipe_analytical_sha256: String,
    pub config_sha256: String,
    pub model_id: String,
    pub model_scientific_sha256: String,
    pub dataset_id: String,
    pub dataset_fingerprint: String,
    pub plan_sha256: String,
    pub analytical_identity_sha256: String,
    pub capability_cell: CapabilityCellReferenceV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledMultiModRecipeV1 {
    plan: CompiledMultiModPlanV1,
    receipt: MultiModCompilationReceiptV1,
}

impl CompiledMultiModRecipeV1 {
    pub fn plan(&self) -> &CompiledMultiModPlanV1 {
        &self.plan
    }

    pub fn receipt(&self) -> &MultiModCompilationReceiptV1 {
        &self.receipt
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum MultiModCompilationErrorV1 {
    #[error(transparent)]
    InvalidRecipe(#[from] AnalysisRecipeV4Error),
    #[error(transparent)]
    InvalidModel(#[from] SemModelV4ValidationError),
    #[error(transparent)]
    InvalidConfig(#[from] MultiModConfigErrorV1),
    #[error("the requested MultiMod compiler target has no matching configuration")]
    TargetConfigurationMismatch,
    #[error("MultiMod execution requires raw rows with listwise deletion")]
    RawListwiseRowsRequired,
    #[error(
        "clusters, strata, sampling weights, covariance input, and correlation input are unsupported"
    )]
    UnsupportedDataDesign,
    #[error("grouping column {0} is used as a model indicator")]
    GroupingColumnIsIndicator(String),
    #[error("the model group declaration disagrees with the selected grouping column")]
    GroupDeclarationMismatch,
    #[error("the selected MultiMod profile is incompatible with the model: {0}")]
    ProfileMismatch(String),
    #[error("explicit path {path_id} is invalid: {reason}")]
    InvalidPath { path_id: String, reason: String },
    #[error("causal role {0} is not a directly observed variable")]
    CausalRoleNotObserved(String),
    #[error(
        "the model has latent, composite, derived, grouped, or weighted features outside causal V1"
    )]
    CausalModelScope,
    #[error("compiled MultiMod artifact differs from deterministic recompilation")]
    ArtifactMismatch,
}

#[derive(Serialize)]
struct MultiModAnalyticalIdentityV1<'a> {
    compiler_version: &'static str,
    target: MultiModCompilerTargetV1,
    recipe_analytical_sha256: &'a str,
    config_sha256: &'a str,
    model_id: &'a str,
    model_scientific_sha256: &'a str,
    dataset_id: &'a str,
    plan_sha256: &'a str,
    capability_cell: &'a CapabilityCellReferenceV2,
}

pub fn compile_multimod_recipe_v1(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    target: MultiModCompilerTargetV1,
) -> Result<CompiledMultiModRecipeV1, MultiModCompilationErrorV1> {
    let canonical_causal_config =
        if target == MultiModCompilerTargetV1::InterventionalCausalMediationV1 {
            Some(canonicalize_interventional_causal_config_v1(
                model,
                recipe
                    .interventional_causal_mediation
                    .as_ref()
                    .ok_or(MultiModCompilationErrorV1::TargetConfigurationMismatch)?,
            )?)
        } else {
            None
        };
    let mut validation_recipe = recipe.clone();
    if let Some(config) = &canonical_causal_config {
        validation_recipe.interventional_causal_mediation = Some(config.clone());
    }
    validation_recipe.ensure_valid()?;
    model.ensure_valid()?;
    ensure_recipe_model_binding(recipe, model)?;
    ensure_raw_listwise(model)?;

    let interactions = compiled_interactions(model);
    let hocs = compiled_hocs(model);
    let (plan, config_sha256) = match target {
        MultiModCompilerTargetV1::MgaMultigroupV1 => {
            let config = recipe
                .mga_multigroup
                .as_ref()
                .ok_or(MultiModCompilationErrorV1::TargetConfigurationMismatch)?;
            compile_mga_plan(model, config, interactions, hocs)?
        }
        MultiModCompilerTargetV1::PlsHeterogeneityV2 => {
            let config = recipe
                .pls_heterogeneity
                .as_ref()
                .ok_or(MultiModCompilationErrorV1::TargetConfigurationMismatch)?;
            compile_heterogeneity_plan(model, config, interactions, hocs)?
        }
        MultiModCompilerTargetV1::GeneralSemConditionalProcessV2 => {
            let config = recipe
                .general_sem_conditional_process
                .as_ref()
                .ok_or(MultiModCompilationErrorV1::TargetConfigurationMismatch)?;
            compile_conditional_plan(model, config, interactions, hocs)?
        }
        MultiModCompilerTargetV1::InterventionalCausalMediationV1 => {
            let authored_config = recipe
                .interventional_causal_mediation
                .as_ref()
                .ok_or(MultiModCompilationErrorV1::TargetConfigurationMismatch)?;
            let config = canonical_causal_config
                .as_ref()
                .ok_or(MultiModCompilationErrorV1::TargetConfigurationMismatch)?;
            compile_causal_plan(
                model,
                config,
                interactions,
                hocs,
                sha256_serialized(authored_config),
            )?
        }
    };
    let recipe_analytical_sha256 = recipe_v4_analytical_sha256(recipe, model)
        .map_err(|error| MultiModCompilationErrorV1::ProfileMismatch(error.to_string()))?;
    let model_scientific_sha256 = model.scientific_sha256()?;
    let dataset_id = match &model.data_binding {
        SemDataBindingV4::Raw { dataset_id, .. }
        | SemDataBindingV4::Covariance { dataset_id, .. }
        | SemDataBindingV4::Correlation { dataset_id, .. } => dataset_id.clone(),
    };
    let plan_sha256 = sha256_serialized(&plan);
    let capability_cell = target.capability_cell();
    let analytical_identity_sha256 = sha256_serialized(&MultiModAnalyticalIdentityV1 {
        compiler_version: MULTIMOD_RECIPE_COMPILER_VERSION_V1,
        target,
        recipe_analytical_sha256: &recipe_analytical_sha256,
        config_sha256: &config_sha256,
        model_id: &model.id,
        model_scientific_sha256: &model_scientific_sha256,
        dataset_id: &dataset_id,
        plan_sha256: &plan_sha256,
        capability_cell: &capability_cell,
    });
    Ok(CompiledMultiModRecipeV1 {
        plan,
        receipt: MultiModCompilationReceiptV1 {
            schema_version: MULTIMOD_COMPILATION_RECEIPT_SCHEMA_VERSION_V1,
            target,
            compiler_version: MULTIMOD_RECIPE_COMPILER_VERSION_V1.into(),
            method_version: target.method_version().into(),
            recipe_id: recipe.id.to_string(),
            recipe_analytical_sha256,
            config_sha256,
            model_id: model.id.clone(),
            model_scientific_sha256,
            dataset_id,
            dataset_fingerprint: recipe.dataset_fingerprint.clone(),
            plan_sha256,
            analytical_identity_sha256,
            capability_cell,
        },
    })
}

pub fn validate_compiled_multimod_recipe_v1(
    artifact: &CompiledMultiModRecipeV1,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
) -> Result<(), MultiModCompilationErrorV1> {
    let expected = compile_multimod_recipe_v1(recipe, model, artifact.receipt.target)?;
    if expected == *artifact {
        Ok(())
    } else {
        Err(MultiModCompilationErrorV1::ArtifactMismatch)
    }
}

fn ensure_recipe_model_binding(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
) -> Result<(), MultiModCompilationErrorV1> {
    if recipe.model_binding.model_id() != model.id {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "recipe/model identifier mismatch".into(),
        ));
    }
    let bound_digest = match &recipe.model_binding {
        crate::AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256, ..
        }
        | crate::AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            scientific_sha256,
            ..
        } => scientific_sha256,
        crate::AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified { .. } => {
            return Err(MultiModCompilationErrorV1::ProfileMismatch(
                "legacy estimand is not confirmed".into(),
            ));
        }
    };
    if model.scientific_sha256()? != *bound_digest {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "recipe/model scientific digest mismatch".into(),
        ));
    }
    Ok(())
}

fn ensure_raw_listwise(model: &SemModelV4) -> Result<(), MultiModCompilationErrorV1> {
    match &model.data_binding {
        SemDataBindingV4::Raw {
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            cluster_variable: None,
            strata_variable: None,
            weight,
            ..
        } => {
            if matches!(weight, Some(SemWeightBindingV4::Sampling { .. })) {
                Err(MultiModCompilationErrorV1::UnsupportedDataDesign)
            } else {
                Ok(())
            }
        }
        SemDataBindingV4::Raw { .. } => Err(MultiModCompilationErrorV1::UnsupportedDataDesign),
        _ => Err(MultiModCompilationErrorV1::RawListwiseRowsRequired),
    }
}

fn compiled_interactions(model: &SemModelV4) -> Vec<CompiledInteractionIdentityV1> {
    model
        .derived_terms
        .iter()
        .filter_map(|term| match term {
            SemDerivedTermV4::Interaction {
                id,
                output,
                predictor,
                moderator,
                focal_relation,
                ..
            } => Some(CompiledInteractionIdentityV1 {
                term_id: id.clone(),
                output_id: output.clone(),
                operands: vec![predictor.clone(), moderator.clone()],
                focal_relation_id: focal_relation.clone(),
            }),
            SemDerivedTermV4::InteractionV2 {
                id,
                output,
                operands,
                focal_relation,
                ..
            } => Some(CompiledInteractionIdentityV1 {
                term_id: id.clone(),
                output_id: output.clone(),
                operands: operands.clone(),
                focal_relation_id: focal_relation.clone(),
            }),
            _ => None,
        })
        .collect()
}

fn compiled_hocs(model: &SemModelV4) -> Vec<CompiledHocIdentityV1> {
    model
        .derived_terms
        .iter()
        .filter_map(|term| match term {
            SemDerivedTermV4::HigherOrder {
                id,
                output,
                components,
                approach,
                ..
            } => Some(CompiledHocIdentityV1 {
                term_id: id.clone(),
                output_id: output.clone(),
                components: components.clone(),
                approach: format!("{approach:?}").to_ascii_lowercase(),
            }),
            _ => None,
        })
        .collect()
}

fn ensure_grouping_not_indicator(
    model: &SemModelV4,
    grouping_column: &str,
) -> Result<(), MultiModCompilationErrorV1> {
    let grouping_variable_ids = model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } if source_column == grouping_column || id == grouping_column => Some(id.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let is_indicator = model.relations.iter().any(|relation| match relation {
        SemRelationV4::MeasurementEffect { indicator, .. }
        | SemRelationV4::MeasurementCausal { indicator, .. } => {
            grouping_variable_ids.contains(indicator.as_str())
        }
        _ => false,
    });
    if is_indicator {
        Err(MultiModCompilationErrorV1::GroupingColumnIsIndicator(
            grouping_column.into(),
        ))
    } else {
        Ok(())
    }
}

fn ensure_group_declaration(
    model: &SemModelV4,
    grouping_column: &str,
) -> Result<(), MultiModCompilationErrorV1> {
    match &model.group {
        SemGroupV4::SingleGroup => Ok(()),
        SemGroupV4::ObservedGroups {
            grouping_variable, ..
        } if grouping_variable == grouping_column
            || model.variables.iter().any(|variable| {
                matches!(variable, SemVariableV4::Observed { id, source_column, .. }
                    if id == grouping_variable && source_column == grouping_column)
            }) =>
        {
            Ok(())
        }
        SemGroupV4::ObservedGroups { .. } => {
            Err(MultiModCompilationErrorV1::GroupDeclarationMismatch)
        }
    }
}

fn ensure_interactions_are_current_two_stage_strong(
    model: &SemModelV4,
    allow_three_way: bool,
) -> Result<(), MultiModCompilationErrorV1> {
    for term in &model.derived_terms {
        match term {
            SemDerivedTermV4::InteractionV2 {
                operands,
                method: InteractionMethodV4::TwoStage,
                hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
                product_indicator: None,
                ..
            } if operands.len() == 2 || (allow_three_way && operands.len() == 3) => {}
            SemDerivedTermV4::Interaction { .. } | SemDerivedTermV4::InteractionV2 { .. } => {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "interactions require InteractionV2, TwoStage, Strong hierarchy, and order admitted by the profile".into(),
                ));
            }
            SemDerivedTermV4::Polynomial { .. } => {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "polynomial/self-moderation terms are outside MultiMod".into(),
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn ensure_hocs_disjoint_and_homogeneous(
    model: &SemModelV4,
    hocs: &[CompiledHocIdentityV1],
) -> Result<(), MultiModCompilationErrorV1> {
    if hocs.is_empty() || hocs.len() > 4 {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "multiple-HOC profiles require one through four HOCs".into(),
        ));
    }
    let mut components = BTreeSet::new();
    let mut approaches = BTreeSet::new();
    let outputs = hocs
        .iter()
        .map(|hoc| hoc.output_id.as_str())
        .collect::<BTreeSet<_>>();
    for hoc in hocs {
        approaches.insert(hoc.approach.as_str());
        if hoc.approach.contains("hybrid")
            || hoc
                .components
                .iter()
                .any(|component| outputs.contains(component.as_str()))
            || hoc
                .components
                .iter()
                .any(|component| !components.insert(component.as_str()))
        {
            return Err(MultiModCompilationErrorV1::ProfileMismatch(
                "HOCs must be non-hybrid, nonnested, pairwise-disjoint, and use one approach"
                    .into(),
            ));
        }
    }
    if approaches.len() != 1 {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "mixed HOC approaches are outside the qualified profile".into(),
        ));
    }
    if model
        .derived_terms
        .iter()
        .any(|term| matches!(term, SemDerivedTermV4::Polynomial { .. }))
    {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "HOC profiles cannot contain polynomial terms".into(),
        ));
    }
    Ok(())
}

fn ensure_reflective_plsc_model(model: &SemModelV4) -> Result<(), MultiModCompilationErrorV1> {
    let mut construct_count = 0usize;
    for variable in &model.variables {
        match variable {
            SemVariableV4::Composite {
                id,
                weighting: CompositeWeightingV4::ModeA,
                ..
            } => {
                construct_count += 1;
                let indicators = model
                    .relations
                    .iter()
                    .filter(|relation| {
                        matches!(relation, SemRelationV4::MeasurementEffect { construct, .. }
                            if construct == id)
                    })
                    .count();
                if indicators < 2 {
                    return Err(MultiModCompilationErrorV1::ProfileMismatch(format!(
                        "reflective PLSc construct {id} requires at least two indicators"
                    )));
                }
            }
            SemVariableV4::Composite { id, .. } | SemVariableV4::CommonFactor { id, .. } => {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(format!(
                    "reflective PLSc profile rejects non-Mode-A construct {id}"
                )));
            }
            _ => {}
        }
    }
    if construct_count == 0
        || model
            .relations
            .iter()
            .any(|relation| matches!(relation, SemRelationV4::MeasurementCausal { .. }))
    {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "reflective PLSc requires only Mode-A reflective constructs".into(),
        ));
    }
    Ok(())
}

fn ensure_weight_binding(
    model: &SemModelV4,
    requested: Option<&AnalysisWeightBindingV1>,
) -> Result<(), MultiModCompilationErrorV1> {
    let resident = match &model.data_binding {
        SemDataBindingV4::Raw { weight, .. } => weight.as_ref(),
        _ => None,
    };
    let coherent = match (requested, resident) {
        (None, None) => true,
        (
            Some(AnalysisWeightBindingV1::Case { column }),
            Some(SemWeightBindingV4::Case { variable }),
        )
        | (
            Some(AnalysisWeightBindingV1::Frequency { column }),
            Some(SemWeightBindingV4::Frequency { variable }),
        ) => {
            column == variable
                || model.variables.iter().any(|candidate| {
                    matches!(candidate, SemVariableV4::Observed { id, source_column, .. }
                    if id == variable && source_column == column)
                })
        }
        _ => false,
    };
    if coherent {
        Ok(())
    } else {
        Err(MultiModCompilationErrorV1::ProfileMismatch(
            "recipe and SemModelV4 weight declarations differ".into(),
        ))
    }
}

fn compile_mga_plan(
    model: &SemModelV4,
    config: &MgaMultigroupV1,
    interactions: Vec<CompiledInteractionIdentityV1>,
    hocs: Vec<CompiledHocIdentityV1>,
) -> Result<(CompiledMultiModPlanV1, String), MultiModCompilationErrorV1> {
    config.ensure_valid()?;
    ensure_grouping_not_indicator(model, &config.grouping_column)?;
    ensure_group_declaration(model, &config.grouping_column)?;
    match config.profile {
        MgaModelProfileV1::GeneralSemPls | MgaModelProfileV1::ReflectivePlsc => {
            if !interactions.is_empty() || !hocs.is_empty() {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "ordinary/PLSc MGA profiles cannot contain interactions or HOCs".into(),
                ));
            }
            ensure_weight_binding(model, None)?;
            if config.profile == MgaModelProfileV1::ReflectivePlsc {
                ensure_reflective_plsc_model(model)?;
            }
        }
        MgaModelProfileV1::MultipleTwoWayModeration => {
            ensure_interactions_are_current_two_stage_strong(model, false)?;
            if interactions.is_empty()
                || interactions.iter().any(|term| term.operands.len() != 2)
                || !hocs.is_empty()
            {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "multiple-two-way MGA requires one or more two-way interactions and no HOCs"
                        .into(),
                ));
            }
            ensure_weight_binding(model, None)?;
        }
        MgaModelProfileV1::BoundedThreeWayModeration => {
            ensure_interactions_are_current_two_stage_strong(model, true)?;
            if interactions.len() != 4
                || interactions
                    .iter()
                    .filter(|term| term.operands.len() == 3)
                    .count()
                    != 1
                || !hocs.is_empty()
            {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "bounded three-way MGA requires exactly one three-way term and no HOCs".into(),
                ));
            }
            ensure_weight_binding(model, None)?;
        }
        MgaModelProfileV1::BoundedTwoWayModeratedMediation => {
            ensure_interactions_are_current_two_stage_strong(model, false)?;
            if interactions.len() != 1 || interactions[0].operands.len() != 2 || !hocs.is_empty() {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "bounded moderated-mediation MGA requires exactly one two-way interaction"
                        .into(),
                ));
            }
            ensure_weight_binding(model, None)?;
        }
        MgaModelProfileV1::MultipleNonnestedHoc => {
            if !interactions.is_empty() {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "multiple-HOC MGA cannot contain interactions".into(),
                ));
            }
            ensure_hocs_disjoint_and_homogeneous(model, &hocs)?;
            ensure_weight_binding(model, None)?;
        }
        MgaModelProfileV1::CaseWeightedPls | MgaModelProfileV1::FrequencyWeightedPls => {
            if !interactions.is_empty() || !hocs.is_empty() {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "weighted MGA profiles cannot contain interactions or HOCs".into(),
                ));
            }
            ensure_weight_binding(model, config.weight.as_ref())?;
        }
    }
    let plan = CompiledMultiModPlanV1::MgaMultigroupV1 {
        profile: config.profile,
        grouping_column: config.grouping_column.clone(),
        group_ids: config
            .groups
            .iter()
            .map(|group| group.group_id.clone())
            .collect(),
        selected_parameter_ids: config.selected_parameter_ids.clone(),
        interactions,
        hocs,
    };
    Ok((plan, sha256_serialized(config)))
}

fn compile_heterogeneity_plan(
    model: &SemModelV4,
    config: &PlsUnobservedHeterogeneityConfigV2,
    interactions: Vec<CompiledInteractionIdentityV1>,
    hocs: Vec<CompiledHocIdentityV1>,
) -> Result<(CompiledMultiModPlanV1, String), MultiModCompilationErrorV1> {
    config.ensure_valid()?;
    if !matches!(model.group, SemGroupV4::SingleGroup) || !hocs.is_empty() {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "heterogeneity v2 requires one unweighted group and no HOCs".into(),
        ));
    }
    ensure_weight_binding(model, None)?;
    match config.profile {
        HeterogeneityInteractionProfileV2::P0Structural if !interactions.is_empty() => {
            return Err(MultiModCompilationErrorV1::ProfileMismatch(
                "P0 cannot contain interaction terms".into(),
            ));
        }
        HeterogeneityInteractionProfileV2::P2MultiTwoWay => {
            ensure_interactions_are_current_two_stage_strong(model, false)?;
            if interactions.is_empty() || interactions.iter().any(|term| term.operands.len() != 2) {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "P2 requires one or more current two-way interactions".into(),
                ));
            }
        }
        HeterogeneityInteractionProfileV2::P23AllCurrent => {
            ensure_interactions_are_current_two_stage_strong(model, true)?;
            if interactions
                .iter()
                .filter(|term| term.operands.len() == 3)
                .count()
                != 1
            {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "P23 requires exactly one current three-way term and its hierarchy".into(),
                ));
            }
        }
        _ => {}
    }
    let (algorithms, candidate_k) = match &config.phase {
        crate::HeterogeneityPhaseV2::Discovery {
            algorithms,
            candidate_k,
        } => (algorithms.clone(), candidate_k.clone()),
        crate::HeterogeneityPhaseV2::Inference { lock } => (
            lock.discovery_algorithms.clone(),
            lock.discovery_candidate_k.clone(),
        ),
    };
    let plan = CompiledMultiModPlanV1::PlsHeterogeneityV2 {
        profile: config.profile,
        algorithms,
        candidate_k,
        interactions,
    };
    Ok((plan, sha256_serialized(config)))
}

fn relation_map(model: &SemModelV4) -> BTreeMap<&str, (&str, &str)> {
    model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::Structural {
                id, source, target, ..
            } => Some((id.as_str(), (source.as_str(), target.as_str()))),
            _ => None,
        })
        .collect()
}

fn validate_conditional_paths(
    model: &SemModelV4,
    paths: &[ConditionalProcessPathV2],
) -> Result<(), MultiModCompilationErrorV1> {
    let relations = relation_map(model);
    for path in paths {
        let mut previous_target: Option<&str> = None;
        let mut visited_variables = BTreeSet::new();
        for relation_id in &path.ordered_relation_ids {
            let Some((source, target)) = relations.get(relation_id.as_str()).copied() else {
                return Err(MultiModCompilationErrorV1::InvalidPath {
                    path_id: path.path_id.clone(),
                    reason: format!("unknown structural relation {relation_id}"),
                });
            };
            if previous_target.is_some_and(|previous| previous != source) {
                return Err(MultiModCompilationErrorV1::InvalidPath {
                    path_id: path.path_id.clone(),
                    reason: "relations do not form one contiguous directed path".into(),
                });
            }
            if previous_target.is_none() {
                visited_variables.insert(source);
            }
            if !visited_variables.insert(target) {
                return Err(MultiModCompilationErrorV1::InvalidPath {
                    path_id: path.path_id.clone(),
                    reason: "path revisits a variable and is not recursive-acyclic".into(),
                });
            }
            previous_target = Some(target);
        }
    }
    Ok(())
}

fn compile_conditional_plan(
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
    interactions: Vec<CompiledInteractionIdentityV1>,
    hocs: Vec<CompiledHocIdentityV1>,
) -> Result<(CompiledMultiModPlanV1, String), MultiModCompilationErrorV1> {
    config.ensure_valid()?;
    validate_conditional_paths(model, &config.paths)?;
    let declared = config
        .declared_interaction_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if declared
        != interactions
            .iter()
            .map(|interaction| interaction.term_id.as_str())
            .collect::<BTreeSet<_>>()
    {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "declared interaction inventory differs from SemModelV4".into(),
        ));
    }
    let selected_relations = config
        .paths
        .iter()
        .flat_map(|path| path.ordered_relation_ids.iter())
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if interactions
        .iter()
        .any(|interaction| !selected_relations.contains(interaction.focal_relation_id.as_str()))
    {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "every declared interaction must moderate an edge of an explicitly selected indirect path"
                .into(),
        ));
    }
    let compiled_moderators = interactions
        .iter()
        .flat_map(|interaction| interaction.operands.iter().skip(1))
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let requested_moderators = config
        .moderator_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if compiled_moderators != requested_moderators {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "declared moderator inventory differs from the interaction operands".into(),
        ));
    }
    let requested_hocs = config
        .hoc_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let compiled_hoc_ids = hocs
        .iter()
        .map(|hoc| hoc.term_id.as_str())
        .collect::<BTreeSet<_>>();
    if requested_hocs != compiled_hoc_ids {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "declared HOC inventory differs from SemModelV4".into(),
        ));
    }
    for probe in &config.probes {
        if probe.scale != ConditionalProbeScaleV2::RawObservedWithTransformationReceipt {
            continue;
        }
        let source_columns = probe
            .raw_transformation_receipt
            .iter()
            .map(|receipt| receipt.source_column.as_str())
            .chain(
                probe
                    .raw_fit_metric_receipts
                    .iter()
                    .map(|receipt| receipt.source_column.as_str()),
            )
            .collect::<BTreeSet<_>>();
        if source_columns.len() != 1 {
            return Err(MultiModCompilationErrorV1::ProfileMismatch(format!(
                "raw-unit moderator {} requires one authoritative observed source column",
                probe.moderator_id
            )));
        }
        let expected_source_column = *source_columns
            .iter()
            .next()
            .expect("validated singleton source inventory");
        let valid_raw_probe = model.variables.iter().any(|variable| {
            matches!(variable, SemVariableV4::Observed {
                id,
                source_column,
                scale: ObservedScaleV4::Continuous | ObservedScaleV4::Binary,
                ..
            } if id == &probe.moderator_id && source_column == expected_source_column)
        });
        if !valid_raw_probe {
            return Err(MultiModCompilationErrorV1::ProfileMismatch(format!(
                "raw-unit moderator {} must be directly observed and match the receipt source column",
                probe.moderator_id
            )));
        }
    }
    match config.profile {
        ConditionalProcessProfileV2::BoundedThreeWayPercentile => {
            ensure_interactions_are_current_two_stage_strong(model, true)?;
            if interactions
                .iter()
                .filter(|term| term.operands.len() == 3)
                .count()
                != 1
                || !hocs.is_empty()
            {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "three-way conditional process requires exactly one three-way term and no HOCs"
                        .into(),
                ));
            }
            if config.three_way_interaction_id.as_deref()
                != interactions
                    .iter()
                    .find(|term| term.operands.len() == 3)
                    .map(|term| term.term_id.as_str())
            {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "the selected three-way interaction identity differs from SemModelV4".into(),
                ));
            }
            ensure_weight_binding(model, None)?;
        }
        ConditionalProcessProfileV2::MultipleHocPercentile => {
            ensure_interactions_are_current_two_stage_strong(model, false)?;
            ensure_hocs_disjoint_and_homogeneous(model, &hocs)?;
            ensure_weight_binding(model, None)?;
        }
        ConditionalProcessProfileV2::GroupedPercentile => {
            ensure_interactions_are_current_two_stage_strong(model, false)?;
            let grouping = config.grouping_column.as_deref().ok_or_else(|| {
                MultiModCompilationErrorV1::ProfileMismatch("grouping column missing".into())
            })?;
            ensure_grouping_not_indicator(model, grouping)?;
            ensure_group_declaration(model, grouping)?;
            if !hocs.is_empty() {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "grouped conditional process cannot contain HOCs".into(),
                ));
            }
            ensure_weight_binding(model, None)?;
        }
        ConditionalProcessProfileV2::CaseWeightedPercentile
        | ConditionalProcessProfileV2::FrequencyWeightedPercentile => {
            ensure_interactions_are_current_two_stage_strong(model, false)?;
            if !matches!(model.group, SemGroupV4::SingleGroup) || !hocs.is_empty() {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "weighted conditional process requires one group and no HOCs".into(),
                ));
            }
            ensure_weight_binding(model, config.weight.as_ref())?;
        }
        _ => {
            ensure_interactions_are_current_two_stage_strong(model, false)?;
            if !matches!(model.group, SemGroupV4::SingleGroup) || !hocs.is_empty() {
                return Err(MultiModCompilationErrorV1::ProfileMismatch(
                    "this conditional-process profile requires one unweighted group and no HOCs"
                        .into(),
                ));
            }
            ensure_weight_binding(model, None)?;
        }
    }
    let points = if config.explicit_joint_tuples.is_empty() {
        config
            .probes
            .iter()
            .map(|probe| probe.values.len())
            .product::<usize>()
    } else {
        config.explicit_joint_tuples.len()
    };
    let groups = config.groups.len().max(1);
    let compiled_target_upper_bound =
        conditional_process_target_upper_bound_v2(config, points, groups);
    let plan = CompiledMultiModPlanV1::GeneralSemConditionalProcessV2 {
        profile: config.profile,
        paths: config.paths.clone(),
        interactions,
        hocs,
        compiled_target_upper_bound,
    };
    Ok((plan, sha256_serialized(config)))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResolvedObservedCausalRoleV1 {
    pub variable_id: String,
    pub source_column: String,
    pub scale: ObservedScaleV4,
}

/// Resolves one observed causal reference through the same exact typed rule
/// used by compilation and raw-data preparation. Public causal configurations
/// may consistently identify an observed variable by its SEM id or its bound
/// source-column alias; an ambiguous id/alias collision fails closed.
pub fn resolve_observed_causal_role_v1(
    model: &SemModelV4,
    identity: &str,
) -> Option<ResolvedObservedCausalRoleV1> {
    let matches = model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id,
                source_column,
                scale,
                ..
            } if id == identity || source_column == identity => {
                Some(ResolvedObservedCausalRoleV1 {
                    variable_id: id.clone(),
                    source_column: source_column.clone(),
                    scale: *scale,
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    (matches.len() == 1 && !matches[0].source_column.trim().is_empty()).then(|| matches[0].clone())
}

/// Canonicalizes every observed causal reference before any overlap,
/// membership, path, equation, or positivity validation. This closes the
/// otherwise dangerous gap where an SEM id and its source-column alias could
/// appear to be two different causal roles.
pub fn canonicalize_interventional_causal_config_v1(
    model: &SemModelV4,
    config: &InterventionalCausalMediationConfigV1,
) -> Result<InterventionalCausalMediationConfigV1, MultiModCompilationErrorV1> {
    let canonical_id = |identity: &str| {
        resolve_observed_causal_role_v1(model, identity)
            .map(|binding| binding.variable_id)
            .ok_or_else(|| MultiModCompilationErrorV1::CausalRoleNotObserved(identity.to_owned()))
    };
    let mut canonical = config.clone();
    canonical.treatment = canonical_id(&config.treatment)?;
    canonical.outcome = canonical_id(&config.outcome)?;
    canonical.mediators = config
        .mediators
        .iter()
        .map(|identity| canonical_id(identity))
        .collect::<Result<Vec<_>, _>>()?;
    canonical.baseline_moderators = config
        .baseline_moderators
        .iter()
        .map(|identity| canonical_id(identity))
        .collect::<Result<Vec<_>, _>>()?;
    canonical.adjustment_covariates = config
        .adjustment_covariates
        .iter()
        .map(|identity| canonical_id(identity))
        .collect::<Result<Vec<_>, _>>()?;
    canonical.positivity_policy.positivity_strata_variable_ids = config
        .positivity_policy
        .positivity_strata_variable_ids
        .iter()
        .map(|identity| canonical_id(identity))
        .collect::<Result<Vec<_>, _>>()?;
    for (canonical_path, authored_path) in canonical.paths.iter_mut().zip(&config.paths) {
        canonical_path.ordered_variable_ids = authored_path
            .ordered_variable_ids
            .iter()
            .map(|identity| canonical_id(identity))
            .collect::<Result<Vec<_>, _>>()?;
        for (canonical_equation, authored_equation) in canonical_path
            .equations
            .iter_mut()
            .zip(&authored_path.equations)
        {
            canonical_equation.outcome_variable_id =
                canonical_id(&authored_equation.outcome_variable_id)?;
            for (canonical_term, authored_term) in canonical_equation
                .terms
                .iter_mut()
                .zip(&authored_equation.terms)
            {
                canonical_term.factor_variable_ids = authored_term
                    .factor_variable_ids
                    .iter()
                    .map(|identity| canonical_id(identity))
                    .collect::<Result<Vec<_>, _>>()?;
            }
        }
    }
    Ok(canonical)
}

fn compile_causal_plan(
    model: &SemModelV4,
    config: &InterventionalCausalMediationConfigV1,
    interactions: Vec<CompiledInteractionIdentityV1>,
    hocs: Vec<CompiledHocIdentityV1>,
    authored_config_sha256: String,
) -> Result<(CompiledMultiModPlanV1, String), MultiModCompilationErrorV1> {
    config.ensure_valid()?;
    if !matches!(model.group, SemGroupV4::SingleGroup)
        || !interactions.is_empty()
        || !hocs.is_empty()
        || model
            .derived_terms
            .iter()
            .any(|term| matches!(term, SemDerivedTermV4::Polynomial { .. }))
        || model
            .variables
            .iter()
            .any(|variable| !matches!(variable, SemVariableV4::Observed { .. }))
    {
        return Err(MultiModCompilationErrorV1::CausalModelScope);
    }
    ensure_weight_binding(model, None)?;
    let referenced_identities = std::iter::once(config.treatment.as_str())
        .chain(std::iter::once(config.outcome.as_str()))
        .chain(config.mediators.iter().map(String::as_str))
        .chain(config.baseline_moderators.iter().map(String::as_str))
        .chain(config.adjustment_covariates.iter().map(String::as_str))
        .chain(
            config
                .positivity_policy
                .positivity_strata_variable_ids
                .iter()
                .map(String::as_str),
        )
        .chain(
            config
                .paths
                .iter()
                .flat_map(|path| path.ordered_variable_ids.iter().map(String::as_str)),
        )
        .chain(config.paths.iter().flat_map(|path| {
            path.equations.iter().flat_map(|equation| {
                std::iter::once(equation.outcome_variable_id.as_str()).chain(
                    equation
                        .terms
                        .iter()
                        .flat_map(|term| term.factor_variable_ids.iter().map(String::as_str)),
                )
            })
        }))
        .collect::<BTreeSet<_>>();
    let mut resolved = BTreeMap::new();
    for identity in referenced_identities {
        let binding = resolve_observed_causal_role_v1(model, identity).ok_or_else(|| {
            MultiModCompilationErrorV1::CausalRoleNotObserved(identity.to_owned())
        })?;
        resolved.insert(identity.to_owned(), binding);
    }
    if config
        .positivity_policy
        .positivity_strata_variable_ids
        .iter()
        .any(|identity| resolved[identity].scale != ObservedScaleV4::Binary)
    {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "positivity strata must be directly observed binary baseline variables".into(),
        ));
    }
    let treatment_scale = Some(resolved[&config.treatment].scale);
    let treatment_scale_valid = match config.treatment_contrast {
        ObservedTreatmentContrastV1::Binary { .. } => {
            treatment_scale == Some(ObservedScaleV4::Binary)
        }
        ObservedTreatmentContrastV1::Continuous { .. } => {
            treatment_scale == Some(ObservedScaleV4::Continuous)
        }
    };
    if !treatment_scale_valid {
        return Err(MultiModCompilationErrorV1::ProfileMismatch(
            "treatment contrast kind differs from the observed treatment scale".into(),
        ));
    }
    for identity in
        std::iter::once(config.outcome.as_str()).chain(config.mediators.iter().map(String::as_str))
    {
        if resolved[identity].scale != ObservedScaleV4::Continuous {
            return Err(MultiModCompilationErrorV1::ProfileMismatch(format!(
                "causal outcome and mediator {identity} must be observed continuous"
            )));
        }
    }
    for identity in config
        .baseline_moderators
        .iter()
        .chain(config.adjustment_covariates.iter())
    {
        if !matches!(
            resolved[identity].scale,
            ObservedScaleV4::Continuous | ObservedScaleV4::Binary
        ) {
            return Err(MultiModCompilationErrorV1::ProfileMismatch(format!(
                "causal moderator/covariate {identity} must be observed numeric or binary"
            )));
        }
    }
    let structural_pairs = model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::Structural { source, target, .. } => {
                Some((source.as_str(), target.as_str()))
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    for path in &config.paths {
        if path.ordered_variable_ids.windows(2).any(|edge| {
            !structural_pairs.contains(&(
                resolved[&edge[0]].variable_id.as_str(),
                resolved[&edge[1]].variable_id.as_str(),
            ))
        }) {
            return Err(MultiModCompilationErrorV1::InvalidPath {
                path_id: path.path_id.clone(),
                reason: "a selected causal edge is absent from SemModelV4".into(),
            });
        }
    }
    let plan = CompiledMultiModPlanV1::InterventionalCausalMediationV1 {
        treatment: resolved[&config.treatment].variable_id.clone(),
        outcome: resolved[&config.outcome].variable_id.clone(),
        mediator_ids: config
            .mediators
            .iter()
            .map(|identity| resolved[identity].variable_id.clone())
            .collect(),
        path_ids: config
            .paths
            .iter()
            .map(|path| path.path_id.clone())
            .collect(),
    };
    Ok((plan, authored_config_sha256))
}

#[cfg(test)]
mod causal_role_resolution_tests {
    use super::*;

    fn reviewed_identification() -> crate::CausalIdentificationChecklistV1 {
        crate::CausalIdentificationChecklistV1 {
            temporal_order_declared: true,
            adjustment_set_justified: true,
            consistency_assumption_acknowledged: true,
            no_unmeasured_treatment_outcome_confounding_acknowledged: true,
            no_unmeasured_treatment_mediator_confounding_acknowledged: true,
            no_unmeasured_mediator_outcome_confounding_acknowledged: true,
            no_exposure_induced_mediator_outcome_confounder_confirmed: true,
            no_recanting_witness_confirmed: true,
            linear_model_specification_reviewed: true,
            positivity_reviewed: true,
        }
    }

    fn observed(id: &str, source_column: &str, scale: ObservedScaleV4) -> SemVariableV4 {
        SemVariableV4::Observed {
            id: id.into(),
            label: id.into(),
            source_column: source_column.into(),
            scale,
            role: crate::ObservedRoleV4::Structural,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        }
    }

    fn resolver_model(variables: Vec<SemVariableV4>) -> SemModelV4 {
        SemModelV4 {
            schema_version: crate::SEM_MODEL_V4_SCHEMA_VERSION,
            id: "causal-alias-model".into(),
            name: "causal alias model".into(),
            variables,
            relations: Vec::new(),
            parameters: Vec::new(),
            constraints: Vec::new(),
            derived_terms: Vec::new(),
            group: SemGroupV4::SingleGroup,
            data_binding: SemDataBindingV4::Raw {
                dataset_id: "dataset".into(),
                missing_data: MissingDataPolicyV4::ListwiseDeletion,
                weight: None,
                cluster_variable: None,
                strata_variable: None,
            },
            annotations: Vec::new(),
            presentation: crate::SemPresentationV4::default(),
        }
    }

    fn term(term_id: &str, factors: &[&str]) -> crate::CausalLinearTermV1 {
        crate::CausalLinearTermV1 {
            term_id: term_id.into(),
            factor_variable_ids: factors.iter().map(|factor| (*factor).into()).collect(),
        }
    }

    fn canonical_config() -> InterventionalCausalMediationConfigV1 {
        InterventionalCausalMediationConfigV1 {
            schema_version: 1,
            treatment: "x_id".into(),
            treatment_contrast: ObservedTreatmentContrastV1::Binary {
                control: 0.0,
                treated: 1.0,
            },
            outcome: "y_id".into(),
            mediators: vec!["m_id".into()],
            baseline_moderators: vec!["z_id".into()],
            adjustment_covariates: vec!["c_id".into()],
            paths: vec![crate::ObservedCausalPathV1 {
                path_id: "x-m-y".into(),
                ordered_variable_ids: vec!["x_id".into(), "m_id".into(), "y_id".into()],
                equations: vec![
                    crate::CausalLinearEquationV1 {
                        equation_id: "m-equation".into(),
                        outcome_variable_id: "m_id".into(),
                        terms: vec![
                            term("x", &["x_id"]),
                            term("z", &["z_id"]),
                            term("c", &["c_id"]),
                        ],
                    },
                    crate::CausalLinearEquationV1 {
                        equation_id: "y-equation".into(),
                        outcome_variable_id: "y_id".into(),
                        terms: vec![
                            term("x", &["x_id"]),
                            term("m", &["m_id"]),
                            term("z", &["z_id"]),
                            term("c", &["c_id"]),
                            term("x-by-z", &["x_id", "z_id"]),
                        ],
                    },
                ],
            }],
            positivity_policy: crate::CausalPositivityPolicyV1 {
                positivity_strata_variable_ids: vec!["z_id".into()],
                ..crate::CausalPositivityPolicyV1::default()
            },
            identification: reviewed_identification(),
            bootstrap_resamples: 500,
            seed: 42,
            confidence_level: 0.95,
        }
    }

    fn causal_model() -> SemModelV4 {
        resolver_model(vec![
            observed("x_id", "x_col", ObservedScaleV4::Binary),
            observed("m_id", "m_col", ObservedScaleV4::Continuous),
            observed("y_id", "y_col", ObservedScaleV4::Continuous),
            observed("z_id", "z_col", ObservedScaleV4::Binary),
            observed("c_id", "c_col", ObservedScaleV4::Continuous),
        ])
    }

    fn alias_every_reference(config: &mut InterventionalCausalMediationConfigV1) {
        let aliases = BTreeMap::from([
            ("x_id", "x_col"),
            ("m_id", "m_col"),
            ("y_id", "y_col"),
            ("z_id", "z_col"),
            ("c_id", "c_col"),
        ]);
        let replace = |identity: &mut String| {
            if let Some(alias) = aliases.get(identity.as_str()) {
                *identity = (*alias).into();
            }
        };
        replace(&mut config.treatment);
        replace(&mut config.outcome);
        for identity in &mut config.mediators {
            replace(identity);
        }
        for identity in &mut config.baseline_moderators {
            replace(identity);
        }
        for identity in &mut config.adjustment_covariates {
            replace(identity);
        }
        for identity in &mut config.positivity_policy.positivity_strata_variable_ids {
            replace(identity);
        }
        for path in &mut config.paths {
            for identity in &mut path.ordered_variable_ids {
                replace(identity);
            }
            for equation in &mut path.equations {
                replace(&mut equation.outcome_variable_id);
                for term in &mut equation.terms {
                    for identity in &mut term.factor_variable_ids {
                        replace(identity);
                    }
                }
            }
        }
    }

    #[test]
    fn causal_role_resolver_maps_id_and_source_alias_to_one_typed_binding() {
        let model = resolver_model(vec![observed(
            "treatment_id",
            "treatment_column",
            ObservedScaleV4::Binary,
        )]);
        let by_id = resolve_observed_causal_role_v1(&model, "treatment_id").unwrap();
        let by_alias = resolve_observed_causal_role_v1(&model, "treatment_column").unwrap();
        assert_eq!(by_id, by_alias);
        assert_eq!(by_alias.variable_id, "treatment_id");
        assert_eq!(by_alias.source_column, "treatment_column");
        assert_eq!(by_alias.scale, ObservedScaleV4::Binary);
    }

    #[test]
    fn causal_role_resolver_rejects_id_source_alias_collisions() {
        let model = resolver_model(vec![
            observed("first", "shared", ObservedScaleV4::Continuous),
            observed("shared", "second", ObservedScaleV4::Continuous),
        ]);
        assert!(resolve_observed_causal_role_v1(&model, "shared").is_none());
    }

    #[test]
    fn causal_canonicalization_equates_all_id_all_alias_and_mixed_references() {
        let model = causal_model();
        let expected = canonical_config();
        expected.ensure_valid().unwrap();

        let mut aliases = expected.clone();
        alias_every_reference(&mut aliases);
        let mut mixed = aliases.clone();
        mixed.treatment = "x_id".into();
        mixed.mediators[0] = "m_id".into();
        mixed.adjustment_covariates[0] = "c_id".into();
        mixed.paths[0].ordered_variable_ids[2] = "y_id".into();
        mixed.paths[0].equations[0].outcome_variable_id = "m_id".into();
        mixed.paths[0].equations[1].terms[4].factor_variable_ids[0] = "x_id".into();
        mixed.positivity_policy.positivity_strata_variable_ids[0] = "z_id".into();

        for authored in [&expected, &aliases, &mixed] {
            let canonical = canonicalize_interventional_causal_config_v1(&model, authored).unwrap();
            canonical.ensure_valid().unwrap();
            assert_eq!(canonical, expected);
        }
    }

    #[test]
    fn causal_canonicalization_exposes_id_alias_role_overlap() {
        let model = causal_model();
        let mut config = canonical_config();
        config.baseline_moderators = vec!["c_col".into()];
        let canonical = canonicalize_interventional_causal_config_v1(&model, &config).unwrap();
        assert_eq!(
            canonical.ensure_valid().unwrap_err().code,
            "interventional_causal_mediation_v1.role_overlap"
        );
    }

    #[test]
    fn causal_canonicalization_rejects_ambiguous_id_source_collision() {
        let mut model = causal_model();
        model
            .variables
            .push(observed("x_col", "other_x_col", ObservedScaleV4::Binary));
        let mut aliases = canonical_config();
        alias_every_reference(&mut aliases);
        assert!(matches!(
            canonicalize_interventional_causal_config_v1(&model, &aliases),
            Err(MultiModCompilationErrorV1::CausalRoleNotObserved(identity))
                if identity == "x_col"
        ));
    }
}
