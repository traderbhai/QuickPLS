//! Build-only fixture preparation for installed/portable MultiMod qualification.
//!
//! The command is always registered so preview builds have a stable fail-closed
//! surface, but fixture construction is compiled only for candidate packages
//! built with `multimod-qualification-harness`.  It creates new isolated V6
//! archives; scientific execution, promotion, sidecar publication, canonical
//! export, and strict reopen continue through the ordinary native commands.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MultiModPackagedQualificationUnavailableV1 {
    schema_version: u32,
    code: &'static str,
    message: &'static str,
}

#[cfg(feature = "multimod-qualification-harness")]
mod enabled {
    use super::*;
    use crate::multimod_candidate_authority_v1::{
        NativeMultiModCandidateAuthorityStateV1, embedded_multimod_candidate_authority_v1,
    };
    use chrono::{TimeZone, Utc};
    use qpls_core::*;
    use qpls_data::{Dataset, ImportOptions, import_delimited_bytes};
    use qpls_project::{create_populated_general_sem_project_archive_v6, load_project_archive_v6};
    use serde::Deserialize;
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use std::{
        collections::BTreeMap,
        fs::{self, File, OpenOptions},
        io::{Read, Write},
        path::{Path, PathBuf},
    };
    use uuid::Uuid;
    use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

    const SURFACE: &str = "internal_labs_multimod_packaged_qualification_v1";

    type FixtureError = Box<dyn std::error::Error + Send + Sync>;

    fn invalid(message: impl Into<String>) -> FixtureError {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, message.into()).into()
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub(super) struct PrepareRequestV1 {
        surface: String,
        experimental_labs_enabled: bool,
        output_directory: String,
        seed: u64,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub(super) struct IntegrityRequestV1 {
        surface: String,
        experimental_labs_enabled: bool,
        fixture_root: String,
        archive_path: String,
        expected_archive_sha256: String,
        result_id: String,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ArchiveAuthorityV1 {
        archive_path: String,
        archive_sha256: String,
        project_id: String,
        dataset_id: String,
        dataset_fingerprint: String,
        model_id: String,
        model_scientific_sha256: String,
        source_recipe_id: String,
        source_recipe_document_sha256: String,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct FamilyFixtureV1 {
        family_id: &'static str,
        authority: ArchiveAuthorityV1,
        config: Value,
        cancellation_recovery_required: bool,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct PreparedFixturesV1 {
        schema_version: u32,
        fixture_id: &'static str,
        surface: &'static str,
        seed: u64,
        output_directory: String,
        families: Vec<FamilyFixtureV1>,
        production_lifecycle_required: Vec<&'static str>,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct IntegrityVariantsV1 {
        schema_version: u32,
        receipt_id: &'static str,
        source_archive_sha256: String,
        sidecar_entry: String,
        missing_sidecar_archive_path: String,
        missing_sidecar_archive_sha256: String,
        tampered_sidecar_archive_path: String,
        tampered_sidecar_archive_sha256: String,
        production_strict_reopen_rejected_missing: bool,
        production_strict_reopen_rejected_tamper: bool,
    }

    fn require_enabled_surface(surface: &str, labs: bool) -> Result<(), FixtureError> {
        if surface != SURFACE || !labs {
            return Err(invalid(
                "the packaged qualification surface and Labs confirmation are required",
            ));
        }
        let authority = embedded_multimod_candidate_authority_v1().map_err(invalid)?;
        if authority.state != NativeMultiModCandidateAuthorityStateV1::ReleaseQualifiedCandidate {
            return Err(invalid(
                "the build-embedded MultiMod candidate authority is unavailable",
            ));
        }
        Ok(())
    }

    fn lower_sha256(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn file_sha256(path: &Path) -> Result<String, FixtureError> {
        Ok(lower_sha256(&fs::read(path)?))
    }

    fn dataset_from_columns(
        source_name: &str,
        headers: &[&str],
        columns: &[Vec<String>],
    ) -> Result<Dataset, FixtureError> {
        let rows = columns
            .first()
            .map(Vec::len)
            .ok_or_else(|| invalid("fixture has no columns"))?;
        if headers.len() != columns.len() || columns.iter().any(|column| column.len() != rows) {
            return Err(invalid("fixture columns have inconsistent dimensions"));
        }
        let mut csv = headers.join(",") + "\n";
        for row in 0..rows {
            for (column, values) in columns.iter().enumerate() {
                if column != 0 {
                    csv.push(',');
                }
                csv.push_str(&values[row]);
            }
            csv.push('\n');
        }
        Ok(import_delimited_bytes(
            csv.as_bytes(),
            source_name,
            b',',
            &ImportOptions::default(),
        )?)
    }

    fn numeric(values: impl IntoIterator<Item = f64>) -> Vec<String> {
        values
            .into_iter()
            .map(|value| format!("{value:.17}"))
            .collect()
    }

    fn base_recipe_model(
        dataset: &Dataset,
        fixture_id: u128,
        name: &str,
        constructs: &[(&str, &[&str])],
        paths: &[(&str, &str)],
        seed: u64,
    ) -> Result<(AnalysisRecipeV4, SemModelV4), FixtureError> {
        let source_model = ModelSpec {
            id: Uuid::from_u128(fixture_id),
            name: name.into(),
            constructs: constructs
                .iter()
                .map(|(id, indicators)| Construct {
                    id: (*id).into(),
                    name: id.to_uppercase(),
                    short_name: id.to_uppercase(),
                    mode: MeasurementMode::Reflective,
                    indicators: indicators.iter().map(|value| (*value).into()).collect(),
                })
                .collect(),
            paths: paths
                .iter()
                .map(|(source, target)| StructuralPath {
                    source: (*source).into(),
                    target: (*target).into(),
                })
                .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let source = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(fixture_id ^ 0x4d55_4c54_494d_4f44),
            created_at: Utc
                .timestamp_opt(1_700_000_000, 0)
                .single()
                .ok_or_else(|| invalid("fixed fixture timestamp is invalid"))?,
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: source_model.clone(),
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                bootstrap_samples: 0,
                permutation_samples: 0,
                seed,
                confidence_level: 0.95,
                bootstrap_test_tail: PlsBootstrapTestTail::TwoSided,
                studentized_inner_samples: 0,
                workers: 1,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let pending = migrate_analysis_recipe_to_v4_pending(&source)?;
        let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source_model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )?;
        let SemDataBindingV4::Raw {
            dataset_id,
            missing_data,
            ..
        } = &mut model.data_binding
        else {
            return Err(invalid(
                "migrated fixture did not retain a raw data binding",
            ));
        };
        *dataset_id = dataset.id.to_string();
        *missing_data = MissingDataPolicyV4::ListwiseDeletion;
        recipe.settings.seed = seed;
        recipe.settings.workers = 1;
        recipe.settings.bootstrap_samples = 0;
        recipe.settings.permutation_samples = 0;
        recipe.settings.studentized_inner_samples = 0;
        recipe.general_sem_config = Some(GeneralSemConfigV1::default());
        Ok((recipe, model))
    }

    fn observed_control(
        id: &str,
        source_column: &str,
        scale: ObservedScaleV4,
        categories: Vec<String>,
    ) -> SemVariableV4 {
        SemVariableV4::Observed {
            id: id.into(),
            label: id.into(),
            source_column: source_column.into(),
            scale,
            role: ObservedRoleV4::Control,
            categories,
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        }
    }

    fn relation_id(model: &SemModelV4, source: &str, target: &str) -> Result<String, FixtureError> {
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
            .ok_or_else(|| invalid(format!("missing structural relation {source}->{target}")))
    }

    fn add_interaction(
        model: &mut SemModelV4,
        interaction_id: &str,
        operands: &[&str],
        focal_predictor: &str,
        outcome: &str,
    ) -> Result<(), FixtureError> {
        let focal_relation = relation_id(model, focal_predictor, outcome)?;
        let output = format!("derived:{interaction_id}");
        let relation = format!("relation:{interaction_id}:effect");
        let parameter = format!("parameter:{interaction_id}:effect");
        model.variables.push(SemVariableV4::Derived {
            id: output.clone(),
            label: interaction_id.into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: relation,
            source: output.clone(),
            target: outcome.into(),
            parameter: parameter.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: parameter,
            label: format!("{interaction_id} -> {outcome}"),
            target: SemParameterTargetV4::Regression {
                source: output.clone(),
                target: outcome.into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
            id: interaction_id.into(),
            output,
            operands: operands.iter().map(|value| (*value).into()).collect(),
            focal_relation,
            method: InteractionMethodV4::TwoStage,
            hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
            product_indicator: None,
        });
        Ok(())
    }

    fn bind_project_recipe(
        recipe: &mut AnalysisRecipeV4,
        model: &SemModelV4,
    ) -> Result<(), FixtureError> {
        model.ensure_valid()?;
        recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: model.id.clone(),
            scientific_sha256: model.scientific_sha256()?,
        };
        recipe.mga_multigroup = None;
        recipe.pls_heterogeneity = None;
        recipe.general_sem_conditional_process = None;
        recipe.interventional_causal_mediation = None;
        recipe.ensure_valid()?;
        Ok(())
    }

    fn create_archive(
        root: &Path,
        name: &str,
        project_id: Uuid,
        dataset: Dataset,
        mut recipe: AnalysisRecipeV4,
        model: SemModelV4,
        config: Value,
        family_id: &'static str,
        cancellation_recovery_required: bool,
    ) -> Result<FamilyFixtureV1, FixtureError> {
        bind_project_recipe(&mut recipe, &model)?;
        let model_sha = model.scientific_sha256()?;
        let source_recipe_sha = sha256_serialized(&recipe);
        let archive = root.join(format!("{name}.qpls"));
        create_populated_general_sem_project_archive_v6(
            &archive,
            project_id,
            format!("MultiMod packaged qualification {name}"),
            Utc.timestamp_opt(1_700_000_100, 0)
                .single()
                .ok_or_else(|| invalid("fixed project timestamp is invalid"))?,
            &dataset,
            model.clone(),
            recipe.clone(),
        )?;
        let loaded = load_project_archive_v6(&archive)?;
        if loaded.document.project_id != project_id || loaded.datasets.len() != 1 {
            return Err(invalid("new fixture archive failed strict identity reopen"));
        }
        Ok(FamilyFixtureV1 {
            family_id,
            authority: ArchiveAuthorityV1 {
                archive_path: archive.to_string_lossy().into_owned(),
                archive_sha256: file_sha256(&archive)?,
                project_id: project_id.to_string(),
                dataset_id: dataset.id.to_string(),
                dataset_fingerprint: dataset.fingerprint.0,
                model_id: model.id,
                model_scientific_sha256: model_sha,
                source_recipe_id: recipe.id.to_string(),
                source_recipe_document_sha256: source_recipe_sha,
            },
            config,
            cancellation_recovery_required,
        })
    }

    fn checklist() -> MicomConfiguralChecklistV1 {
        MicomConfiguralChecklistV1 {
            identical_indicators_and_coding: true,
            identical_data_treatment: true,
            identical_algorithm_settings: true,
            identical_model_specification: true,
            deterministic_sign_orientation_reviewed: true,
            analyst_review_confirmed: true,
        }
    }

    fn mga_fixture(root: &Path, seed: u64) -> Result<FamilyFixtureV1, FixtureError> {
        let rows_per_group = 16usize;
        let groups = 5usize;
        let mut group = Vec::new();
        let mut x = Vec::new();
        let mut y = Vec::new();
        for group_index in 0..groups {
            for row in 0..rows_per_group {
                let t = (group_index * rows_per_group + row) as f64 + 1.0;
                let latent_x = (t * 0.173).sin() + (t * 0.071).cos();
                let slope = 0.45 + group_index as f64 * 0.24;
                let latent_y = slope * latent_x + 0.18 * (t * 0.313).sin();
                group.push(format!("G{:02}", group_index + 1));
                x.push(latent_x);
                y.push(latent_y);
            }
        }
        let dataset = dataset_from_columns(
            "multimod-packaged-mga.csv",
            &["x1", "x2", "y1", "y2", "group"],
            &[
                numeric(
                    x.iter()
                        .enumerate()
                        .map(|(i, value)| value + 0.03 * ((i + 1) as f64 * 0.41).sin()),
                ),
                numeric(
                    x.iter()
                        .enumerate()
                        .map(|(i, value)| 0.94 * value + 0.04 * ((i + 1) as f64 * 0.29).cos()),
                ),
                numeric(
                    y.iter()
                        .enumerate()
                        .map(|(i, value)| value + 0.03 * ((i + 1) as f64 * 0.37).sin()),
                ),
                numeric(
                    y.iter()
                        .enumerate()
                        .map(|(i, value)| 0.91 * value + 0.05 * ((i + 1) as f64 * 0.23).cos()),
                ),
                group,
            ],
        )?;
        let (recipe, mut model) = base_recipe_model(
            &dataset,
            0x6d75_6c74_696d_6f64_706b_6700_0000_0001,
            "packaged-mga-model",
            &[("x", &["x1", "x2"]), ("y", &["y1", "y2"])],
            &[("x", "y")],
            seed,
        )?;
        model.variables.push(observed_control(
            "observed:packaged_group",
            "group",
            ObservedScaleV4::Nominal,
            (1..=groups).map(|index| format!("G{index:02}")).collect(),
        ));
        model.group = SemGroupV4::ObservedGroups {
            grouping_variable: "observed:packaged_group".into(),
            levels: (1..=groups)
                .map(|index| SemGroupLevelV4 {
                    id: format!("g{index:02}"),
                    value: format!("G{index:02}"),
                    label: format!("Group {index:02}"),
                })
                .collect(),
        };
        let config = MgaMultigroupV1 {
            schema_version: 1,
            profile: MgaModelProfileV1::GeneralSemPls,
            grouping_column: "group".into(),
            groups: (1..=groups)
                .map(|index| SelectedGroupV1 {
                    group_id: format!("g{index:02}"),
                    label: format!("Group {index:02}"),
                    value: TypedGroupValueV1::Text {
                        value: format!("G{index:02}"),
                    },
                })
                .collect(),
            comparison_plan: MgaComparisonPlanV1::AllPairs {
                heavy_run_confirmed: false,
            },
            procedures: vec![
                MgaProcedureV1::OmnibusMaxSpreadPermutation,
                MgaProcedureV1::MicomPairwise,
                MgaProcedureV1::PairwisePermutation,
            ],
            permutation_samples: 5_000,
            bootstrap_samples: 5_000,
            seed,
            confidence_level: 0.95,
            alpha: 0.05,
            alternative: InferenceAlternativeV1::TwoSided,
            multiplicity: MultiplicityAdjustmentV1::Holm,
            configural_checklist: checklist(),
            weight: None,
            selected_parameter_ids: Vec::new(),
        };
        config.ensure_valid()?;
        create_archive(
            root,
            "mga",
            Uuid::from_u128(0x6d75_6c74_696d_6f64_706b_6700_1000_0001),
            dataset,
            recipe,
            model,
            serde_json::to_value(config)?,
            "qpls.multimod.mga_multigroup_v1",
            true,
        )
    }

    fn heterogeneity_fixture(root: &Path, seed: u64) -> Result<FamilyFixtureV1, FixtureError> {
        let rows = 160usize;
        let mut x = Vec::new();
        let mut y = Vec::new();
        for row in 0..rows {
            let t = row as f64 + 1.0;
            let latent_x = (t * 0.137).sin() + 0.55 * (t * 0.059).cos();
            let class_sign = if row % 2 == 0 { -1.0 } else { 1.0 };
            x.push(latent_x);
            y.push(class_sign * (1.6 * latent_x + 0.5) + 0.07 * (t * 0.311).sin());
        }
        let dataset = dataset_from_columns(
            "multimod-packaged-heterogeneity.csv",
            &["x1", "x2", "y1", "y2"],
            &[
                numeric(
                    x.iter()
                        .enumerate()
                        .map(|(i, value)| value + 0.025 * ((i + 1) as f64 * 0.43).sin()),
                ),
                numeric(
                    x.iter()
                        .enumerate()
                        .map(|(i, value)| 0.93 * value + 0.03 * ((i + 1) as f64 * 0.31).cos()),
                ),
                numeric(
                    y.iter()
                        .enumerate()
                        .map(|(i, value)| value + 0.02 * ((i + 1) as f64 * 0.47).sin()),
                ),
                numeric(
                    y.iter()
                        .enumerate()
                        .map(|(i, value)| 0.95 * value + 0.03 * ((i + 1) as f64 * 0.19).cos()),
                ),
            ],
        )?;
        let (recipe, model) = base_recipe_model(
            &dataset,
            0x6d75_6c74_696d_6f64_706b_6700_0000_0002,
            "packaged-heterogeneity-model",
            &[("x", &["x1", "x2"]), ("y", &["y1", "y2"])],
            &[("x", "y")],
            seed,
        )?;
        let config = PlsUnobservedHeterogeneityConfigV2 {
            schema_version: 2,
            profile: HeterogeneityInteractionProfileV2::P0Structural,
            phase: HeterogeneityPhaseV2::Discovery {
                candidate_k: vec![2],
                algorithms: vec![
                    HeterogeneityAlgorithmV2::FimixPlsV2,
                    HeterogeneityAlgorithmV2::PlsPosPublishedV2,
                ],
            },
            seed,
            fimix: FimixSettingsV2::default(),
            pls_pos: PlsPosSettingsV2::default(),
            pos_common_metric: None,
            bootstrap: None,
        };
        config.ensure_valid()?;
        create_archive(
            root,
            "heterogeneity",
            Uuid::from_u128(0x6d75_6c74_696d_6f64_706b_6700_1000_0002),
            dataset,
            recipe,
            model,
            serde_json::to_value(config)?,
            "qpls.multimod.pls_heterogeneity_v2",
            false,
        )
    }

    fn conditional_fixture(root: &Path, seed: u64) -> Result<FamilyFixtureV1, FixtureError> {
        let rows = 140usize;
        let mut x = Vec::new();
        let mut z = Vec::new();
        let mut m = Vec::new();
        let mut y = Vec::new();
        for row in 0..rows {
            let t = row as f64 + 1.0;
            let xv = (t * 0.151).sin() + 0.35 * (t * 0.067).cos();
            let zv = (t * 0.193).cos() - 0.22 * (t * 0.089).sin();
            let mv = 0.72 * xv + 0.28 * zv + 0.46 * xv * zv + 0.09 * (t * 0.337).sin();
            let yv = 0.64 * mv + 0.18 * xv + 0.08 * (t * 0.281).cos();
            x.push(xv);
            z.push(zv);
            m.push(mv);
            y.push(yv);
        }
        let dataset = dataset_from_columns(
            "multimod-packaged-conditional.csv",
            &["x1", "x2", "z1", "z2", "m1", "m2", "y1", "y2"],
            &[
                numeric(
                    x.iter()
                        .enumerate()
                        .map(|(i, value)| value + 0.02 * ((i + 1) as f64 * 0.41).sin()),
                ),
                numeric(
                    x.iter()
                        .enumerate()
                        .map(|(i, value)| 0.94 * value + 0.03 * ((i + 1) as f64 * 0.29).cos()),
                ),
                numeric(
                    z.iter()
                        .enumerate()
                        .map(|(i, value)| value + 0.02 * ((i + 1) as f64 * 0.37).sin()),
                ),
                numeric(
                    z.iter()
                        .enumerate()
                        .map(|(i, value)| 0.93 * value + 0.03 * ((i + 1) as f64 * 0.23).cos()),
                ),
                numeric(
                    m.iter()
                        .enumerate()
                        .map(|(i, value)| value + 0.02 * ((i + 1) as f64 * 0.31).sin()),
                ),
                numeric(
                    m.iter()
                        .enumerate()
                        .map(|(i, value)| 0.95 * value + 0.03 * ((i + 1) as f64 * 0.17).cos()),
                ),
                numeric(
                    y.iter()
                        .enumerate()
                        .map(|(i, value)| value + 0.02 * ((i + 1) as f64 * 0.43).sin()),
                ),
                numeric(
                    y.iter()
                        .enumerate()
                        .map(|(i, value)| 0.92 * value + 0.03 * ((i + 1) as f64 * 0.13).cos()),
                ),
            ],
        )?;
        let (recipe, mut model) = base_recipe_model(
            &dataset,
            0x6d75_6c74_696d_6f64_706b_6700_0000_0003,
            "packaged-conditional-model",
            &[
                ("x", &["x1", "x2"]),
                ("z", &["z1", "z2"]),
                ("m", &["m1", "m2"]),
                ("y", &["y1", "y2"]),
            ],
            &[("x", "m"), ("z", "m"), ("m", "y"), ("x", "y")],
            seed,
        )?;
        add_interaction(&mut model, "int:x:z:m", &["x", "z"], "x", "m")?;
        let config = GeneralSemConditionalProcessConfigV2 {
            schema_version: 2,
            profile: ConditionalProcessProfileV2::MultiTwoWayPercentile,
            paths: vec![ConditionalProcessPathV2 {
                path_id: "x_m_y".into(),
                ordered_relation_ids: vec![
                    relation_id(&model, "x", "m")?,
                    relation_id(&model, "m", "y")?,
                ],
            }],
            declared_interaction_ids: vec!["int:x:z:m".into()],
            three_way_interaction_id: None,
            hoc_ids: Vec::new(),
            moderator_ids: vec!["z".into()],
            probes: vec![ConditionalModeratorProbeV2 {
                probe_id: "probe:z".into(),
                moderator_id: "z".into(),
                scale: ConditionalProbeScaleV2::StandardizedScore,
                values: vec![-1.0, 0.0, 1.0],
                raw_transformation_receipt: None,
                raw_fit_metric_receipts: Vec::new(),
            }],
            explicit_joint_tuples: Vec::new(),
            probe_contrasts: Vec::new(),
            grouping_column: None,
            groups: Vec::new(),
            group_contrasts: Vec::new(),
            weight: None,
            estimands: ConditionalProcessEstimandsV2 {
                conditional_specific_indirect: true,
                conditional_total_indirect: false,
                conditional_total_effect: false,
                scalar_index_when_affine: true,
                local_first_derivatives: true,
                local_second_and_cross_derivatives: false,
                finite_probe_contrasts: false,
            },
            inference: ConditionalProcessInferenceV2 {
                interval: ConditionalProcessIntervalV2::Percentile,
                alternative: InferenceAlternativeV1::TwoSided,
                outer_resamples: 500,
                inner_resamples: 0,
                seed,
                confidence_level: 0.95,
            },
        };
        config.ensure_valid()?;
        create_archive(
            root,
            "conditional",
            Uuid::from_u128(0x6d75_6c74_696d_6f64_706b_6700_1000_0003),
            dataset,
            recipe,
            model,
            serde_json::to_value(config)?,
            "qpls.multimod.general_sem_conditional_process_v2",
            false,
        )
    }

    fn observed(id: &str, scale: ObservedScaleV4) -> SemVariableV4 {
        SemVariableV4::Observed {
            id: id.into(),
            label: id.to_uppercase(),
            source_column: id.into(),
            scale,
            role: ObservedRoleV4::Structural,
            categories: if scale == ObservedScaleV4::Binary {
                vec!["0".into(), "1".into()]
            } else {
                Vec::new()
            },
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        }
    }

    fn causal_relation(source: &str, target: &str) -> (SemRelationV4, SemParameterV4) {
        let relation_id = format!("relation:{source}:{target}");
        let parameter_id = format!("parameter:{source}:{target}");
        (
            SemRelationV4::Structural {
                id: relation_id,
                source: source.into(),
                target: target.into(),
                parameter: parameter_id.clone(),
                role: StructuralRelationRoleV4::Structural,
                intercept_parameter: None,
            },
            SemParameterV4::Free {
                id: parameter_id,
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
            },
        )
    }

    fn causal_equation(id: &str, outcome: &str, factors: &[&str]) -> CausalLinearEquationV1 {
        CausalLinearEquationV1 {
            equation_id: id.into(),
            outcome_variable_id: outcome.into(),
            terms: factors
                .iter()
                .map(|factor| CausalLinearTermV1 {
                    term_id: format!("main:{factor}"),
                    factor_variable_ids: vec![(*factor).into()],
                })
                .collect(),
        }
    }

    fn causal_fixture(root: &Path, seed: u64) -> Result<FamilyFixtureV1, FixtureError> {
        let rows = 96usize;
        let x = (0..rows).map(|row| (row % 2) as f64).collect::<Vec<_>>();
        let z = (0..rows)
            .map(|row| ((row / 2) % 2) as f64)
            .collect::<Vec<_>>();
        let c = (0..rows)
            .map(|row| ((row * 7 + 3) % 31) as f64 / 10.0 - 1.5)
            .collect::<Vec<_>>();
        let m = (0..rows)
            .map(|row| {
                0.4 + 0.82 * x[row]
                    + 0.27 * c[row]
                    + 0.18 * z[row]
                    + 0.04 * ((row + 1) as f64 * 0.31).sin()
            })
            .collect::<Vec<_>>();
        let y = (0..rows)
            .map(|row| {
                0.5 + 0.31 * x[row]
                    + 0.58 * m[row]
                    + 0.21 * c[row]
                    + 0.12 * z[row]
                    + 0.04 * ((row + 1) as f64 * 0.23).cos()
            })
            .collect::<Vec<_>>();
        let dataset = dataset_from_columns(
            "multimod-packaged-causal.csv",
            &["x", "c", "z", "m", "y"],
            &[numeric(x), numeric(c), numeric(z), numeric(m), numeric(y)],
        )?;
        let variables = [
            ("x", ObservedScaleV4::Binary),
            ("c", ObservedScaleV4::Continuous),
            ("z", ObservedScaleV4::Binary),
            ("m", ObservedScaleV4::Continuous),
            ("y", ObservedScaleV4::Continuous),
        ];
        let (relations, parameters): (Vec<_>, Vec<_>) = [("x", "m"), ("m", "y"), ("x", "y")]
            .into_iter()
            .map(|(source, target)| causal_relation(source, target))
            .unzip();
        let model = SemModelV4 {
            schema_version: SEM_MODEL_V4_SCHEMA_VERSION,
            id: "packaged-causal-model".into(),
            name: "Packaged causal model".into(),
            variables: variables
                .into_iter()
                .map(|(id, scale)| observed(id, scale))
                .collect(),
            relations,
            parameters,
            constraints: Vec::new(),
            derived_terms: Vec::new(),
            group: SemGroupV4::SingleGroup,
            data_binding: SemDataBindingV4::Raw {
                dataset_id: dataset.id.to_string(),
                missing_data: MissingDataPolicyV4::ListwiseDeletion,
                weight: None,
                cluster_variable: None,
                strata_variable: None,
            },
            annotations: Vec::new(),
            presentation: SemPresentationV4::default(),
        };
        let config = InterventionalCausalMediationConfigV1 {
            schema_version: 1,
            treatment: "x".into(),
            treatment_contrast: ObservedTreatmentContrastV1::Binary {
                control: 0.0,
                treated: 1.0,
            },
            outcome: "y".into(),
            mediators: vec!["m".into()],
            baseline_moderators: vec!["z".into()],
            adjustment_covariates: vec!["c".into()],
            paths: vec![ObservedCausalPathV1 {
                path_id: "x_m_y".into(),
                ordered_variable_ids: vec!["x".into(), "m".into(), "y".into()],
                equations: vec![
                    causal_equation("x_m_y:m", "m", &["x", "c", "z"]),
                    causal_equation("x_m_y:y", "y", &["x", "m", "c", "z"]),
                ],
            }],
            positivity_policy: CausalPositivityPolicyV1 {
                minimum_binary_arm_count: 10,
                maximum_binary_arm_ratio: 10.0,
                positivity_strata_variable_ids: vec!["z".into()],
                minimum_count_per_binary_stratum_arm: 5,
                ..CausalPositivityPolicyV1::default()
            },
            identification: CausalIdentificationChecklistV1 {
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
            },
            bootstrap_resamples: 500,
            seed,
            confidence_level: 0.95,
        };
        config.ensure_valid()?;
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Regression;
        settings.seed = seed;
        settings.workers = 1;
        settings.bootstrap_samples = 0;
        settings.permutation_samples = 0;
        let recipe = AnalysisRecipeV4 {
            schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: Uuid::from_u128(0x6d75_6c74_696d_6f64_706b_6700_0000_0004),
            created_at: Utc
                .timestamp_opt(1_700_000_000, 0)
                .single()
                .ok_or_else(|| invalid("fixed recipe timestamp is invalid"))?,
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                model_id: model.id.clone(),
                scientific_sha256: model.scientific_sha256()?,
            },
            estimand_confirmation: LegacyEstimandConfirmationV4::NotLegacy,
            settings,
            method_config: None,
            general_sem_config: Some(GeneralSemConfigV1::default()),
            mga_multigroup: None,
            pls_heterogeneity: None,
            general_sem_conditional_process: None,
            interventional_causal_mediation: None,
            metadata: BTreeMap::new(),
            legacy_source: None,
        };
        create_archive(
            root,
            "causal",
            Uuid::from_u128(0x6d75_6c74_696d_6f64_706b_6700_1000_0004),
            dataset,
            recipe,
            model,
            serde_json::to_value(config)?,
            "qpls.multimod.interventional_causal_mediation_v1",
            false,
        )
    }

    pub(super) fn prepare(request: PrepareRequestV1) -> Result<PreparedFixturesV1, String> {
        (|| -> Result<PreparedFixturesV1, FixtureError> {
            require_enabled_surface(&request.surface, request.experimental_labs_enabled)?;
            let output = PathBuf::from(&request.output_directory);
            if !output.is_absolute() || output.exists() || output.extension().is_some() {
                return Err(invalid("fixture output must be a new absolute directory"));
            }
            let parent = output
                .parent()
                .ok_or_else(|| invalid("fixture output has no parent"))?;
            let parent_metadata = fs::symlink_metadata(parent)?;
            if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
                return Err(invalid(
                    "fixture output parent must be a regular local directory",
                ));
            }
            fs::create_dir(&output)?;
            let canonical_output = fs::canonicalize(&output)?;
            let families = vec![
                mga_fixture(&canonical_output, request.seed)?,
                heterogeneity_fixture(&canonical_output, request.seed)?,
                conditional_fixture(&canonical_output, request.seed)?,
                causal_fixture(&canonical_output, request.seed)?,
            ];
            Ok(PreparedFixturesV1 {
                schema_version: 1,
                fixture_id: "qpls.v256.multimod.packaged-production-fixtures.v1",
                surface: SURFACE,
                seed: request.seed,
                output_directory: canonical_output.to_string_lossy().into_owned(),
                families,
                production_lifecycle_required: vec![
                    "preflight_internal_labs_multimod_v1",
                    "start_internal_labs_multimod_job_v1",
                    "status_internal_labs_multimod_job_v1",
                    "cancel_internal_labs_multimod_job_v1",
                    "result_internal_labs_multimod_job_v1",
                    "inspect_internal_project_archive_v6_zip",
                    "save_internal_project_archive_v6_copy",
                    "publish_canonical_result_export_v2",
                    "publish_internal_labs_multimod_raw_sidecar_v1",
                ],
            })
        })()
        .map_err(|error| error.to_string())
    }

    fn rewrite_archive_variant(
        source: &Path,
        destination: &Path,
        selected_sidecar: &str,
        omit: bool,
    ) -> Result<(), FixtureError> {
        let source_file = File::open(source)?;
        let mut reader = ZipArchive::new(source_file)?;
        let destination_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        let mut writer = ZipWriter::new(destination_file);
        for index in 0..reader.len() {
            let mut entry = reader.by_index(index)?;
            let name = entry.name().to_owned();
            if entry.enclosed_name().is_none() {
                return Err(invalid("source archive contains an unsafe entry name"));
            }
            if name == selected_sidecar && omit {
                continue;
            }
            let options = SimpleFileOptions::default().compression_method(entry.compression());
            if entry.is_dir() {
                writer.add_directory(name, options)?;
                continue;
            }
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            if name == selected_sidecar {
                let index = bytes.len() / 2;
                let value = bytes
                    .get_mut(index)
                    .ok_or_else(|| invalid("selected sidecar is empty"))?;
                *value ^= 0x01;
            }
            writer.start_file(name, options)?;
            writer.write_all(&bytes)?;
        }
        writer.finish()?.sync_all()?;
        Ok(())
    }

    pub(super) fn integrity_variants(
        request: IntegrityRequestV1,
    ) -> Result<IntegrityVariantsV1, String> {
        (|| -> Result<IntegrityVariantsV1, FixtureError> {
            require_enabled_surface(&request.surface, request.experimental_labs_enabled)?;
            if request.result_id.is_empty() || request.result_id.trim() != request.result_id {
                return Err(invalid("resultId must be nonempty exact text"));
            }
            let root = fs::canonicalize(PathBuf::from(&request.fixture_root))?;
            let archive = fs::canonicalize(PathBuf::from(&request.archive_path))?;
            if archive.parent() != Some(root.as_path())
                || archive.extension().and_then(|value| value.to_str()) != Some("qpls")
            {
                return Err(invalid(
                    "integrity variants are restricted to one prepared fixture archive",
                ));
            }
            let observed_sha = file_sha256(&archive)?;
            if observed_sha != request.expected_archive_sha256 {
                return Err(invalid(
                    "fixture archive digest changed before integrity mutation",
                ));
            }
            let loaded = load_project_archive_v6(&archive)?;
            let attachment = loaded
                .document
                .multimod_results
                .iter()
                .find(|attachment| attachment.result_id == request.result_id)
                .ok_or_else(|| invalid("completed result is absent from fixture archive"))?;
            let sidecar = attachment
                .sidecars
                .first()
                .ok_or_else(|| invalid("completed result has no integrity-bound sidecar"))?
                .entry_name
                .clone();
            let missing = root.join(format!("{}-missing-sidecar.qpls", request.result_id));
            let tampered = root.join(format!("{}-tampered-sidecar.qpls", request.result_id));
            rewrite_archive_variant(&archive, &missing, &sidecar, true)?;
            rewrite_archive_variant(&archive, &tampered, &sidecar, false)?;
            let missing_rejected = load_project_archive_v6(&missing).is_err();
            let tamper_rejected = load_project_archive_v6(&tampered).is_err();
            if !missing_rejected || !tamper_rejected {
                return Err(invalid(
                    "production strict reopen accepted a missing or tampered sidecar",
                ));
            }
            Ok(IntegrityVariantsV1 {
                schema_version: 1,
                receipt_id: "qpls.v256.multimod.packaged-sidecar-integrity-variants.v1",
                source_archive_sha256: observed_sha,
                sidecar_entry: sidecar,
                missing_sidecar_archive_path: missing.to_string_lossy().into_owned(),
                missing_sidecar_archive_sha256: file_sha256(&missing)?,
                tampered_sidecar_archive_path: tampered.to_string_lossy().into_owned(),
                tampered_sidecar_archive_sha256: file_sha256(&tampered)?,
                production_strict_reopen_rejected_missing: true,
                production_strict_reopen_rejected_tamper: true,
            })
        })()
        .map_err(|error| error.to_string())
    }
}

#[tauri::command]
pub(crate) fn prepare_multimod_packaged_qualification_fixtures_v1(
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    #[cfg(feature = "multimod-qualification-harness")]
    {
        let request = serde_json::from_value::<enabled::PrepareRequestV1>(request)
            .map_err(|error| format!("invalid packaged qualification fixture request: {error}"))?;
        return serde_json::to_value(enabled::prepare(request)?)
            .map_err(|error| format!("fixture receipt serialization failed: {error}"));
    }
    #[cfg(not(feature = "multimod-qualification-harness"))]
    {
        let _ = request;
        Err(
            serde_json::to_string(&MultiModPackagedQualificationUnavailableV1 {
                schema_version: 1,
                code: "multimod.qualification_fixture.build_feature_absent",
                message: "This preview build contains no packaged MultiMod qualification fixtures.",
            })
            .unwrap_or_else(|_| "packaged MultiMod qualification fixtures are unavailable".into()),
        )
    }
}

#[tauri::command]
pub(crate) fn prepare_multimod_packaged_integrity_variants_v1(
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    #[cfg(feature = "multimod-qualification-harness")]
    {
        let request = serde_json::from_value::<enabled::IntegrityRequestV1>(request)
            .map_err(|error| format!("invalid packaged integrity request: {error}"))?;
        return serde_json::to_value(enabled::integrity_variants(request)?)
            .map_err(|error| format!("integrity receipt serialization failed: {error}"));
    }
    #[cfg(not(feature = "multimod-qualification-harness"))]
    {
        let _ = request;
        Err(serde_json::to_string(&MultiModPackagedQualificationUnavailableV1 {
            schema_version: 1,
            code: "multimod.qualification_fixture.build_feature_absent",
            message: "This preview build contains no packaged MultiMod qualification integrity seam.",
        })
        .unwrap_or_else(|_| "packaged MultiMod qualification integrity seam is unavailable".into()))
    }
}
