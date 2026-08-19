use crate::{
    RecipeV4PlsExecutionError, RecipeV4PlsExecutionResultV1, RunnerProgress,
    project_pls_plan_to_current_recipe, run_compiled_pls_recipe_v4,
};
use qpls_core::{
    AnalysisRecipeV4, CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION,
    CanonicalAggregateEffectKindV1, CanonicalAggregateEffectResultV1,
    CanonicalConditionalEffectProbeResultV1, CanonicalConditionalEffectResultV1,
    CanonicalConditionalProbeValuesResultV1, CanonicalGeneralSemBootstrapIntervalV1,
    CanonicalGeneralSemEffectIdentityV1, CanonicalGeneralSemEstimateV1,
    CanonicalGeneralSemFailedReplicateReasonV1, CanonicalGeneralSemFailedReplicateV1,
    CanonicalGeneralSemInferenceKindV1, CanonicalGeneralSemInferenceReceiptV1,
    CanonicalGeneralSemInferenceTailV1, CanonicalGeneralSemResultTraceV1,
    CanonicalGeneralSemResultsV1, CanonicalInteractionConstructionMethodV1,
    CanonicalInteractionEffectResultV1, CanonicalInteractionHierarchyPolicyV1,
    CanonicalInteractionPlotPointV1, CanonicalInteractionPlotResultV1,
    CanonicalInteractionPlotSeriesV1, CanonicalSpecificIndirectEffectResultV1,
    CapabilityCellReferenceV2, CompiledGeneralSemPlsRecipeV1, CompiledPlsEffectEstimandV3,
    ExecutionRecipeError, GeneralSemBootstrapIntervalV1, GeneralSemEffectsV1,
    GeneralSemEffectsV1Error, GeneralSemInferenceTailV1, GeneralSemInferenceV1,
    GeneralSemPlsRecipeCompilationErrorV1, MethodConfig, SemModelV4, StructuralRelationRoleV4,
    ValidatedExecutionRecipe, decompose_general_sem_effects_v1,
    general_sem_effect_identity_set_sha256_v1, pls_general_bootstrap_capability_cell_v1,
    project_general_sem_pls_stage_one_recipe_v1, validate_compiled_general_sem_pls_recipe_v1,
};
use qpls_data::Dataset;
use qpls_estimation::{
    GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1, GeneralSemPlsInteractionPointErrorV1,
    GeneralSemPlsMultipleInteractionPointResultV1,
    estimate_general_sem_pls_multiple_two_way_interactions_v1,
};
use qpls_resampling::{
    GeneralSemPlsBootstrapEffectInferenceV1, GeneralSemPlsBootstrapErrorV1,
    GeneralSemPlsBootstrapResultV1, bootstrap_general_sem_pls_v1,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const RECIPE_V4_GENERAL_SEM_PLS_EXECUTION_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub const RECIPE_V4_GENERAL_SEM_PLS_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_general_sem_pls_recipe_v1_point_execution_v1";
pub const RECIPE_V4_GENERAL_SEM_PLS_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_general_sem_pls_recipe_v1_percentile_bootstrap_execution_v1";
pub const RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_point_execution_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemRequestedEffectEstimateV1 {
    SpecificIndirect {
        estimand_id: String,
        path_identity: String,
        source_id: String,
        target_id: String,
        ordered_relation_ids: Vec<String>,
        coefficient: f64,
    },
    TotalIndirect {
        estimand_id: String,
        source_id: String,
        target_id: String,
        contributing_path_identities: Vec<String>,
        coefficient: f64,
    },
    TotalEffect {
        estimand_id: String,
        source_id: String,
        target_id: String,
        direct_relation_ids: Vec<String>,
        contributing_indirect_path_identities: Vec<String>,
        coefficient: f64,
    },
}

impl GeneralSemRequestedEffectEstimateV1 {
    pub fn estimand_id(&self) -> &str {
        match self {
            Self::SpecificIndirect { estimand_id, .. }
            | Self::TotalIndirect { estimand_id, .. }
            | Self::TotalEffect { estimand_id, .. } => estimand_id,
        }
    }

    pub fn coefficient(&self) -> f64 {
        match self {
            Self::SpecificIndirect { coefficient, .. }
            | Self::TotalIndirect { coefficient, .. }
            | Self::TotalEffect { coefficient, .. } => *coefficient,
        }
    }

    pub fn canonical_effect_id(&self) -> &str {
        match self {
            Self::SpecificIndirect { path_identity, .. } => path_identity,
            Self::TotalIndirect { estimand_id, .. } | Self::TotalEffect { estimand_id, .. } => {
                estimand_id
            }
        }
    }
}

fn requested_effect_identities_v1(
    effects: &[GeneralSemRequestedEffectEstimateV1],
) -> Vec<CanonicalGeneralSemEffectIdentityV1> {
    let mut identities = effects
        .iter()
        .map(|effect| match effect {
            GeneralSemRequestedEffectEstimateV1::SpecificIndirect {
                estimand_id,
                path_identity,
                source_id,
                target_id,
                ordered_relation_ids,
                ..
            } => CanonicalGeneralSemEffectIdentityV1::SpecificIndirect {
                effect_id: path_identity.clone(),
                estimand_id: estimand_id.clone(),
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                ordered_relation_ids: ordered_relation_ids.clone(),
            },
            GeneralSemRequestedEffectEstimateV1::TotalIndirect {
                estimand_id,
                source_id,
                target_id,
                contributing_path_identities,
                ..
            } => CanonicalGeneralSemEffectIdentityV1::TotalIndirect {
                effect_id: estimand_id.clone(),
                estimand_id: estimand_id.clone(),
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                contributing_path_identities: contributing_path_identities.clone(),
            },
            GeneralSemRequestedEffectEstimateV1::TotalEffect {
                estimand_id,
                source_id,
                target_id,
                direct_relation_ids,
                contributing_indirect_path_identities,
                ..
            } => CanonicalGeneralSemEffectIdentityV1::TotalEffect {
                effect_id: estimand_id.clone(),
                estimand_id: estimand_id.clone(),
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                direct_relation_ids: direct_relation_ids.clone(),
                contributing_path_identities: contributing_indirect_path_identities.clone(),
            },
        })
        .collect::<Vec<_>>();
    identities.sort_by(|left, right| left.effect_id().cmp(right.effect_id()));
    identities
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecipeV4GeneralSemPlsExecutionResultV1 {
    schema_version: u32,
    adapter_version: String,
    capability_cell: CapabilityCellReferenceV2,
    compilation_artifact_identity_sha256: String,
    compiled_plan_sha256: String,
    recipe_analytical_sha256: String,
    model_scientific_sha256: String,
    stage_one_model_scientific_sha256: String,
    source_dataset_fingerprint: String,
    general_sem_config_sha256: String,
    point_estimation: RecipeV4PlsExecutionResultV1,
    requested_effects: Vec<GeneralSemRequestedEffectEstimateV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interaction_point_estimation: Option<GeneralSemPlsMultipleInteractionPointResultV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bootstrap_inference: Option<GeneralSemPlsBootstrapResultV1>,
}

impl RecipeV4GeneralSemPlsExecutionResultV1 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn adapter_version(&self) -> &str {
        &self.adapter_version
    }

    pub fn capability_cell(&self) -> &CapabilityCellReferenceV2 {
        &self.capability_cell
    }

    pub fn compilation_artifact_identity_sha256(&self) -> &str {
        &self.compilation_artifact_identity_sha256
    }

    pub fn general_sem_config_sha256(&self) -> &str {
        &self.general_sem_config_sha256
    }

    pub fn compiled_plan_sha256(&self) -> &str {
        &self.compiled_plan_sha256
    }

    pub fn recipe_analytical_sha256(&self) -> &str {
        &self.recipe_analytical_sha256
    }

    pub fn model_scientific_sha256(&self) -> &str {
        &self.model_scientific_sha256
    }

    pub fn stage_one_model_scientific_sha256(&self) -> &str {
        &self.stage_one_model_scientific_sha256
    }

    pub fn source_dataset_fingerprint(&self) -> &str {
        &self.source_dataset_fingerprint
    }

    pub fn point_estimation(&self) -> &RecipeV4PlsExecutionResultV1 {
        &self.point_estimation
    }

    pub fn requested_effects(&self) -> &[GeneralSemRequestedEffectEstimateV1] {
        &self.requested_effects
    }

    pub fn interaction_point_estimation(
        &self,
    ) -> Option<&GeneralSemPlsMultipleInteractionPointResultV1> {
        self.interaction_point_estimation.as_ref()
    }

    pub fn bootstrap_inference(&self) -> Option<&GeneralSemPlsBootstrapResultV1> {
        self.bootstrap_inference.as_ref()
    }

    /// Produces the typed additive section consumed by
    /// `CanonicalResultDocumentV2`. Point-only executions preserve the legacy
    /// omission shape; inferred executions require an exact bootstrap receipt.
    pub fn canonical_general_sem_results_v1(
        &self,
    ) -> Result<CanonicalGeneralSemResultsV1, RecipeV4GeneralSemPlsExecutionErrorV1> {
        let point_receipt = self.point_estimation.provenance().compilation_receipt();
        if point_receipt.model_scientific_sha256() != self.stage_one_model_scientific_sha256
            || point_receipt.dataset_fingerprint() != self.source_dataset_fingerprint
        {
            return Err(
                RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                    "point execution provenance differs from the General SEM authority".to_string(),
                ),
            );
        }
        let trace = CanonicalGeneralSemResultTraceV1 {
            model_id: self
                .point_estimation
                .provenance()
                .compilation_receipt()
                .model_id()
                .into(),
            capability_cell: self.capability_cell.clone(),
        };
        let inference_by_id = validate_bootstrap_inference_binding(self)?;
        let mut specific_indirect_effects = Vec::new();
        let mut aggregate_effects = Vec::new();
        for effect in &self.requested_effects {
            match effect {
                GeneralSemRequestedEffectEstimateV1::SpecificIndirect {
                    estimand_id,
                    path_identity,
                    source_id,
                    target_id,
                    ordered_relation_ids,
                    coefficient,
                    ..
                } => specific_indirect_effects.push(CanonicalSpecificIndirectEffectResultV1 {
                    effect_id: path_identity.clone(),
                    estimand_id: estimand_id.clone(),
                    trace: trace.clone(),
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    ordered_relation_ids: ordered_relation_ids.clone(),
                    value: canonical_estimate(
                        *coefficient,
                        inference_by_id.get(path_identity.as_str()).copied(),
                    ),
                }),
                GeneralSemRequestedEffectEstimateV1::TotalIndirect {
                    estimand_id,
                    source_id,
                    target_id,
                    contributing_path_identities,
                    coefficient,
                } => aggregate_effects.push(CanonicalAggregateEffectResultV1 {
                    effect_id: estimand_id.clone(),
                    estimand_id: estimand_id.clone(),
                    trace: trace.clone(),
                    kind: CanonicalAggregateEffectKindV1::TotalIndirect,
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    direct_relation_ids: Vec::new(),
                    contributing_path_identities: contributing_path_identities.clone(),
                    value: canonical_estimate(
                        *coefficient,
                        inference_by_id.get(estimand_id.as_str()).copied(),
                    ),
                }),
                GeneralSemRequestedEffectEstimateV1::TotalEffect {
                    estimand_id,
                    source_id,
                    target_id,
                    direct_relation_ids,
                    contributing_indirect_path_identities,
                    coefficient,
                } => aggregate_effects.push(CanonicalAggregateEffectResultV1 {
                    effect_id: estimand_id.clone(),
                    estimand_id: estimand_id.clone(),
                    trace: trace.clone(),
                    kind: CanonicalAggregateEffectKindV1::TotalEffect,
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    direct_relation_ids: direct_relation_ids.clone(),
                    contributing_path_identities: contributing_indirect_path_identities.clone(),
                    value: canonical_estimate(
                        *coefficient,
                        inference_by_id.get(estimand_id.as_str()).copied(),
                    ),
                }),
            }
        }
        specific_indirect_effects.sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
        aggregate_effects.sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
        let (
            interaction_effects,
            conditional_effect_probes,
            conditional_effects,
            interaction_plots,
        ) = canonical_interaction_sections_v1(self, &trace)?;
        Ok(CanonicalGeneralSemResultsV1 {
            schema_version: CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION,
            inference_receipt: self
                .bootstrap_inference
                .as_ref()
                .map(|bootstrap| canonical_inference_receipt(self, bootstrap))
                .transpose()?,
            specific_indirect_effects,
            aggregate_effects,
            joint_stage_structural_coefficients: Vec::new(),
            interaction_effects,
            conditional_effect_probes,
            conditional_effects,
            interaction_plots,
            higher_order_stages: Vec::new(),
            cbsem_fit: Vec::new(),
            identification_diagnostics: Vec::new(),
        })
    }
}

fn canonical_estimate(
    estimate: f64,
    inference: Option<&GeneralSemPlsBootstrapEffectInferenceV1>,
) -> CanonicalGeneralSemEstimateV1 {
    let Some(inference) = inference else {
        return CanonicalGeneralSemEstimateV1 {
            estimate,
            bootstrap_mean: None,
            bootstrap_bias: None,
            standard_error: None,
            lower: None,
            upper: None,
            p_value: None,
            bootstrap_usable_replicates: None,
            bootstrap_two_sided_exceedances: None,
        };
    };
    CanonicalGeneralSemEstimateV1 {
        estimate,
        bootstrap_mean: Some(inference.bootstrap_mean),
        bootstrap_bias: Some(inference.bootstrap_bias),
        standard_error: Some(inference.standard_error),
        lower: Some(inference.lower),
        upper: Some(inference.upper),
        p_value: Some(inference.p_value_two_sided),
        bootstrap_usable_replicates: Some(inference.usable_replicates),
        bootstrap_two_sided_exceedances: Some(inference.two_sided_exceedances),
    }
}

type CanonicalInteractionSectionsV1 = (
    Vec<CanonicalInteractionEffectResultV1>,
    Vec<CanonicalConditionalEffectProbeResultV1>,
    Vec<CanonicalConditionalEffectResultV1>,
    Vec<CanonicalInteractionPlotResultV1>,
);

fn canonical_interaction_sections_v1(
    result: &RecipeV4GeneralSemPlsExecutionResultV1,
    trace: &CanonicalGeneralSemResultTraceV1,
) -> Result<CanonicalInteractionSectionsV1, RecipeV4GeneralSemPlsExecutionErrorV1> {
    let Some(interactions) = &result.interaction_point_estimation else {
        if result.stage_one_model_scientific_sha256 != result.model_scientific_sha256 {
            return Err(
                RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                    "a projected stage-one model requires a typed interaction point payload".into(),
                ),
            );
        }
        return Ok((Vec::new(), Vec::new(), Vec::new(), Vec::new()));
    };
    interactions.ensure_self_consistent_v1()?;
    if result.capability_cell
        != qpls_core::pls_general_multiple_moderation_point_capability_cell_v1()
        || result.bootstrap_inference.is_some()
        || !result.requested_effects.is_empty()
        || result.stage_one_model_scientific_sha256 == result.model_scientific_sha256
    {
        return Err(
            RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                "interaction point payload contradicts its capability, source projection, or point-only scope"
                    .into(),
            ),
        );
    }

    let receipts = interactions
        .product_scale_receipts()
        .iter()
        .map(|receipt| (receipt.interaction_id(), receipt))
        .collect::<BTreeMap<_, _>>();
    let mut interaction_effects = Vec::new();
    let mut probes = Vec::new();
    let mut conditional_effects = Vec::new();
    let mut plots = Vec::new();
    for coefficient in interactions.interaction_coefficients() {
        let receipt = receipts.get(coefficient.interaction_id()).ok_or_else(|| {
            GeneralSemPlsInteractionPointErrorV1::InvalidResultContract(format!(
                "interaction {} has no canonical product-scale receipt",
                coefficient.interaction_id()
            ))
        })?;
        let interaction_effect_id = coefficient.interaction_effect_relation_id().to_string();
        interaction_effects.push(CanonicalInteractionEffectResultV1 {
            effect_id: interaction_effect_id.clone(),
            trace: trace.clone(),
            interaction_id: coefficient.interaction_id().to_string(),
            focal_relation_id: coefficient.focal_relation_id().to_string(),
            interaction_effect_relation_id: coefficient
                .interaction_effect_relation_id()
                .to_string(),
            interaction_effect_parameter_id: coefficient
                .interaction_effect_parameter_id()
                .to_string(),
            focal_predictor_id: coefficient.focal_predictor_id().to_string(),
            moderator_id: coefficient.moderator_id().to_string(),
            outcome_id: coefficient.outcome_id().to_string(),
            generated_product_column_id: receipt.generated_product_column_id().to_string(),
            stage_one_model_scientific_sha256: result.stage_one_model_scientific_sha256.clone(),
            method_version: interactions.method_version().to_string(),
            construction_method: CanonicalInteractionConstructionMethodV1::TwoStage,
            product_scale_version: receipt.scale_version().to_string(),
            hierarchy_policy: CanonicalInteractionHierarchyPolicyV1::Strong,
            hierarchy_policy_version: coefficient.hierarchy_policy_version().to_string(),
            conditioning_policy_version: GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1.to_string(),
            observation_count: u32::try_from(interactions.observation_count()).map_err(|_| {
                GeneralSemPlsInteractionPointErrorV1::InvalidResultContract(
                    "interaction observation count does not fit the canonical u32 wire".into(),
                )
            })?,
            unstandardized_product_mean: receipt.unstandardized_product_mean(),
            unstandardized_product_sample_standard_deviation: receipt
                .unstandardized_product_sample_standard_deviation(),
            standardized_product_coefficient: canonical_estimate(
                coefficient.standardized_product_estimate(),
                None,
            ),
            scientific_rescaled_gamma: canonical_estimate(coefficient.raw_product_estimate(), None),
        });

        let probe_id = format!(
            "probe:{}:standardized_minus1_zero_plus1",
            coefficient.interaction_id()
        );
        probes.push(CanonicalConditionalEffectProbeResultV1 {
            probe_id: probe_id.clone(),
            trace: trace.clone(),
            moderator_id: coefficient.moderator_id().to_string(),
            values: CanonicalConditionalProbeValuesResultV1::Explicit {
                values: vec![-1.0, 0.0, 1.0],
            },
        });
        let slopes = interactions
            .simple_slopes()
            .iter()
            .filter(|slope| slope.interaction_id() == coefficient.interaction_id())
            .collect::<Vec<_>>();
        if slopes.len() != 3 {
            return Err(
                GeneralSemPlsInteractionPointErrorV1::InvalidResultContract(format!(
                    "interaction {} does not have exactly three canonical simple slopes",
                    coefficient.interaction_id()
                ))
                .into(),
            );
        }
        let estimand_id = format!("conditional_slope:{}", coefficient.interaction_id());
        let mut series = Vec::new();
        for (probe_value_index, slope) in slopes.into_iter().enumerate() {
            let probe_value_index = u32::try_from(probe_value_index).expect("three probes fit u32");
            conditional_effects.push(CanonicalConditionalEffectResultV1 {
                effect_id: format!(
                    "conditional:{}:{}",
                    coefficient.interaction_id(),
                    probe_value_index
                ),
                estimand_id: estimand_id.clone(),
                trace: trace.clone(),
                interaction_id: coefficient.interaction_id().to_string(),
                interaction_effect_id: Some(interaction_effect_id.clone()),
                focal_relation_id: coefficient.focal_relation_id().to_string(),
                probe_id: probe_id.clone(),
                moderator_id: coefficient.moderator_id().to_string(),
                probe_value_index,
                moderator_value: slope.moderator_value_standardized(),
                value: canonical_estimate(slope.estimate(), None),
            });
            let points = [-1.0_f64, 0.0, 1.0]
                .into_iter()
                .map(|focal_value| {
                    let predictor_values = BTreeMap::from([
                        (coefficient.focal_predictor_id().to_string(), focal_value),
                        (
                            coefficient.moderator_id().to_string(),
                            slope.moderator_value_standardized(),
                        ),
                    ]);
                    Ok(CanonicalInteractionPlotPointV1 {
                        focal_value,
                        predicted_value: interactions.standardized_outcome_linear_predictor_v1(
                            coefficient.outcome_id(),
                            &predictor_values,
                        )?,
                        lower: None,
                        upper: None,
                    })
                })
                .collect::<Result<Vec<_>, GeneralSemPlsInteractionPointErrorV1>>()?;
            series.push(CanonicalInteractionPlotSeriesV1 {
                series_id: format!(
                    "series:{}:{}",
                    coefficient.interaction_id(),
                    probe_value_index
                ),
                probe_id: probe_id.clone(),
                probe_value_index,
                moderator_value: slope.moderator_value_standardized(),
                points,
            });
        }
        plots.push(CanonicalInteractionPlotResultV1 {
            plot_id: format!("plot:{}", coefficient.interaction_id()),
            trace: trace.clone(),
            interaction_id: coefficient.interaction_id().to_string(),
            interaction_effect_id: Some(interaction_effect_id),
            focal_relation_id: coefficient.focal_relation_id().to_string(),
            focal_predictor_id: coefficient.focal_predictor_id().to_string(),
            moderator_id: coefficient.moderator_id().to_string(),
            outcome_id: coefficient.outcome_id().to_string(),
            series,
        });
    }
    interaction_effects.sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
    probes.sort_by(|left, right| left.probe_id.cmp(&right.probe_id));
    conditional_effects.sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
    plots.sort_by(|left, right| left.plot_id.cmp(&right.plot_id));
    Ok((interaction_effects, probes, conditional_effects, plots))
}

fn validate_bootstrap_inference_binding(
    result: &RecipeV4GeneralSemPlsExecutionResultV1,
) -> Result<
    BTreeMap<&str, &GeneralSemPlsBootstrapEffectInferenceV1>,
    RecipeV4GeneralSemPlsExecutionErrorV1,
> {
    let Some(bootstrap) = &result.bootstrap_inference else {
        return Ok(BTreeMap::new());
    };
    bootstrap.ensure_valid()?;
    if bootstrap.general_sem_config_sha256 != result.general_sem_config_sha256 {
        return Err(
            RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                "General SEM config digest differs from the execution result".to_string(),
            ),
        );
    }
    if bootstrap.compiled_plan_sha256 != result.compiled_plan_sha256 {
        return Err(
            RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                "compiled PLS v3 plan digest differs from the execution result".to_string(),
            ),
        );
    }
    if bootstrap.model_scientific_sha256 != result.model_scientific_sha256
        || bootstrap.source_dataset_fingerprint != result.source_dataset_fingerprint
    {
        return Err(
            RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                "bootstrap model or dataset authority differs from the point execution".to_string(),
            ),
        );
    }
    let mut expected_effect_ids = result
        .requested_effects
        .iter()
        .map(|effect| effect.canonical_effect_id().to_string())
        .collect::<Vec<_>>();
    expected_effect_ids.sort();
    if bootstrap.effect_ids != expected_effect_ids {
        return Err(
            RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                "bootstrap effect IDs do not exactly match the requested canonical effect IDs"
                    .to_string(),
            ),
        );
    }
    let expected_identity_digest = general_sem_effect_identity_set_sha256_v1(
        &requested_effect_identities_v1(&result.requested_effects),
    );
    if bootstrap.effect_identity_set_sha256 != expected_identity_digest {
        return Err(
            RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                "bootstrap typed effect identity digest differs from the requested estimands"
                    .to_string(),
            ),
        );
    }
    let mut by_id = BTreeMap::new();
    for inference in &bootstrap.effects {
        if by_id
            .insert(inference.effect_id.as_str(), inference)
            .is_some()
        {
            return Err(
                RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(format!(
                    "bootstrap effect ID {} is duplicated",
                    inference.effect_id
                )),
            );
        }
    }
    if by_id.len() != expected_effect_ids.len() {
        return Err(
            RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                "bootstrap inference rows do not exactly cover the requested effects".to_string(),
            ),
        );
    }
    for effect in &result.requested_effects {
        let inference = by_id.get(effect.canonical_effect_id()).ok_or_else(|| {
            RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(format!(
                "bootstrap inference is missing effect {}",
                effect.canonical_effect_id()
            ))
        })?;
        if inference.estimand_id != effect.estimand_id()
            || inference.original.to_bits() != effect.coefficient().to_bits()
        {
            return Err(
                RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(format!(
                    "bootstrap inference contradicts point effect {}",
                    effect.canonical_effect_id()
                )),
            );
        }
    }
    Ok(by_id)
}

fn canonical_inference_receipt(
    result: &RecipeV4GeneralSemPlsExecutionResultV1,
    bootstrap: &GeneralSemPlsBootstrapResultV1,
) -> Result<CanonicalGeneralSemInferenceReceiptV1, RecipeV4GeneralSemPlsExecutionErrorV1> {
    let interval = match bootstrap.interval {
        GeneralSemBootstrapIntervalV1::Percentile => {
            CanonicalGeneralSemBootstrapIntervalV1::PercentileType7
        }
        GeneralSemBootstrapIntervalV1::Bca => {
            return Err(
                RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                    "BCa bootstrap cannot be labeled as the percentile v1 canonical slice"
                        .to_string(),
                ),
            );
        }
    };
    let tail = match bootstrap.tail {
        GeneralSemInferenceTailV1::TwoSided => CanonicalGeneralSemInferenceTailV1::TwoSided,
        GeneralSemInferenceTailV1::OneSidedLower | GeneralSemInferenceTailV1::OneSidedUpper => {
            return Err(
                RecipeV4GeneralSemPlsExecutionErrorV1::InferenceResultMismatch(
                    "one-sided bootstrap cannot be labeled as the two-sided v1 canonical slice"
                        .to_string(),
                ),
            );
        }
    };
    Ok(CanonicalGeneralSemInferenceReceiptV1 {
        kind: CanonicalGeneralSemInferenceKindV1::CaseBootstrap,
        capability_cell: pls_general_bootstrap_capability_cell_v1(),
        method_version: bootstrap.method_version.clone(),
        resampling_operation_version: bootstrap.resampling_operation_version.clone(),
        resampling_stream_version: bootstrap.resampling_stream_version.clone(),
        quantile_method_version: bootstrap.quantile_method_version.clone(),
        standard_error_method_version: bootstrap.standard_error_method_version.clone(),
        summation_method_version: bootstrap.summation_method_version.clone(),
        p_value_method_version: bootstrap.p_value_method_version.clone(),
        failure_policy_version: bootstrap.failure_policy_version.clone(),
        compilation_artifact_identity_sha256: result.compilation_artifact_identity_sha256.clone(),
        compiled_plan_sha256: bootstrap.compiled_plan_sha256.clone(),
        general_sem_config_sha256: bootstrap.general_sem_config_sha256.clone(),
        recipe_analytical_sha256: result.recipe_analytical_sha256.clone(),
        model_scientific_sha256: bootstrap.model_scientific_sha256.clone(),
        source_dataset_fingerprint: bootstrap.source_dataset_fingerprint.clone(),
        complete_case_frame_sha256: bootstrap.complete_case_frame_sha256.clone(),
        usable_replicate_indices_sha256: bootstrap.usable_replicate_indices_sha256.clone(),
        effect_identity_set_sha256: bootstrap.effect_identity_set_sha256.clone(),
        effect_ids: bootstrap.effect_ids.clone(),
        interval,
        tail,
        confidence_level: bootstrap.confidence_level,
        resamples_requested: bootstrap.resamples_requested,
        resamples_usable: bootstrap.resamples_usable,
        minimum_usable_resamples: bootstrap.minimum_usable_resamples,
        seed: bootstrap.seed.clone(),
        workers: bootstrap.workers,
        complete_model_reestimated_per_replicate: bootstrap
            .complete_model_reestimated_per_replicate,
        failed_replicates: bootstrap
            .failed_replicates
            .iter()
            .map(|failure| CanonicalGeneralSemFailedReplicateV1 {
                replicate_index: failure.replicate_index,
                reason_code: match failure.reason_code {
                    qpls_resampling::GeneralSemPlsBootstrapFailureCodeV1::InsufficientObservations => {
                        CanonicalGeneralSemFailedReplicateReasonV1::InsufficientObservations
                    }
                    qpls_resampling::GeneralSemPlsBootstrapFailureCodeV1::ConstantIndicator => {
                        CanonicalGeneralSemFailedReplicateReasonV1::ConstantIndicator
                    }
                    qpls_resampling::GeneralSemPlsBootstrapFailureCodeV1::RankDeficient => {
                        CanonicalGeneralSemFailedReplicateReasonV1::RankDeficient
                    }
                    qpls_resampling::GeneralSemPlsBootstrapFailureCodeV1::IsolatedConstruct => {
                        CanonicalGeneralSemFailedReplicateReasonV1::IsolatedConstruct
                    }
                    qpls_resampling::GeneralSemPlsBootstrapFailureCodeV1::EstimationNonconvergence => {
                        CanonicalGeneralSemFailedReplicateReasonV1::EstimationNonconvergence
                    }
                    qpls_resampling::GeneralSemPlsBootstrapFailureCodeV1::NumericalFailure => {
                        CanonicalGeneralSemFailedReplicateReasonV1::NumericalFailure
                    }
                },
                message: failure.message.clone(),
            })
            .collect(),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum RecipeV4GeneralSemPlsExecutionErrorV1 {
    #[error("analysis was cancelled")]
    Cancelled,
    #[error(transparent)]
    Compilation(#[from] GeneralSemPlsRecipeCompilationErrorV1),
    #[error(transparent)]
    PointEstimation(#[from] RecipeV4PlsExecutionError),
    #[error(transparent)]
    InferenceProjection(#[from] ExecutionRecipeError),
    #[error(transparent)]
    Bootstrap(#[from] GeneralSemPlsBootstrapErrorV1),
    #[error(transparent)]
    InteractionPoint(#[from] GeneralSemPlsInteractionPointErrorV1),
    #[error(transparent)]
    EffectDecomposition(#[from] GeneralSemEffectsV1Error),
    #[error(
        "PLS result does not contain exactly one coefficient for relation {relation_id} ({source_id} -> {target_id})"
    )]
    RelationEstimateCardinality {
        relation_id: String,
        source_id: String,
        target_id: String,
    },
    #[error("compiled effect estimand {0} is absent from the complete decomposition")]
    MissingCompiledEffect(String),
    #[error("General SEM PLS bootstrap result is inconsistent: {0}")]
    InferenceResultMismatch(String),
}

/// Executes the current Labs point or bounded percentile-bootstrap slice for
/// general recursive PLS models. Every bootstrap replicate re-estimates the
/// complete model; no partial result is returned after cancellation or an
/// identity mismatch.
pub fn run_compiled_general_sem_pls_recipe_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    artifact: &CompiledGeneralSemPlsRecipeV1,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<RecipeV4GeneralSemPlsExecutionResultV1, RecipeV4GeneralSemPlsExecutionErrorV1> {
    if should_cancel() {
        return Err(RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled);
    }
    validate_compiled_general_sem_pls_recipe_v1(artifact, recipe, Some(resolved_model))?;
    let (base_recipe, stage_one_model) =
        project_general_sem_pls_stage_one_recipe_v1(recipe, resolved_model)?;
    let point_estimation = run_compiled_pls_recipe_v4(
        dataset,
        &base_recipe,
        &stage_one_model,
        artifact.base_artifact(),
        None,
        &should_cancel,
        &progress,
    )
    .map_err(|error| match error {
        RecipeV4PlsExecutionError::Cancelled => RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled,
        error => RecipeV4GeneralSemPlsExecutionErrorV1::PointEstimation(error),
    })?;
    if should_cancel() {
        return Err(RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled);
    }
    let interaction_point_estimation = if artifact.plan().two_way_interactions().is_empty() {
        None
    } else {
        progress(RunnerProgress {
            phase: "general_sem_interaction_point".into(),
            completed_units: 0,
            total_units: 1,
        });
        if should_cancel() {
            return Err(RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled);
        }
        let result = estimate_general_sem_pls_multiple_two_way_interactions_v1(
            artifact.plan(),
            &point_estimation.estimation().construct_scores,
        )?;
        result.ensure_valid_against_plan_v1(artifact.plan())?;
        progress(RunnerProgress {
            phase: "general_sem_interaction_point".into(),
            completed_units: 1,
            total_units: 1,
        });
        Some(result)
    };
    let requested_effects = if interaction_point_estimation.is_some() {
        Vec::new()
    } else {
        let relation_coefficients = relation_coefficients(artifact, point_estimation.estimation())?;
        let decomposition =
            decompose_general_sem_effects_v1(artifact.plan().topology(), &relation_coefficients)?;
        select_requested_effects(artifact, &decomposition)?
    };
    let config = recipe
        .general_sem_config
        .as_ref()
        .ok_or(GeneralSemPlsRecipeCompilationErrorV1::MissingGeneralSemConfig)?;
    let bootstrap_inference = match config.inference {
        GeneralSemInferenceV1::None => None,
        GeneralSemInferenceV1::CaseBootstrap { .. } => {
            let projected_recipe = project_pls_plan_to_current_recipe(
                &base_recipe,
                resolved_model,
                artifact.plan().base_plan(),
            )?;
            let execution =
                ValidatedExecutionRecipe::for_dataset(&projected_recipe, &dataset.fingerprint.0)?;
            let initialization = match base_recipe.method_config.as_ref() {
                Some(MethodConfig::PlsAlgorithmConfiguredV2(config)) => Some(config),
                _ => None,
            };
            Some(
                bootstrap_general_sem_pls_v1(
                    dataset,
                    &execution,
                    artifact.plan(),
                    point_estimation.estimation(),
                    config,
                    initialization,
                    recipe.settings.workers,
                    &should_cancel,
                    |update| {
                        progress(RunnerProgress {
                            phase: format!("general_sem_{}", update.phase.as_str()),
                            completed_units: u64::from(update.completed_replicates),
                            total_units: u64::from(update.total_replicates),
                        });
                    },
                )
                .map_err(|error| match error {
                    GeneralSemPlsBootstrapErrorV1::PointRefitCancelled
                    | GeneralSemPlsBootstrapErrorV1::Resampling(
                        qpls_resampling::ResamplingError::Cancelled,
                    ) => RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled,
                    error => RecipeV4GeneralSemPlsExecutionErrorV1::Bootstrap(error),
                })?,
            )
        }
    };
    if should_cancel() {
        return Err(RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled);
    }
    let adapter_version = if interaction_point_estimation.is_some() {
        RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1
    } else if bootstrap_inference.is_some() {
        RECIPE_V4_GENERAL_SEM_PLS_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1
    } else {
        RECIPE_V4_GENERAL_SEM_PLS_EXECUTION_ADAPTER_VERSION_V1
    };
    Ok(RecipeV4GeneralSemPlsExecutionResultV1 {
        schema_version: RECIPE_V4_GENERAL_SEM_PLS_EXECUTION_RESULT_SCHEMA_VERSION_V1,
        adapter_version: adapter_version.into(),
        capability_cell: artifact.capability_cell().clone(),
        compilation_artifact_identity_sha256: artifact.artifact_identity_sha256().into(),
        compiled_plan_sha256: artifact.plan().deterministic_sha256(),
        recipe_analytical_sha256: artifact.recipe_analytical_sha256().into(),
        model_scientific_sha256: artifact.plan().scientific_hash().into(),
        stage_one_model_scientific_sha256: artifact
            .plan()
            .stage_one_projection_scientific_sha256()
            .unwrap_or_else(|| artifact.plan().scientific_hash())
            .into(),
        source_dataset_fingerprint: dataset.fingerprint.0.clone(),
        general_sem_config_sha256: artifact.general_sem_config_sha256().into(),
        point_estimation,
        requested_effects,
        interaction_point_estimation,
        bootstrap_inference,
    })
}

fn relation_coefficients(
    artifact: &CompiledGeneralSemPlsRecipeV1,
    estimation: &qpls_estimation::PlsResult,
) -> Result<BTreeMap<String, f64>, RecipeV4GeneralSemPlsExecutionErrorV1> {
    artifact
        .plan()
        .topology()
        .structural_relations()
        .iter()
        .map(|relation| {
            let coefficients = match relation.role() {
                StructuralRelationRoleV4::Structural => estimation
                    .paths
                    .iter()
                    .filter(|estimate| {
                        estimate.source == relation.source() && estimate.target == relation.target()
                    })
                    .map(|estimate| estimate.coefficient)
                    .collect::<Vec<_>>(),
                StructuralRelationRoleV4::Control => estimation
                    .control_estimates
                    .iter()
                    .filter(|estimate| {
                        estimate.source == relation.source() && estimate.target == relation.target()
                    })
                    .map(|estimate| estimate.coefficient)
                    .collect::<Vec<_>>(),
            };
            if coefficients.len() != 1 {
                return Err(
                    RecipeV4GeneralSemPlsExecutionErrorV1::RelationEstimateCardinality {
                        relation_id: relation.relation_id().into(),
                        source_id: relation.source().into(),
                        target_id: relation.target().into(),
                    },
                );
            }
            Ok((relation.relation_id().into(), coefficients[0]))
        })
        .collect()
}

fn select_requested_effects(
    artifact: &CompiledGeneralSemPlsRecipeV1,
    decomposition: &GeneralSemEffectsV1,
) -> Result<Vec<GeneralSemRequestedEffectEstimateV1>, RecipeV4GeneralSemPlsExecutionErrorV1> {
    artifact
        .plan()
        .effect_estimands()
        .iter()
        .map(|estimand| match estimand {
            CompiledPlsEffectEstimandV3::SpecificIndirect {
                estimand_id,
                path_identity,
                source_id,
                target_id,
                ordered_relation_ids,
            } => {
                let effect = decomposition
                    .specific_indirect_effects()
                    .iter()
                    .find(|effect| effect.specific_path_identity() == path_identity)
                    .ok_or_else(|| {
                        RecipeV4GeneralSemPlsExecutionErrorV1::MissingCompiledEffect(
                            estimand_id.clone(),
                        )
                    })?;
                Ok(GeneralSemRequestedEffectEstimateV1::SpecificIndirect {
                    estimand_id: estimand_id.clone(),
                    path_identity: path_identity.clone(),
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    ordered_relation_ids: ordered_relation_ids.clone(),
                    coefficient: effect.coefficient(),
                })
            }
            CompiledPlsEffectEstimandV3::TotalIndirect {
                estimand_id,
                source_id,
                target_id,
                contributing_path_identities,
            } => {
                let effect = pair_effect(decomposition, estimand_id, source_id, target_id)?;
                Ok(GeneralSemRequestedEffectEstimateV1::TotalIndirect {
                    estimand_id: estimand_id.clone(),
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    contributing_path_identities: contributing_path_identities.clone(),
                    coefficient: effect.total_indirect_effect(),
                })
            }
            CompiledPlsEffectEstimandV3::TotalEffect {
                estimand_id,
                source_id,
                target_id,
                direct_relation_ids,
                contributing_indirect_path_identities,
            } => {
                let effect = pair_effect(decomposition, estimand_id, source_id, target_id)?;
                Ok(GeneralSemRequestedEffectEstimateV1::TotalEffect {
                    estimand_id: estimand_id.clone(),
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    direct_relation_ids: direct_relation_ids.clone(),
                    contributing_indirect_path_identities: contributing_indirect_path_identities
                        .clone(),
                    coefficient: effect.total_effect(),
                })
            }
        })
        .collect()
}

fn pair_effect<'a>(
    decomposition: &'a GeneralSemEffectsV1,
    estimand_id: &str,
    source_id: &str,
    target_id: &str,
) -> Result<&'a qpls_core::GeneralSemPairEffectsV1, RecipeV4GeneralSemPlsExecutionErrorV1> {
    decomposition
        .pair_effects()
        .iter()
        .find(|effect| effect.source_id() == source_id && effect.target_id() == target_id)
        .ok_or_else(|| {
            RecipeV4GeneralSemPlsExecutionErrorV1::MissingCompiledEffect(estimand_id.into())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe,
        AnalysisRecipeModelBindingV4, AnalysisSettings, Construct, GeneralSemConfigV1,
        InteractionHierarchyPolicyV2, InteractionMethodV4, LegacyBasicModelInterpretationV4,
        MeasurementMode, MethodConfig, ModelSpec, SemDataBindingV4, SemDerivedTermV4,
        SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4, StructuralPath,
        StructuralRelationRoleV4, compile_general_sem_pls_recipe_v1, compile_pls_plan_v3,
        confirm_legacy_recipe_estimand_v4, convert_legacy_basic_model_v4,
        migrate_analysis_recipe_to_v4_pending,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::collections::BTreeMap;
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };
    use uuid::Uuid;

    fn model() -> SemModelV4 {
        let constructs = ["x", "m1", "m2", "y"]
            .into_iter()
            .map(|id| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}1"), format!("{id}2")],
            })
            .collect();
        let paths = [
            ("x", "m1"),
            ("x", "m2"),
            ("x", "y"),
            ("m1", "m2"),
            ("m1", "y"),
            ("m2", "y"),
        ]
        .into_iter()
        .map(|(source, target)| StructuralPath {
            source: source.into(),
            target: target.into(),
        })
        .collect();
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::from_u128(0x5031_5305),
                name: "Effects fixture".into(),
                constructs,
                paths,
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap()
    }

    fn structural_relation_id(model: &SemModelV4, source: &str, target: &str) -> String {
        model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id,
                    source: actual_source,
                    target: actual_target,
                    ..
                } if actual_source == source && actual_target == target => Some(id.clone()),
                _ => None,
            })
            .unwrap()
    }

    fn add_two_way_interaction(
        model: &mut SemModelV4,
        interaction_id: &str,
        focal_predictor_id: &str,
        moderator_id: &str,
    ) {
        let focal_relation = structural_relation_id(model, focal_predictor_id, "construct:y");
        let output = format!("derived:{interaction_id}");
        let effect_relation = format!("relation:{interaction_id}:effect");
        let effect_parameter = format!("parameter:{interaction_id}:effect");
        model.variables.push(SemVariableV4::Derived {
            id: output.clone(),
            label: interaction_id.to_string(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: effect_relation,
            source: output.clone(),
            target: "construct:y".into(),
            parameter: effect_parameter.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: effect_parameter,
            label: format!("{interaction_id} -> Y"),
            target: SemParameterTargetV4::Regression {
                source: output.clone(),
                target: "construct:y".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
            id: interaction_id.to_string(),
            output,
            operands: vec![focal_predictor_id.to_string(), moderator_id.to_string()],
            focal_relation,
            method: InteractionMethodV4::TwoStage,
            hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
            product_indicator: None,
        });
        model.ensure_valid().unwrap();
    }

    fn moderation_execution_fixture(
        different_focal_paths: bool,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledGeneralSemPlsRecipeV1,
    ) {
        let source_model = ModelSpec {
            id: Uuid::from_u128(if different_focal_paths {
                0x4d4f_4452_554e_4e45_5200_0002
            } else {
                0x4d4f_4452_554e_4e45_5200_0001
            }),
            name: if different_focal_paths {
                "Different-focal simultaneous moderation"
            } else {
                "Same-focal simultaneous moderation"
            }
            .into(),
            constructs: ["x", "w", "z", "y"]
                .into_iter()
                .map(|id| Construct {
                    id: id.into(),
                    name: id.to_uppercase(),
                    short_name: id.to_uppercase(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec![format!("{id}1"), format!("{id}2")],
                })
                .collect(),
            paths: [("x", "y"), ("w", "y"), ("z", "y")]
                .into_iter()
                .map(|(source, target)| StructuralPath {
                    source: source.into(),
                    target: target.into(),
                })
                .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut csv = String::from("x1,x2,w1,w2,z1,z2,y1,y2\n");
        for row in 0..81 {
            let row = f64::from(row);
            let x = (row - 40.0) / 13.0;
            let w = (row * 0.71).sin() + 0.2 * (row * 0.13).cos();
            let z = (row * 0.37).cos() - 0.3 * (row * 0.19).sin();
            let interaction_signal = if different_focal_paths {
                0.55 * x * w - 0.35 * w * z
            } else {
                0.65 * x * w - 0.40 * x * z
            };
            let y = 0.25 * x + 0.20 * w - 0.15 * z + interaction_signal;
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                x + 0.03 * (row * 0.11).sin(),
                1.02 * x + 0.02 * (row * 0.17).cos(),
                w + 0.03 * (row * 0.23).cos(),
                0.98 * w + 0.02 * (row * 0.29).sin(),
                z + 0.03 * (row * 0.31).sin(),
                1.01 * z + 0.02 * (row * 0.41).cos(),
                y + 0.03 * (row * 0.43).sin(),
                1.01 * y + 0.02 * (row * 0.47).cos(),
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            if different_focal_paths {
                "general-sem-different-focal-moderation.csv"
            } else {
                "general-sem-same-focal-moderation.csv"
            },
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source_recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(if different_focal_paths {
                0x4d4f_4452_5245_4349_5045_0002
            } else {
                0x4d4f_4452_5245_4349_5045_0001
            }),
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
        add_two_way_interaction(
            &mut model,
            "interaction:x_by_w",
            "construct:x",
            "construct:w",
        );
        if different_focal_paths {
            add_two_way_interaction(
                &mut model,
                "interaction:w_by_z",
                "construct:w",
                "construct:z",
            );
        } else {
            add_two_way_interaction(
                &mut model,
                "interaction:x_by_z",
                "construct:x",
                "construct:z",
            );
        }
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256: model.scientific_sha256().unwrap(),
            model: model.clone(),
        };
        recipe.general_sem_config = Some(GeneralSemConfigV1::default());
        recipe.ensure_valid().unwrap();
        let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap();
        (dataset, recipe, model, artifact)
    }

    #[test]
    fn selected_effects_retain_stable_path_and_estimand_identities() {
        let model = model();
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        let coefficients = plan
            .topology()
            .structural_relations()
            .iter()
            .map(|relation| (relation.relation_id().to_string(), 0.5))
            .collect::<BTreeMap<_, _>>();
        let effects = decompose_general_sem_effects_v1(plan.topology(), &coefficients).unwrap();
        let x_to_y = effects
            .pair_effects()
            .iter()
            .find(|effect| {
                effect.source_id() == "construct:x" && effect.target_id() == "construct:y"
            })
            .unwrap();
        assert_eq!(x_to_y.direct_effect(), 0.5);
        assert_eq!(x_to_y.total_indirect_effect(), 0.625);
        assert_eq!(x_to_y.total_effect(), 1.125);
        assert!(plan.effect_estimands().iter().any(|estimand| matches!(
            estimand,
            CompiledPlsEffectEstimandV3::SpecificIndirect {
                ordered_relation_ids,
                ..
            } if ordered_relation_ids.len() == 3
        )));
    }

    #[test]
    fn production_pls_execution_reconciles_parallel_and_serial_specific_effects() {
        let source_model = ModelSpec {
            id: Uuid::from_u128(0x5031_5311),
            name: "General SEM execution fixture".into(),
            constructs: ["x", "m1", "m2", "y"]
                .into_iter()
                .map(|id| Construct {
                    id: id.into(),
                    name: id.to_uppercase(),
                    short_name: id.to_uppercase(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec![format!("{id}1"), format!("{id}2")],
                })
                .collect(),
            paths: [
                ("x", "m1"),
                ("x", "m2"),
                ("x", "y"),
                ("m1", "m2"),
                ("m1", "y"),
                ("m2", "y"),
            ]
            .into_iter()
            .map(|(source, target)| StructuralPath {
                source: source.into(),
                target: target.into(),
            })
            .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let dataset = import_delimited_bytes(
            b"x1,x2,m11,m12,m21,m22,y1,y2\n1,2,2,1,1,3,2,1\n2,1,3,2,2,2,3,2\n3,4,4,3,4,3,5,4\n4,3,5,5,3,5,6,5\n5,6,7,6,6,7,8,7\n6,5,6,7,7,6,9,8\n7,8,9,7,8,9,11,9\n8,7,8,9,9,8,10,11\n9,10,11,10,10,12,13,12\n10,9,12,11,12,10,14,13\n11,12,13,12,13,14,16,15\n12,11,14,13,14,13,17,16\n",
            "general-sem-pls-runner.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source_recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x5031_5312),
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
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256: model.scientific_sha256().unwrap(),
            model: model.clone(),
        };
        recipe.general_sem_config = Some(GeneralSemConfigV1::default());
        let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap();

        let progress = Mutex::new(Vec::new());
        let first = run_compiled_general_sem_pls_recipe_v1(
            &dataset,
            &recipe,
            &model,
            &artifact,
            || false,
            |update| progress.lock().unwrap().push(update),
        )
        .unwrap();
        let second = run_compiled_general_sem_pls_recipe_v1(
            &dataset,
            &recipe,
            &model,
            &artifact,
            || false,
            |_| {},
        )
        .unwrap();
        assert_eq!(first, second);
        assert!(first.point_estimation().estimation().converged);
        assert!(!progress.lock().unwrap().is_empty());

        let specifics = first
            .requested_effects()
            .iter()
            .filter_map(|effect| match effect {
                GeneralSemRequestedEffectEstimateV1::SpecificIndirect {
                    source_id,
                    target_id,
                    coefficient,
                    ..
                } if source_id == "construct:x" && target_id == "construct:y" => Some(*coefficient),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(specifics.len(), 3);
        let total_indirect = first
            .requested_effects()
            .iter()
            .find_map(|effect| match effect {
                GeneralSemRequestedEffectEstimateV1::TotalIndirect {
                    source_id,
                    target_id,
                    coefficient,
                    ..
                } if source_id == "construct:x" && target_id == "construct:y" => Some(*coefficient),
                _ => None,
            })
            .unwrap();
        let total = first
            .requested_effects()
            .iter()
            .find_map(|effect| match effect {
                GeneralSemRequestedEffectEstimateV1::TotalEffect {
                    source_id,
                    target_id,
                    coefficient,
                    ..
                } if source_id == "construct:x" && target_id == "construct:y" => Some(*coefficient),
                _ => None,
            })
            .unwrap();
        let direct = first
            .point_estimation()
            .estimation()
            .paths
            .iter()
            .find(|path| path.source == "construct:x" && path.target == "construct:y")
            .unwrap()
            .coefficient;
        assert!((specifics.iter().sum::<f64>() - total_indirect).abs() < 1e-12);
        assert!((direct + total_indirect - total).abs() < 1e-12);
        let canonical = first.canonical_general_sem_results_v1().unwrap();
        assert_eq!(canonical.schema_version, 1);
        assert_eq!(canonical.specific_indirect_effects.len(), 5);
        assert!(
            canonical
                .specific_indirect_effects
                .windows(2)
                .all(|pair| pair[0].effect_id < pair[1].effect_id)
        );
        assert!(
            canonical
                .aggregate_effects
                .windows(2)
                .all(|pair| pair[0].effect_id < pair[1].effect_id)
        );
        assert!(canonical.specific_indirect_effects.iter().all(|effect| {
            effect.value.standard_error.is_none()
                && effect.value.lower.is_none()
                && effect.value.upper.is_none()
                && effect.value.p_value.is_none()
        }));
        assert!(matches!(
            run_compiled_general_sem_pls_recipe_v1(
                &dataset,
                &recipe,
                &model,
                &artifact,
                || true,
                |_| {},
            ),
            Err(RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled)
        ));

        let mut bootstrap_recipe = recipe.clone();
        bootstrap_recipe.settings.bootstrap_samples = 20;
        bootstrap_recipe.settings.seed = qpls_core::GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1;
        bootstrap_recipe.settings.confidence_level = 0.95;
        bootstrap_recipe.settings.bootstrap_test_tail = qpls_core::PlsBootstrapTestTail::TwoSided;
        bootstrap_recipe.settings.studentized_inner_samples = 0;
        bootstrap_recipe.settings.workers = 2;
        bootstrap_recipe.general_sem_config = Some(GeneralSemConfigV1 {
            inference: GeneralSemInferenceV1::CaseBootstrap {
                resamples: 20,
                seed: qpls_core::GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1,
                confidence_level: 0.95,
                interval: GeneralSemBootstrapIntervalV1::Percentile,
                tail: GeneralSemInferenceTailV1::TwoSided,
            },
            ..GeneralSemConfigV1::default()
        });
        let bootstrap_artifact =
            compile_general_sem_pls_recipe_v1(&bootstrap_recipe, Some(&model)).unwrap();
        let bootstrap_result = run_compiled_general_sem_pls_recipe_v1(
            &dataset,
            &bootstrap_recipe,
            &model,
            &bootstrap_artifact,
            || false,
            |_| {},
        )
        .unwrap();
        assert_eq!(
            bootstrap_result.adapter_version(),
            RECIPE_V4_GENERAL_SEM_PLS_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1
        );
        let bootstrap = bootstrap_result.bootstrap_inference().unwrap();
        assert_eq!(bootstrap.resamples_requested, 20);
        assert!(bootstrap.resamples_usable >= 18);
        assert_eq!(
            bootstrap.effects.len(),
            bootstrap_result.requested_effects().len()
        );
        let canonical = bootstrap_result.canonical_general_sem_results_v1().unwrap();
        let receipt = canonical.inference_receipt.as_ref().unwrap();
        assert_eq!(
            receipt.seed,
            qpls_core::GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1.to_string()
        );
        assert_eq!(receipt.resamples_requested, 20);
        assert_eq!(receipt.resamples_usable, bootstrap.resamples_usable);
        assert!(canonical.specific_indirect_effects.iter().all(|effect| {
            effect.value.bootstrap_mean.is_some()
                && effect.value.bootstrap_bias.is_some()
                && effect.value.standard_error.is_some()
                && effect.value.lower.is_some()
                && effect.value.upper.is_some()
                && effect.value.p_value.is_some()
        }));
        assert!(canonical.aggregate_effects.iter().all(|effect| {
            effect.value.bootstrap_mean.is_some()
                && effect.value.bootstrap_bias.is_some()
                && effect.value.standard_error.is_some()
                && effect.value.lower.is_some()
                && effect.value.upper.is_some()
                && effect.value.p_value.is_some()
        }));

        let mut tampered = bootstrap_result.clone();
        tampered.bootstrap_inference.as_mut().unwrap().effects[0].original += 0.01;
        assert!(matches!(
            tampered.canonical_general_sem_results_v1(),
            Err(RecipeV4GeneralSemPlsExecutionErrorV1::Bootstrap(
                GeneralSemPlsBootstrapErrorV1::InvalidResultContract(_)
            ))
        ));

        let cancel_during_bootstrap = AtomicBool::new(false);
        assert!(matches!(
            run_compiled_general_sem_pls_recipe_v1(
                &dataset,
                &bootstrap_recipe,
                &model,
                &bootstrap_artifact,
                || cancel_during_bootstrap.load(Ordering::SeqCst),
                |update| {
                    if update.phase.starts_with("general_sem_") {
                        cancel_during_bootstrap.store(true, Ordering::SeqCst);
                    }
                },
            ),
            Err(RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled)
        ));
    }

    #[test]
    fn production_runner_publishes_joint_same_and_different_focal_interaction_authority() {
        for different_focal_paths in [false, true] {
            let (dataset, recipe, model, artifact) =
                moderation_execution_fixture(different_focal_paths);
            assert_eq!(
                artifact.capability_cell(),
                &qpls_core::pls_general_multiple_moderation_point_capability_cell_v1()
            );
            assert_eq!(
                artifact.compiler_version(),
                qpls_core::GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_RECIPE_COMPILER_VERSION_V1
            );
            assert_eq!(artifact.plan().two_way_interactions().len(), 2);
            assert_ne!(
                artifact.plan().scientific_hash(),
                artifact
                    .plan()
                    .stage_one_projection_scientific_sha256()
                    .unwrap()
            );

            let first = run_compiled_general_sem_pls_recipe_v1(
                &dataset,
                &recipe,
                &model,
                &artifact,
                || false,
                |_| {},
            )
            .unwrap();
            let second = run_compiled_general_sem_pls_recipe_v1(
                &dataset,
                &recipe,
                &model,
                &artifact,
                || false,
                |_| {},
            )
            .unwrap();
            assert_eq!(first, second);
            assert_eq!(
                first.adapter_version(),
                RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1
            );
            assert!(first.requested_effects().is_empty());
            assert!(first.bootstrap_inference().is_none());
            first
                .interaction_point_estimation()
                .unwrap()
                .ensure_valid_against_plan_v1(artifact.plan())
                .unwrap();

            let canonical = first.canonical_general_sem_results_v1().unwrap();
            assert_eq!(canonical.interaction_effects.len(), 2);
            assert_eq!(canonical.conditional_effect_probes.len(), 2);
            assert_eq!(canonical.conditional_effects.len(), 6);
            assert_eq!(canonical.interaction_plots.len(), 2);
            assert!(canonical.specific_indirect_effects.is_empty());
            assert!(canonical.aggregate_effects.is_empty());
            assert!(canonical.interaction_effects.iter().all(|effect| {
                effect.method_version
                    == qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
                    && effect.product_scale_version
                        == qpls_core::GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
                    && effect.stage_one_model_scientific_sha256
                        == first.stage_one_model_scientific_sha256()
                    && effect.construction_method
                        == CanonicalInteractionConstructionMethodV1::TwoStage
                    && (effect.standardized_product_coefficient.estimate
                        - effect.scientific_rescaled_gamma.estimate
                            * effect.unstandardized_product_sample_standard_deviation)
                        .abs()
                        <= f64::EPSILON * 8.0
            }));
            let effect_ids = canonical
                .interaction_effects
                .iter()
                .map(|effect| effect.effect_id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            assert!(canonical.conditional_effects.iter().all(|effect| {
                effect
                    .interaction_effect_id
                    .as_deref()
                    .is_some_and(|effect_id| effect_ids.contains(effect_id))
            }));
            assert!(canonical.interaction_plots.iter().all(|plot| {
                plot.interaction_effect_id
                    .as_deref()
                    .is_some_and(|effect_id| effect_ids.contains(effect_id))
            }));
        }
    }

    #[test]
    fn interaction_stage_cancellation_and_scale_tampering_publish_no_canonical_result() {
        let (dataset, recipe, model, artifact) = moderation_execution_fixture(false);
        let cancel = AtomicBool::new(false);
        let cancelled = run_compiled_general_sem_pls_recipe_v1(
            &dataset,
            &recipe,
            &model,
            &artifact,
            || cancel.load(Ordering::SeqCst),
            |update| {
                if update.phase == "general_sem_interaction_point" && update.completed_units == 0 {
                    cancel.store(true, Ordering::SeqCst);
                }
            },
        );
        assert!(matches!(
            cancelled,
            Err(RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled)
        ));

        let mut result = run_compiled_general_sem_pls_recipe_v1(
            &dataset,
            &recipe,
            &model,
            &artifact,
            || false,
            |_| {},
        )
        .unwrap();
        let mut payload =
            serde_json::to_value(result.interaction_point_estimation().unwrap()).unwrap();
        payload["interaction_coefficients"][0]["raw_product_estimate"] = serde_json::json!(123.456);
        result.interaction_point_estimation = Some(serde_json::from_value(payload).unwrap());
        assert!(matches!(
            result.canonical_general_sem_results_v1(),
            Err(RecipeV4GeneralSemPlsExecutionErrorV1::InteractionPoint(
                GeneralSemPlsInteractionPointErrorV1::InvalidResultContract(_)
            ))
        ));
    }
}
