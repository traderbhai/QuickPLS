use crate::{
    RecipeV4PlsExecutionError, RecipeV4PlsExecutionResultV1, RunnerProgress,
    run_compiled_pls_recipe_v4,
};
use qpls_core::{
    AnalysisRecipeV4, CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION,
    CanonicalAggregateEffectKindV1, CanonicalAggregateEffectResultV1,
    CanonicalGeneralSemEstimateV1, CanonicalGeneralSemResultTraceV1,
    CanonicalGeneralSemResultsV1, CanonicalSpecificIndirectEffectResultV1,
    CapabilityCellReferenceV2, CompiledGeneralSemPlsRecipeV1, CompiledPlsEffectEstimandV3,
    GeneralSemEffectsV1, GeneralSemEffectsV1Error, GeneralSemPlsRecipeCompilationErrorV1,
    SemModelV4, StructuralRelationRoleV4, decompose_general_sem_effects_v1,
    validate_compiled_general_sem_pls_recipe_v1,
};
use qpls_data::Dataset;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const RECIPE_V4_GENERAL_SEM_PLS_EXECUTION_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub const RECIPE_V4_GENERAL_SEM_PLS_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_general_sem_pls_recipe_v1_point_execution_v1";

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecipeV4GeneralSemPlsExecutionResultV1 {
    schema_version: u32,
    adapter_version: String,
    capability_cell: CapabilityCellReferenceV2,
    compilation_artifact_identity_sha256: String,
    general_sem_config_sha256: String,
    point_estimation: RecipeV4PlsExecutionResultV1,
    requested_effects: Vec<GeneralSemRequestedEffectEstimateV1>,
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

    pub fn point_estimation(&self) -> &RecipeV4PlsExecutionResultV1 {
        &self.point_estimation
    }

    pub fn requested_effects(&self) -> &[GeneralSemRequestedEffectEstimateV1] {
        &self.requested_effects
    }

    /// Produces the typed additive section consumed by
    /// `CanonicalResultDocumentV2`. Inference fields remain absent because this
    /// execution adapter is explicitly point-estimation-only.
    pub fn canonical_general_sem_results_v1(&self) -> CanonicalGeneralSemResultsV1 {
        let trace = CanonicalGeneralSemResultTraceV1 {
            model_id: self
                .point_estimation
                .provenance()
                .compilation_receipt()
                .model_id()
                .into(),
            capability_cell: self.capability_cell.clone(),
        };
        let mut specific_indirect_effects = Vec::new();
        let mut aggregate_effects = Vec::new();
        for effect in &self.requested_effects {
            match effect {
                GeneralSemRequestedEffectEstimateV1::SpecificIndirect {
                    estimand_id,
                    path_identity,
                    ordered_relation_ids,
                    coefficient,
                    ..
                } => specific_indirect_effects.push(CanonicalSpecificIndirectEffectResultV1 {
                    effect_id: path_identity.clone(),
                    estimand_id: estimand_id.clone(),
                    trace: trace.clone(),
                    ordered_relation_ids: ordered_relation_ids.clone(),
                    value: point_estimate(*coefficient),
                }),
                GeneralSemRequestedEffectEstimateV1::TotalIndirect {
                    estimand_id,
                    source_id,
                    target_id,
                    coefficient,
                    ..
                } => aggregate_effects.push(CanonicalAggregateEffectResultV1 {
                    effect_id: estimand_id.clone(),
                    estimand_id: estimand_id.clone(),
                    trace: trace.clone(),
                    kind: CanonicalAggregateEffectKindV1::TotalIndirect,
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    value: point_estimate(*coefficient),
                }),
                GeneralSemRequestedEffectEstimateV1::TotalEffect {
                    estimand_id,
                    source_id,
                    target_id,
                    coefficient,
                    ..
                } => aggregate_effects.push(CanonicalAggregateEffectResultV1 {
                    effect_id: estimand_id.clone(),
                    estimand_id: estimand_id.clone(),
                    trace: trace.clone(),
                    kind: CanonicalAggregateEffectKindV1::TotalEffect,
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    value: point_estimate(*coefficient),
                }),
            }
        }
        specific_indirect_effects.sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
        aggregate_effects.sort_by(|left, right| left.effect_id.cmp(&right.effect_id));
        CanonicalGeneralSemResultsV1 {
            schema_version: CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION,
            specific_indirect_effects,
            aggregate_effects,
            conditional_effect_probes: Vec::new(),
            conditional_effects: Vec::new(),
            interaction_plots: Vec::new(),
            higher_order_stages: Vec::new(),
            cbsem_fit: Vec::new(),
            identification_diagnostics: Vec::new(),
        }
    }
}

fn point_estimate(estimate: f64) -> CanonicalGeneralSemEstimateV1 {
    CanonicalGeneralSemEstimateV1 {
        estimate,
        standard_error: None,
        lower: None,
        upper: None,
        p_value: None,
    }
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
}

/// Executes the current Labs point-estimation slice for general recursive PLS
/// models. It reuses the proven recipe-v4 score executor, then decomposes the
/// exact compiled relation graph. No partial result is returned after a
/// cancellation or coefficient-domain mismatch.
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
    let point_estimation = run_compiled_pls_recipe_v4(
        dataset,
        recipe,
        resolved_model,
        artifact.base_artifact(),
        None,
        &should_cancel,
        progress,
    )?;
    if should_cancel() {
        return Err(RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled);
    }
    let relation_coefficients = relation_coefficients(
        artifact,
        point_estimation.estimation(),
    )?;
    let decomposition = decompose_general_sem_effects_v1(
        artifact.plan().topology(),
        &relation_coefficients,
    )?;
    let requested_effects = select_requested_effects(artifact, &decomposition)?;
    Ok(RecipeV4GeneralSemPlsExecutionResultV1 {
        schema_version: RECIPE_V4_GENERAL_SEM_PLS_EXECUTION_RESULT_SCHEMA_VERSION_V1,
        adapter_version: RECIPE_V4_GENERAL_SEM_PLS_EXECUTION_ADAPTER_VERSION_V1.into(),
        capability_cell: artifact.capability_cell().clone(),
        compilation_artifact_identity_sha256: artifact.artifact_identity_sha256().into(),
        general_sem_config_sha256: artifact.general_sem_config_sha256().into(),
        point_estimation,
        requested_effects,
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
                        estimate.source == relation.source()
                            && estimate.target == relation.target()
                    })
                    .map(|estimate| estimate.coefficient)
                    .collect::<Vec<_>>(),
                StructuralRelationRoleV4::Control => estimation
                    .control_estimates
                    .iter()
                    .filter(|estimate| {
                        estimate.source == relation.source()
                            && estimate.target == relation.target()
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
                    contributing_indirect_path_identities:
                        contributing_indirect_path_identities.clone(),
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
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe,
        AnalysisRecipeModelBindingV4, AnalysisSettings, Construct, GeneralSemConfigV1,
        LegacyBasicModelInterpretationV4, MeasurementMode, MethodConfig, ModelSpec,
        SemDataBindingV4, StructuralPath, compile_general_sem_pls_recipe_v1,
        compile_pls_plan_v3, confirm_legacy_recipe_estimand_v4,
        convert_legacy_basic_model_v4, migrate_analysis_recipe_to_v4_pending,
    };
    use chrono::{TimeZone, Utc};
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::collections::BTreeMap;
    use std::sync::Mutex;
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
                } if source_id == "construct:x" && target_id == "construct:y" => {
                    Some(*coefficient)
                }
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
                } if source_id == "construct:x" && target_id == "construct:y" => {
                    Some(*coefficient)
                }
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
                } if source_id == "construct:x" && target_id == "construct:y" => {
                    Some(*coefficient)
                }
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
        let canonical = first.canonical_general_sem_results_v1();
        assert_eq!(canonical.schema_version, 1);
        assert_eq!(canonical.specific_indirect_effects.len(), 5);
        assert!(canonical
            .specific_indirect_effects
            .windows(2)
            .all(|pair| pair[0].effect_id < pair[1].effect_id));
        assert!(canonical
            .aggregate_effects
            .windows(2)
            .all(|pair| pair[0].effect_id < pair[1].effect_id));
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
    }
}
