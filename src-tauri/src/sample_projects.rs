use chrono::{DateTime, Utc};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisRecipe, AnalysisSettings, Construct, MeasurementMode,
    MethodConfig, MissingDataPolicy, ModelSpec, Preprocessing, StructuralPath, WeightingScheme,
};
use qpls_data::{ImportOptions, import_delimited_bytes};
use qpls_project::Project;
use qpls_runner::run_pls_analysis;
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BundledSampleProject {
    CorporateReputation,
    OrganizationalIdentification,
    SimplePls,
    Mediation,
}

impl BundledSampleProject {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "corporate_reputation" => Ok(Self::CorporateReputation),
            "organizational_identification" => Ok(Self::OrganizationalIdentification),
            "simple_pls" => Ok(Self::SimplePls),
            "mediation" => Ok(Self::Mediation),
            other => Err(format!(
                "unknown bundled sample project {other:?}; expected corporate_reputation, organizational_identification, simple_pls, or mediation"
            )),
        }
    }

    fn identity(self) -> SampleIdentity {
        match self {
            Self::CorporateReputation => SampleIdentity {
                project_name: "Corporate Reputation Sample",
                dataset_name: "corporate_reputation_smartpls_mean_replaced_v1.csv",
                dataset_bytes: include_bytes!(
                    "../../validation/fixtures/corporate_reputation_smartpls_mean_replaced_v1.csv"
                ),
                model_id: "00000000-0000-0000-0000-000000003201",
                recipe_id: "00000000-0000-0000-0000-000000003202",
                created_at: "2026-08-23T00:00:00Z",
                sample_version: "quickpls_sample_corporate_reputation_smartpls_v1",
            },
            Self::OrganizationalIdentification => SampleIdentity {
                project_name: "Organizational Identification Model",
                dataset_name: "organizational_identification_v1.csv",
                dataset_bytes: include_bytes!(
                    "../../validation/fixtures/organizational_identification_v1.csv"
                ),
                model_id: "00000000-0000-0000-0000-000000003301",
                recipe_id: "00000000-0000-0000-0000-000000003302",
                created_at: "2026-08-23T00:00:00Z",
                sample_version: "quickpls_sample_organizational_identification_v1",
            },
            Self::SimplePls => SampleIdentity {
                project_name: "Simple Reflective PLS Sample",
                dataset_name: "simple_reflective.csv",
                dataset_bytes: include_bytes!("../../validation/fixtures/simple_reflective.csv"),
                model_id: "00000000-0000-0000-0000-000000003001",
                recipe_id: "00000000-0000-0000-0000-000000003002",
                created_at: "2026-08-13T00:00:00Z",
                sample_version: "quickpls_sample_simple_pls_v1",
            },
            Self::Mediation => SampleIdentity {
                project_name: "Mediation Sample",
                dataset_name: "mediation_sample.csv",
                dataset_bytes: include_bytes!("../../validation/fixtures/mediation_sample.csv"),
                model_id: "00000000-0000-0000-0000-000000003101",
                recipe_id: "00000000-0000-0000-0000-000000003102",
                created_at: "2026-08-13T00:00:00Z",
                sample_version: "quickpls_sample_mediation_v1",
            },
        }
    }
}

struct SampleIdentity {
    project_name: &'static str,
    dataset_name: &'static str,
    dataset_bytes: &'static [u8],
    model_id: &'static str,
    recipe_id: &'static str,
    created_at: &'static str,
    sample_version: &'static str,
}

pub(crate) fn build_bundled_sample_project(
    sample: BundledSampleProject,
) -> Result<Project, String> {
    let identity = sample.identity();
    let dataset = import_delimited_bytes(
        identity.dataset_bytes,
        identity.dataset_name,
        b',',
        &ImportOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let model = sample_model(sample, identity.model_id)?;
    let settings = sample_settings(sample);
    let mut metadata = BTreeMap::from([
        ("sample_project".into(), identity.sample_version.into()),
        (
            "fixture".into(),
            format!("validation/fixtures/{}", identity.dataset_name),
        ),
    ]);
    if sample == BundledSampleProject::CorporateReputation {
        metadata.extend([
            (
                "preprocessing_receipt".into(),
                "validation/fixtures/corporate_reputation_smartpls_mean_replaced_v1.provenance.json"
                    .into(),
            ),
            (
                "preprocessing_receipt_sha256".into(),
                "eff5124d1a7589aee4a7cf580c16de93a2fefebb6fc4b03d9efa60a8768f009b".into(),
            ),
            (
                "source_workbook_sha256".into(),
                "45373b5177b19352d146c0a3b9bc66d58255744b795e453974d1248f816db9cb".into(),
            ),
            (
                "cleaned_csv_sha256".into(),
                "2f2b369d74086d8d9646b052fec98991fbb7b870169f106f175c320ace9b97cb".into(),
            ),
            ("source_missing_marker".into(), "-99".into()),
            (
                "source_missing_treatment".into(),
                "external_indicator_mean_replacement_before_quickpls_import".into(),
            ),
            (
                "reference_scope".into(),
                "matches_48_published_smartpls_values_at_three_decimals".into(),
            ),
            (
                "smartpls_reference_sha256".into(),
                "730c96d062b1724fcd2405da1116d6d4fb34162c02abbb72f5dd51380814f441"
                    .into(),
            ),
            (
                "smartpls_screenshot_sha256".into(),
                "2d604f98aaeb618469486be3bfea55cc57d1901ad49cab8b5d97dd17504f9ed9"
                    .into(),
            ),
            ("reference_precision_decimals".into(), "3".into()),
            (
                "evidence_boundary".into(),
                "displayed_three_decimal_parity_not_bitwise_equivalence".into(),
            ),
        ]);
    } else if sample == BundledSampleProject::OrganizationalIdentification {
        metadata.extend([
            (
                "preprocessing_receipt".into(),
                "validation/fixtures/organizational_identification_v1.provenance.json".into(),
            ),
            (
                "preprocessing_receipt_sha256".into(),
                "98f13a5c7b59cbf1ff54fbda218ee0d2b9423a07d87b0d2a9f3e765cdd930b5b".into(),
            ),
            (
                "source_workbook_sha256".into(),
                "5d803952b8009d406ab2f6317527d3df646de083ea86aa0b534a65b339713ae7".into(),
            ),
            (
                "bundled_csv_sha256".into(),
                "5066d3b4bd24d14ad5d3efc91c1c40c57c41de63d987456d2cf8aad40c20ceed".into(),
            ),
            ("source_missing_marker".into(), "none_observed".into()),
            (
                "source_missing_treatment".into(),
                "none_required_all_305_cases_complete".into(),
            ),
            (
                "reference_scope".into(),
                "matches_27_supplied_screenshot_values_at_three_decimals".into(),
            ),
            (
                "smartpls_reference".into(),
                "validation/benchmarks/organizational_identification/screenshot_reference_v1.json"
                    .into(),
            ),
            (
                "smartpls_reference_sha256".into(),
                "9c1c6848f055ec2f3c00b1d3fe81ddb9fc973a4cc279b0399ffa54ca339af4fa".into(),
            ),
            (
                "smartpls_screenshot_sha256".into(),
                "6809e7f76ed209f8f83dfc2c16fa057c1f5d6e5f09633a9507eb3dd278e83544".into(),
            ),
            ("reference_precision_decimals".into(), "3".into()),
            (
                "evidence_boundary".into(),
                "displayed_three_decimal_parity_not_bitwise_equivalence".into(),
            ),
        ]);
    }
    let recipe = AnalysisRecipe {
        schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        id: parse_uuid(identity.recipe_id)?,
        created_at: parse_time(identity.created_at)?,
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model: model.clone(),
        settings: settings.clone(),
        method_config: Some(MethodConfig::PlsAlgorithm),
        metadata,
    };
    let result =
        run_pls_analysis(&dataset, &recipe, || false, |_| {}).map_err(|error| error.to_string())?;
    let dataset_id = dataset.id.to_string();
    let model_id = model.id.to_string();
    let recipe_id = recipe.id.to_string();
    let result_id = result.id.to_string();
    let (nodes, edges) = sample_presentation(sample, &model);
    let lineage_summary = match sample {
        BundledSampleProject::CorporateReputation => "Bundled Corporate Reputation sample; -99 missing values were replaced by indicator means before import".to_owned(),
        BundledSampleProject::OrganizationalIdentification => "Bundled Organizational Identification sample; all 305 source cases were complete and no value replacement was required".to_owned(),
        _ => format!("Bundled {}", identity.project_name),
    };
    let mut project = Project::new(identity.project_name);
    project.datasets.push(dataset);
    project.models.push(model);
    project
        .append_validated_result(recipe, result)
        .map_err(|error| error.to_string())?;
    project.layouts.insert(
        "workspace".into(),
        json!({
            "activeDatasetId": dataset_id,
            "activeModelId": model_id,
            "activeRecipeId": recipe_id,
            "activeRunId": result_id,
            "selectedRunId": result_id,
            "nodes": nodes,
            "edges": edges,
            "analysisSettings": {
                "method": "pls_pm",
                "weightingScheme": "path",
                "tolerance": settings.tolerance,
                "maxIterations": settings.max_iterations,
                "bootstrapSamples": 0,
                "studentizedInnerSamples": 0,
                "permutationSamples": 0,
                "seed": settings.seed,
                "workers": 1,
                "confidenceLevel": settings.confidence_level,
                "preprocessing": "standardized",
                "missingData": "listwise_deletion"
            },
            "diagramMode": "sem",
            "diagramOverlaySettings": {"selectedRunId": result_id, "mode": "paths_r2"}
        }),
    );
    project.layouts.insert(
        "data_lineage".into(),
        json!({
            "schemaVersion": 1,
            "records": [{
                "datasetId": dataset_id,
                "parentDatasetId": null,
                "operation": "import",
                "createdAt": null,
                "summary": lineage_summary,
                "sourceColumn": null,
                "targetColumn": null
            }]
        }),
    );
    Ok(project)
}

fn sample_settings(sample: BundledSampleProject) -> AnalysisSettings {
    let mut settings = AnalysisSettings::default();
    if matches!(
        sample,
        BundledSampleProject::CorporateReputation
            | BundledSampleProject::OrganizationalIdentification
    ) {
        settings.method = qpls_core::AnalysisMethod::PlsPm;
        settings.weighting_scheme = WeightingScheme::Path;
        settings.preprocessing = Preprocessing::Standardized;
        settings.missing_data = MissingDataPolicy::ListwiseDeletion;
        settings.tolerance = 1e-7;
        settings.max_iterations = 3_000;
        settings.bootstrap_samples = 0;
        settings.studentized_inner_samples = 0;
        settings.permutation_samples = 0;
        settings.seed = 20_260_823;
        settings.workers = 1;
        settings.confidence_level = 0.95;
        settings.case_weight_column = None;
    }
    settings
}

fn sample_model(sample: BundledSampleProject, id: &str) -> Result<ModelSpec, String> {
    let (name, constructs, paths) = match sample {
        BundledSampleProject::CorporateReputation => (
            "Corporate reputation - full SmartPLS comparison model",
            vec![
                construct_with_mode(
                    "qual",
                    "Quality",
                    "QUAL",
                    MeasurementMode::Formative,
                    &[
                        "qual_1", "qual_2", "qual_3", "qual_4", "qual_5", "qual_6", "qual_7",
                        "qual_8",
                    ],
                ),
                construct_with_mode(
                    "perf",
                    "Performance",
                    "PERF",
                    MeasurementMode::Formative,
                    &["perf_1", "perf_2", "perf_3", "perf_4", "perf_5"],
                ),
                construct_with_mode(
                    "csor",
                    "Corporate social responsibility",
                    "CSOR",
                    MeasurementMode::Formative,
                    &["csor_1", "csor_2", "csor_3", "csor_4", "csor_5"],
                ),
                construct_with_mode(
                    "attr",
                    "Attractiveness",
                    "ATTR",
                    MeasurementMode::Formative,
                    &["attr_1", "attr_2", "attr_3"],
                ),
                construct(
                    "comp",
                    "Competence",
                    "COMP",
                    &["comp_1", "comp_2", "comp_3"],
                ),
                construct(
                    "like",
                    "Likeability",
                    "LIKE",
                    &["like_1", "like_2", "like_3"],
                ),
                construct("cusa", "Customer satisfaction", "CUSA", &["cusa"]),
                construct(
                    "cusl",
                    "Customer loyalty",
                    "CUSL",
                    &["cusl_1", "cusl_2", "cusl_3"],
                ),
            ],
            vec![
                path("attr", "comp"),
                path("attr", "like"),
                path("csor", "comp"),
                path("csor", "like"),
                path("perf", "comp"),
                path("perf", "like"),
                path("qual", "comp"),
                path("qual", "like"),
                path("comp", "cusa"),
                path("comp", "cusl"),
                path("like", "cusa"),
                path("like", "cusl"),
                path("cusa", "cusl"),
            ],
        ),
        BundledSampleProject::OrganizationalIdentification => (
            "Organizational Identification Model",
            vec![
                construct(
                    "org_prestige",
                    "Organizational Prestige",
                    "ORG_PRE",
                    &[
                        "org_pre1", "org_pre2", "org_pre3", "org_pre4", "org_pre5", "org_pre6",
                        "org_pre7", "org_pre8",
                    ],
                ),
                construct(
                    "org_identification",
                    "Organizational Identification",
                    "ORG_IDENT",
                    &[
                        "org_ident1",
                        "org_ident2",
                        "org_ident3",
                        "org_ident4",
                        "org_ident5",
                        "org_ident6",
                    ],
                ),
                construct(
                    "affective_commitment_joy",
                    "Affective Commitment (Joy)",
                    "AC_JOY",
                    &["ac_joy1", "ac_joy2", "ac_joy3", "ac_joy4"],
                ),
                construct(
                    "affective_commitment_love",
                    "Affective Commitment (Love)",
                    "AC_LOVE",
                    &["ac_love1", "ac_love2", "ac_love3"],
                ),
            ],
            vec![
                path("org_prestige", "org_identification"),
                path("org_identification", "affective_commitment_joy"),
                path("org_identification", "affective_commitment_love"),
            ],
        ),
        BundledSampleProject::SimplePls => (
            "Simple reflective PLS model",
            vec![
                construct("x", "Predictor", "X", &["x1", "x2"]),
                construct("y", "Outcome", "Y", &["y1", "y2"]),
            ],
            vec![path("x", "y")],
        ),
        BundledSampleProject::Mediation => (
            "Three-construct mediation model",
            vec![
                construct("x", "Predictor", "X", &["x"]),
                construct("m", "Mediator", "M", &["m"]),
                construct("y", "Outcome", "Y", &["y"]),
            ],
            vec![path("x", "m"), path("m", "y")],
        ),
    };
    Ok(ModelSpec {
        id: parse_uuid(id)?,
        name: name.into(),
        constructs,
        paths,
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    })
}

fn construct(id: &str, name: &str, short_name: &str, indicators: &[&str]) -> Construct {
    construct_with_mode(
        id,
        name,
        short_name,
        MeasurementMode::Reflective,
        indicators,
    )
}

fn construct_with_mode(
    id: &str,
    name: &str,
    short_name: &str,
    mode: MeasurementMode,
    indicators: &[&str],
) -> Construct {
    Construct {
        id: id.into(),
        name: name.into(),
        short_name: short_name.into(),
        mode,
        indicators: indicators.iter().map(|value| (*value).into()).collect(),
    }
}

fn sample_presentation(
    sample: BundledSampleProject,
    model: &ModelSpec,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let (positions, edge_type) = match sample {
        BundledSampleProject::CorporateReputation => (
            BTreeMap::from([
                ("perf", (80, 260)),
                ("csor", (80, 610)),
                ("qual", (430, 80)),
                ("attr", (430, 860)),
                ("comp", (850, 280)),
                ("like", (850, 650)),
                ("cusa", (1_210, 460)),
                ("cusl", (1_570, 460)),
            ]),
            "smoothstep",
        ),
        BundledSampleProject::OrganizationalIdentification => (
            BTreeMap::from([
                ("org_prestige", (140, 430)),
                ("org_identification", (720, 430)),
                ("affective_commitment_joy", (1_280, 250)),
                ("affective_commitment_love", (1_280, 650)),
            ]),
            "straight",
        ),
        _ => return (Vec::new(), Vec::new()),
    };
    let nodes = model
        .constructs
        .iter()
        .map(|construct| {
            let (x, y) = positions[construct.id.as_str()];
            json!({
                "id": construct.id,
                "type": "construct",
                "position": {"x": x, "y": y},
                "data": {
                    "label": construct.name,
                    "shortName": construct.short_name,
                    "mode": match construct.mode {
                        MeasurementMode::Reflective => "reflective",
                        MeasurementMode::Formative => "formative",
                    },
                    "indicators": construct.indicators
                }
            })
        })
        .collect();
    let edges = model
        .paths
        .iter()
        .map(|path| {
            json!({
                "id": format!("path-{}-{}", path.source, path.target),
                "source": path.source,
                "target": path.target,
                "type": edge_type,
                "label": "Path",
                "markerEnd": {"type": "arrowclosed", "width": 16, "height": 16}
            })
        })
        .collect();
    (nodes, edges)
}

fn path(source: &str, target: &str) -> StructuralPath {
    StructuralPath {
        source: source.into(),
        target: target.into(),
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, String> {
    value
        .parse()
        .map_err(|error| format!("invalid bundled sample UUID: {error}"))
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| format!("invalid bundled sample timestamp: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{RunStatus, Severity, validate_recipe};
    use qpls_estimation::PlsResult;
    use qpls_project::{load_project, save_project};
    use sha2::{Digest, Sha256};

    #[test]
    fn bundled_samples_are_distinct_complete_and_round_trip() {
        for (sample, expected_name, expected_constructs, expected_paths) in [
            (
                BundledSampleProject::CorporateReputation,
                "Corporate Reputation Sample",
                8,
                13,
            ),
            (
                BundledSampleProject::OrganizationalIdentification,
                "Organizational Identification Model",
                4,
                3,
            ),
            (
                BundledSampleProject::SimplePls,
                "Simple Reflective PLS Sample",
                2,
                1,
            ),
            (BundledSampleProject::Mediation, "Mediation Sample", 3, 2),
        ] {
            let project = build_bundled_sample_project(sample).unwrap();
            assert_eq!(project.manifest.name, expected_name);
            assert_eq!(project.datasets.len(), 1);
            assert_eq!(project.models.len(), 1);
            assert_eq!(project.models[0].constructs.len(), expected_constructs);
            assert_eq!(project.models[0].paths.len(), expected_paths);
            assert_eq!(project.recipes.len(), 1);
            assert_eq!(project.results.len(), 1);
            assert_eq!(
                project.recipes[0].method_config,
                Some(MethodConfig::PlsAlgorithm)
            );
            assert_eq!(project.recipes[0].settings.bootstrap_samples, 0);
            assert!(project.layouts["workspace"]["activeDatasetId"].is_string());
            assert_eq!(
                project.layouts["workspace"]["activeModelId"],
                project.models[0].id.to_string()
            );

            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("sample.qpls");
            save_project(&path, &project).unwrap();
            let reopened = load_project(&path).unwrap();
            assert_eq!(reopened.manifest.name, expected_name);
            assert_eq!(reopened.models, project.models);
            assert_eq!(reopened.recipes, project.recipes);
            assert_eq!(reopened.results.len(), 1);
            assert_eq!(reopened.results[0].id, project.results[0].id);
            assert_eq!(
                reopened.results[0].provenance.recipe_id,
                reopened.recipes[0].id
            );
            assert_eq!(
                reopened.results[0].provenance.dataset_fingerprint,
                reopened.datasets[0].fingerprint.0
            );
            assert_eq!(
                std::mem::discriminant(&reopened.results[0].payload),
                std::mem::discriminant(&project.results[0].payload)
            );
        }
    }

    #[test]
    fn corporate_reputation_sample_matches_the_published_three_decimal_reference() {
        let project =
            build_bundled_sample_project(BundledSampleProject::CorporateReputation).unwrap();
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(
                    BundledSampleProject::CorporateReputation
                        .identity()
                        .dataset_bytes
                )
            ),
            "2f2b369d74086d8d9646b052fec98991fbb7b870169f106f175c320ace9b97cb"
        );
        let dataset = &project.datasets[0];
        assert_eq!(dataset.schema.case_count, 344);
        assert_eq!(dataset.schema.columns.len(), 31);
        assert_eq!(
            dataset.fingerprint.0,
            "v2:2c46656f75626ca8fae8ca0752326dd5847f96e168dc02fa8195aeadb4a4a836"
        );
        assert!(
            dataset
                .batch
                .columns()
                .iter()
                .all(|column| column.null_count() == 0)
        );

        let model = &project.models[0];
        assert_eq!(model.constructs.len(), 8);
        assert_eq!(model.paths.len(), 13);
        assert_eq!(
            model
                .constructs
                .iter()
                .filter(|construct| construct.mode == MeasurementMode::Formative)
                .count(),
            4
        );
        assert_eq!(
            model
                .constructs
                .iter()
                .map(|construct| construct.indicators.len())
                .sum::<usize>(),
            31
        );

        let recipe = &project.recipes[0];
        assert_eq!(recipe.method_config, Some(MethodConfig::PlsAlgorithm));
        assert_eq!(recipe.settings.weighting_scheme, WeightingScheme::Path);
        assert_eq!(recipe.settings.preprocessing, Preprocessing::Standardized);
        assert_eq!(recipe.settings.tolerance, 1e-7);
        assert_eq!(recipe.settings.max_iterations, 3_000);
        assert_eq!(recipe.settings.bootstrap_samples, 0);
        assert_eq!(
            recipe
                .metadata
                .get("preprocessing_receipt_sha256")
                .map(String::as_str),
            Some("eff5124d1a7589aee4a7cf580c16de93a2fefebb6fc4b03d9efa60a8768f009b")
        );
        let warning_codes = validate_recipe(recipe)
            .into_iter()
            .filter(|issue| issue.severity == Severity::Warning)
            .map(|issue| issue.code)
            .collect::<Vec<_>>();
        assert_eq!(warning_codes, ["construct.single_item"]);

        assert_eq!(project.results[0].status, RunStatus::Completed);
        let estimation: PlsResult = match &project.results[0].payload {
            qpls_core::AnalysisPayload::PlsPmV1 { estimation, .. } => {
                serde_json::from_value(estimation.clone()).unwrap()
            }
            payload => panic!("corporate sample produced unexpected payload {payload:?}"),
        };
        assert!(estimation.converged);
        assert_eq!(estimation.iterations, 8);
        assert_eq!(estimation.used_observations, 344);
        assert_eq!(estimation.omitted_observations, 0);
        assert!(estimation.warnings.is_empty());

        let mut actual = BTreeMap::new();
        for estimate in &estimation.paths {
            actual.insert(
                format!("path:{}->{}", estimate.source, estimate.target),
                estimate.coefficient,
            );
        }
        for (construct, value) in &estimation.r_squared {
            actual.insert(format!("r2:{construct}"), *value);
        }
        for estimate in &estimation.outer_estimates {
            let mode = &model
                .constructs
                .iter()
                .find(|construct| construct.id == estimate.construct)
                .unwrap()
                .mode;
            let (family, value) = if *mode == MeasurementMode::Formative {
                ("weight", estimate.weight)
            } else {
                ("loading", estimate.loading)
            };
            actual.insert(
                format!("{family}:{}:{}", estimate.construct, estimate.indicator),
                value,
            );
        }
        let reference: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../validation/benchmarks/corporate_reputation_smartpls/smartpls_reference_v1.json"
        ))
        .unwrap();
        let expected = reference["values"].as_object().unwrap();
        assert_eq!(actual.len(), 48);
        assert_eq!(expected.len(), 48);
        for (parameter, expected) in expected {
            let expected = expected.as_f64().unwrap();
            let observed = actual[parameter];
            let rounded = (observed * 1_000.0).round() / 1_000.0;
            assert_eq!(
                rounded, expected,
                "{parameter} was {observed} and rounded to {rounded}"
            );
        }

        let workspace = &project.layouts["workspace"];
        assert_eq!(workspace["nodes"].as_array().unwrap().len(), 8);
        assert_eq!(workspace["edges"].as_array().unwrap().len(), 13);
        assert_eq!(workspace["diagramMode"], "sem");
        assert_eq!(workspace["diagramOverlaySettings"]["mode"], "paths_r2");
        assert_eq!(workspace["activeRecipeId"], recipe.id.to_string());
        assert_eq!(workspace["activeRunId"], project.results[0].id.to_string());
    }

    #[test]
    fn organizational_identification_sample_matches_the_screenshot_three_decimal_reference() {
        let sample = BundledSampleProject::OrganizationalIdentification;
        let project = build_bundled_sample_project(sample).unwrap();
        assert_eq!(project.manifest.name, "Organizational Identification Model");
        assert_eq!(
            format!("{:x}", Sha256::digest(sample.identity().dataset_bytes)),
            "5066d3b4bd24d14ad5d3efc91c1c40c57c41de63d987456d2cf8aad40c20ceed"
        );

        let dataset = &project.datasets[0];
        assert_eq!(dataset.schema.case_count, 305);
        assert_eq!(dataset.schema.columns.len(), 22);
        assert_eq!(
            dataset.fingerprint.0,
            "v2:fa5968177bc154d04ae8bfbba9853c56126a07ff84fc8d7d40cf3a46c6d8290a"
        );
        assert!(
            dataset
                .schema
                .columns
                .iter()
                .any(|column| column.name == "gender")
        );
        assert!(
            dataset
                .batch
                .columns()
                .iter()
                .all(|column| column.null_count() == 0)
        );

        let model = &project.models[0];
        assert_eq!(model.id.to_string(), "00000000-0000-0000-0000-000000003301");
        assert_eq!(model.name, "Organizational Identification Model");
        assert_eq!(model.constructs.len(), 4);
        assert_eq!(model.paths.len(), 3);
        assert!(
            model
                .constructs
                .iter()
                .all(|construct| construct.mode == MeasurementMode::Reflective)
        );
        assert_eq!(
            model
                .constructs
                .iter()
                .map(|construct| construct.indicators.len())
                .sum::<usize>(),
            21
        );
        assert!(model.constructs.iter().all(|construct| {
            !construct
                .indicators
                .iter()
                .any(|indicator| indicator == "gender")
        }));

        let recipe = &project.recipes[0];
        assert_eq!(
            recipe.id.to_string(),
            "00000000-0000-0000-0000-000000003302"
        );
        assert_eq!(recipe.method_config, Some(MethodConfig::PlsAlgorithm));
        assert_eq!(recipe.settings.method, qpls_core::AnalysisMethod::PlsPm);
        assert_eq!(recipe.settings.weighting_scheme, WeightingScheme::Path);
        assert_eq!(recipe.settings.preprocessing, Preprocessing::Standardized);
        assert_eq!(
            recipe.settings.missing_data,
            MissingDataPolicy::ListwiseDeletion
        );
        assert_eq!(recipe.settings.tolerance, 1e-7);
        assert_eq!(recipe.settings.max_iterations, 3_000);
        assert_eq!(recipe.settings.bootstrap_samples, 0);
        assert_eq!(recipe.settings.studentized_inner_samples, 0);
        assert_eq!(recipe.settings.permutation_samples, 0);
        assert_eq!(recipe.settings.seed, 20_260_823);
        assert_eq!(recipe.settings.workers, 1);
        assert_eq!(recipe.settings.confidence_level, 0.95);
        assert_eq!(recipe.settings.case_weight_column, None);
        assert_eq!(
            recipe
                .metadata
                .get("preprocessing_receipt")
                .map(String::as_str),
            Some("validation/fixtures/organizational_identification_v1.provenance.json")
        );
        assert_eq!(
            recipe
                .metadata
                .get("preprocessing_receipt_sha256")
                .map(String::as_str),
            Some("98f13a5c7b59cbf1ff54fbda218ee0d2b9423a07d87b0d2a9f3e765cdd930b5b")
        );
        assert_eq!(
            recipe
                .metadata
                .get("smartpls_reference")
                .map(String::as_str),
            Some(
                "validation/benchmarks/organizational_identification/screenshot_reference_v1.json"
            )
        );
        assert_eq!(
            recipe
                .metadata
                .get("smartpls_reference_sha256")
                .map(String::as_str),
            Some("9c1c6848f055ec2f3c00b1d3fe81ddb9fc973a4cc279b0399ffa54ca339af4fa")
        );
        let recipe_issues = validate_recipe(recipe);
        assert!(
            recipe_issues
                .iter()
                .all(|issue| issue.severity != Severity::Error)
        );
        assert!(
            recipe_issues
                .iter()
                .all(|issue| issue.severity != Severity::Warning)
        );

        assert_eq!(project.results[0].status, RunStatus::Completed);
        let estimation: PlsResult = match &project.results[0].payload {
            qpls_core::AnalysisPayload::PlsPmV1 { estimation, .. } => {
                serde_json::from_value(estimation.clone()).unwrap()
            }
            payload => panic!(
                "organizational identification sample produced unexpected payload {payload:?}"
            ),
        };
        assert!(estimation.converged);
        assert_eq!(estimation.iterations, 7);
        assert_eq!(estimation.used_observations, 305);
        assert_eq!(estimation.omitted_observations, 0);
        assert!(estimation.warnings.is_empty());

        let mut actual = BTreeMap::new();
        for estimate in &estimation.paths {
            actual.insert(
                format!("path:{}->{}", estimate.source, estimate.target),
                estimate.coefficient,
            );
        }
        for (construct, value) in &estimation.r_squared {
            actual.insert(format!("r2:{construct}"), *value);
        }
        for estimate in &estimation.outer_estimates {
            actual.insert(
                format!("loading:{}:{}", estimate.construct, estimate.indicator),
                estimate.loading,
            );
        }
        let reference: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../validation/benchmarks/organizational_identification/screenshot_reference_v1.json"
        ))
        .unwrap();
        let expected = reference["values"].as_object().unwrap();
        assert_eq!(actual.len(), 27);
        assert_eq!(expected.len(), 27);
        for (parameter, expected) in expected {
            let expected = expected.as_f64().unwrap();
            let observed = actual[parameter];
            let rounded = (observed * 1_000.0).round() / 1_000.0;
            assert_eq!(
                rounded, expected,
                "{parameter} was {observed} and rounded to {rounded}"
            );
        }

        let workspace = &project.layouts["workspace"];
        assert_eq!(workspace["nodes"].as_array().unwrap().len(), 4);
        assert_eq!(workspace["edges"].as_array().unwrap().len(), 3);
        assert!(
            workspace["edges"]
                .as_array()
                .unwrap()
                .iter()
                .all(|edge| edge["type"] == "straight")
        );
        assert_eq!(workspace["diagramMode"], "sem");
        assert_eq!(workspace["diagramOverlaySettings"]["mode"], "paths_r2");
        assert_eq!(workspace["activeRecipeId"], recipe.id.to_string());
        assert_eq!(workspace["activeRunId"], project.results[0].id.to_string());
    }

    #[test]
    fn bundled_sample_parser_rejects_unadvertised_or_unknown_ids() {
        assert_eq!(
            BundledSampleProject::parse("corporate_reputation").unwrap(),
            BundledSampleProject::CorporateReputation
        );
        assert_eq!(
            BundledSampleProject::parse("organizational_identification").unwrap(),
            BundledSampleProject::OrganizationalIdentification
        );
        assert_eq!(
            BundledSampleProject::parse("simple_pls").unwrap(),
            BundledSampleProject::SimplePls
        );
        assert_eq!(
            BundledSampleProject::parse("mediation").unwrap(),
            BundledSampleProject::Mediation
        );
        for unsupported in ["", "plspredict", "cbsem_cfa"] {
            assert!(BundledSampleProject::parse(unsupported).is_err());
        }
    }
}
