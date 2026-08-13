use chrono::{DateTime, Utc};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisRecipe, AnalysisSettings, Construct, MeasurementMode,
    MethodConfig, ModelSpec, StructuralPath,
};
use qpls_data::{ImportOptions, import_delimited_bytes};
use qpls_project::Project;
use qpls_runner::run_pls_analysis;
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BundledSampleProject {
    SimplePls,
    Mediation,
}

impl BundledSampleProject {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "simple_pls" => Ok(Self::SimplePls),
            "mediation" => Ok(Self::Mediation),
            other => Err(format!(
                "unknown bundled sample project {other:?}; expected corporate_reputation, simple_pls, or mediation"
            )),
        }
    }

    fn identity(self) -> SampleIdentity {
        match self {
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
    let settings = AnalysisSettings::default();
    let recipe = AnalysisRecipe {
        schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        id: parse_uuid(identity.recipe_id)?,
        created_at: parse_time(identity.created_at)?,
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model: model.clone(),
        settings: settings.clone(),
        method_config: Some(MethodConfig::PlsAlgorithm),
        metadata: BTreeMap::from([
            ("sample_project".into(), identity.sample_version.into()),
            (
                "fixture".into(),
                format!("validation/fixtures/{}", identity.dataset_name),
            ),
        ]),
    };
    let result =
        run_pls_analysis(&dataset, &recipe, || false, |_| {}).map_err(|error| error.to_string())?;
    let dataset_id = dataset.id.to_string();
    let model_id = model.id.to_string();
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
            }
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
                "summary": format!("Bundled {}", identity.project_name),
                "sourceColumn": null,
                "targetColumn": null
            }]
        }),
    );
    Ok(project)
}

fn sample_model(sample: BundledSampleProject, id: &str) -> Result<ModelSpec, String> {
    let (name, constructs, paths) = match sample {
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
    Construct {
        id: id.into(),
        name: name.into(),
        short_name: short_name.into(),
        mode: MeasurementMode::Reflective,
        indicators: indicators.iter().map(|value| (*value).into()).collect(),
    }
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
    use qpls_project::{load_project, save_project};

    #[test]
    fn bundled_samples_are_distinct_complete_and_round_trip() {
        for (sample, expected_name, expected_constructs, expected_paths) in [
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
    fn bundled_sample_parser_rejects_unadvertised_or_unknown_ids() {
        assert_eq!(
            BundledSampleProject::parse("simple_pls").unwrap(),
            BundledSampleProject::SimplePls
        );
        assert_eq!(
            BundledSampleProject::parse("mediation").unwrap(),
            BundledSampleProject::Mediation
        );
        for unsupported in ["", "corporate_reputation", "plspredict", "cbsem_cfa"] {
            assert!(BundledSampleProject::parse(unsupported).is_err());
        }
    }
}
