use chrono::{DateTime, Utc};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisRecipe, AnalysisSettings, Construct, ControlPath,
    HigherOrderConstruct, InteractionTerm, MeasurementMode, MethodConfig, ModelSpec,
    StructuralPath,
};
use qpls_data::{Dataset, ImportOptions, import_delimited_bytes};
use qpls_project::Project;
use qpls_runner::run_pls_analysis;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
use uuid::Uuid;

use crate::sample_projects_general_sem::{
    BundledGeneralSemSampleV1, materialize_bundled_general_sem_sample_v1,
};

const SAMPLE_CATALOG_JSON: &str = include_str!("../../src/data/bundledSampleProjects.v1.json");
const SUPPORTED_SAMPLE_CATALOG_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BundledSampleCatalogV1 {
    schema_version: u32,
    default_sample_id: String,
    datasets: Vec<CatalogDatasetV1>,
    construct_sets: BTreeMap<String, Vec<Construct>>,
    samples: Vec<CatalogSampleV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogDatasetV1 {
    id: String,
    file_name: String,
    source_path: String,
    sha256: String,
    #[serde(default)]
    expected_fingerprint: Option<String>,
    lineage_summary: String,
    #[serde(default)]
    metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogSampleV1 {
    id: String,
    label: String,
    detail: String,
    project_name: String,
    sample_version: String,
    dataset_id: String,
    #[serde(default)]
    project_kind: CatalogProjectKindV1,
    model: CatalogModelV1,
    runs: Vec<CatalogRunV1>,
    presentation: CatalogPresentationV1,
    #[serde(default)]
    metadata: BTreeMap<String, String>,
    acceptance: CatalogAcceptanceV1,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CatalogProjectKindV1 {
    #[default]
    OrdinaryV1,
    GeneralSemV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogModelV1 {
    id: Uuid,
    name: String,
    construct_set_id: String,
    #[serde(default)]
    extra_constructs: Vec<Construct>,
    paths: Vec<StructuralPath>,
    #[serde(default)]
    controls: Vec<ControlPath>,
    #[serde(default, rename = "higher_order_constructs")]
    higher_order_constructs: Vec<HigherOrderConstruct>,
    #[serde(default)]
    interactions: Vec<InteractionTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogRunV1 {
    id: Uuid,
    created_at: String,
    settings: AnalysisSettings,
    method_config: MethodConfig,
    #[serde(default)]
    metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogPresentationV1 {
    edge_type: String,
    #[serde(default)]
    positions: BTreeMap<String, CatalogPositionV1>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogPositionV1 {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogAcceptanceV1 {
    case_count: usize,
    column_count: usize,
    construct_count: usize,
    path_count: usize,
    #[serde(default)]
    reference_path: Option<String>,
    #[serde(default)]
    expected_value_count: Option<usize>,
}

struct EmbeddedDataset {
    source_path: &'static str,
    file_name: &'static str,
    bytes: &'static [u8],
}

static SAMPLE_CATALOG: OnceLock<Result<BundledSampleCatalogV1, String>> = OnceLock::new();

fn bundled_sample_catalog() -> Result<&'static BundledSampleCatalogV1, String> {
    SAMPLE_CATALOG
        .get_or_init(|| {
            let catalog: BundledSampleCatalogV1 = serde_json::from_str(SAMPLE_CATALOG_JSON)
                .map_err(|error| format!("invalid bundled sample catalog: {error}"))?;
            validate_catalog(&catalog)?;
            Ok(catalog)
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn validate_catalog(catalog: &BundledSampleCatalogV1) -> Result<(), String> {
    if catalog.schema_version != SUPPORTED_SAMPLE_CATALOG_SCHEMA {
        return Err(format!(
            "unsupported bundled sample catalog schema {}; expected {}",
            catalog.schema_version, SUPPORTED_SAMPLE_CATALOG_SCHEMA
        ));
    }
    let mut dataset_ids = BTreeSet::new();
    for dataset in &catalog.datasets {
        if dataset.id.trim().is_empty() || !dataset_ids.insert(dataset.id.as_str()) {
            return Err(format!(
                "duplicate or empty bundled dataset id {:?}",
                dataset.id
            ));
        }
        if dataset.sha256.len() != 64
            || !dataset
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(format!(
                "bundled dataset {} has an invalid lowercase SHA-256",
                dataset.id
            ));
        }
        let embedded = embedded_dataset(&dataset.id)?;
        if embedded.source_path != dataset.source_path || embedded.file_name != dataset.file_name {
            return Err(format!(
                "bundled dataset {} does not match its embedded asset",
                dataset.id
            ));
        }
    }
    let mut sample_ids = BTreeSet::new();
    let mut model_ids = BTreeSet::new();
    let mut recipe_ids = BTreeSet::new();
    for sample in &catalog.samples {
        if sample.id.trim().is_empty() || !sample_ids.insert(sample.id.as_str()) {
            return Err(format!(
                "duplicate or empty bundled sample id {:?}",
                sample.id
            ));
        }
        if !dataset_ids.contains(sample.dataset_id.as_str()) {
            return Err(format!(
                "sample {} references unknown dataset {}",
                sample.id, sample.dataset_id
            ));
        }
        let Some(base_constructs) = catalog.construct_sets.get(&sample.model.construct_set_id)
        else {
            return Err(format!(
                "sample {} references unknown construct set {}",
                sample.id, sample.model.construct_set_id
            ));
        };
        if !model_ids.insert(sample.model.id) {
            return Err(format!(
                "sample {} reuses model UUID {}",
                sample.id, sample.model.id
            ));
        }
        if sample.runs.is_empty() {
            return Err(format!(
                "sample {} must declare at least one completed run",
                sample.id
            ));
        }
        for run in &sample.runs {
            parse_time(&run.created_at)?;
            if !recipe_ids.insert(run.id) {
                return Err(format!(
                    "sample {} reuses recipe UUID {}",
                    sample.id, run.id
                ));
            }
        }
        if !matches!(
            sample.presentation.edge_type.as_str(),
            "straight" | "smoothstep" | "default"
        ) {
            return Err(format!(
                "sample {} has unsupported edge type {}",
                sample.id, sample.presentation.edge_type
            ));
        }
        let constructs = base_constructs
            .iter()
            .chain(sample.model.extra_constructs.iter())
            .collect::<Vec<_>>();
        let construct_ids = constructs
            .iter()
            .map(|construct| construct.id.as_str())
            .collect::<BTreeSet<_>>();
        if construct_ids.len() != constructs.len() {
            return Err(format!(
                "sample {} contains duplicate construct ids",
                sample.id
            ));
        }
        for path in &sample.model.paths {
            if !construct_ids.contains(path.source.as_str())
                || !construct_ids.contains(path.target.as_str())
            {
                return Err(format!(
                    "sample {} path {} -> {} has an unknown endpoint",
                    sample.id, path.source, path.target
                ));
            }
        }
        for interaction in &sample.model.interactions {
            let product = constructs
                .iter()
                .find(|construct| construct.id == interaction.product_construct)
                .ok_or_else(|| {
                    format!(
                        "sample {} interaction {} has no product construct",
                        sample.id, interaction.id
                    )
                })?;
            if !product.indicators.is_empty() {
                return Err(format!(
                    "sample {} interaction product {} must have no authored indicators",
                    sample.id, product.id
                ));
            }
            if !sample.model.paths.iter().any(|path| {
                path.source == interaction.product_construct && path.target == interaction.outcome
            }) {
                return Err(format!(
                    "sample {} interaction {} has no product-to-outcome path",
                    sample.id, interaction.id
                ));
            }
        }
        if sample.project_kind == CatalogProjectKindV1::OrdinaryV1
            && sample.model.interactions.len() > 1
        {
            return Err(format!(
                "ordinary bundled sample {} exceeds the validated single-interaction scope",
                sample.id
            ));
        }
        if sample.project_kind == CatalogProjectKindV1::GeneralSemV1 {
            if sample.runs.len() != 1 {
                return Err(format!(
                    "General SEM sample {} must declare exactly one resident point run",
                    sample.id
                ));
            }
            if sample.model.interactions.is_empty() {
                return Err(format!(
                    "General SEM sample {} must declare at least one interaction",
                    sample.id
                ));
            }
            if !sample.model.controls.is_empty() || !sample.model.higher_order_constructs.is_empty()
            {
                return Err(format!(
                    "General SEM bundled moderation sample {} exceeds the qualified direct-only point scope",
                    sample.id
                ));
            }
        }
        for higher_order in &sample.model.higher_order_constructs {
            if !construct_ids.contains(higher_order.id.as_str())
                || higher_order
                    .components
                    .iter()
                    .any(|component| !construct_ids.contains(component.as_str()))
            {
                return Err(format!(
                    "sample {} has an invalid higher-order declaration {}",
                    sample.id, higher_order.id
                ));
            }
        }
        if sample.acceptance.construct_count != constructs.len()
            || sample.acceptance.path_count != sample.model.paths.len()
        {
            return Err(format!(
                "sample {} acceptance counts do not match its model",
                sample.id
            ));
        }
    }
    if !sample_ids.contains(catalog.default_sample_id.as_str()) {
        return Err("bundled sample default id is not present in the catalog".into());
    }
    Ok(())
}

fn embedded_dataset(id: &str) -> Result<EmbeddedDataset, String> {
    match id {
        "corporate_reputation_mean_replaced_v1" => Ok(EmbeddedDataset {
            source_path: "validation/fixtures/corporate_reputation_smartpls_mean_replaced_v1.csv",
            file_name: "corporate_reputation_smartpls_mean_replaced_v1.csv",
            bytes: include_bytes!(
                "../../validation/fixtures/corporate_reputation_smartpls_mean_replaced_v1.csv"
            ),
        }),
        "organizational_identification_v1" => Ok(EmbeddedDataset {
            source_path: "validation/fixtures/organizational_identification_v1.csv",
            file_name: "organizational_identification_v1.csv",
            bytes: include_bytes!("../../validation/fixtures/organizational_identification_v1.csv"),
        }),
        "simple_reflective_v1" => Ok(EmbeddedDataset {
            source_path: "validation/fixtures/simple_reflective.csv",
            file_name: "simple_reflective.csv",
            bytes: include_bytes!("../../validation/fixtures/simple_reflective.csv"),
        }),
        "mediation_sample_v1" => Ok(EmbeddedDataset {
            source_path: "validation/fixtures/mediation_sample.csv",
            file_name: "mediation_sample.csv",
            bytes: include_bytes!("../../validation/fixtures/mediation_sample.csv"),
        }),
        other => Err(format!("bundled dataset asset {other:?} is not embedded")),
    }
}

pub(crate) fn build_bundled_sample_project(sample_id: &str) -> Result<Project, String> {
    let catalog = bundled_sample_catalog()?;
    let sample = catalog
        .samples
        .iter()
        .find(|sample| sample.id == sample_id)
        .ok_or_else(|| format!("unknown bundled sample project {sample_id:?}"))?;
    if sample.project_kind != CatalogProjectKindV1::OrdinaryV1 {
        return Err(format!(
            "bundled sample {} is a strict General SEM project; use the schema-6 sample materializer",
            sample.id
        ));
    }
    let (dataset_spec, dataset) = load_catalog_sample_dataset(catalog, sample)?;
    let model = catalog_model(catalog, sample)?;
    let template_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&sample.model).map_err(|error| error.to_string())?)
    );
    let dataset_id = dataset.id.to_string();
    let model_id = model.id.to_string();
    let mut project = Project::new(&sample.project_name);
    project.datasets.push(dataset);
    project.models.push(model.clone());
    let mut active_recipe_id = String::new();
    let mut active_result_id = String::new();
    for run in &sample.runs {
        let mut metadata = dataset_spec.metadata.clone();
        metadata.extend(sample.metadata.clone());
        metadata.extend(run.metadata.clone());
        metadata.extend(BTreeMap::from([
            ("sample_project".into(), sample.sample_version.clone()),
            ("sample_id".into(), sample.id.clone()),
            ("sample_version".into(), sample.sample_version.clone()),
            (
                "sample_catalog_schema_version".into(),
                catalog.schema_version.to_string(),
            ),
            ("sample_dataset_asset".into(), dataset_spec.id.clone()),
            ("sample_template_sha256".into(), template_sha256.clone()),
            ("fixture".into(), dataset_spec.source_path.clone()),
            ("fixture_sha256".into(), dataset_spec.sha256.clone()),
        ]));
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: run.id,
            created_at: parse_time(&run.created_at)?,
            dataset_fingerprint: project.datasets[0].fingerprint.0.clone(),
            model: model.clone(),
            settings: run.settings.clone(),
            method_config: Some(run.method_config.clone()),
            metadata,
        };
        let result = run_pls_analysis(&project.datasets[0], &recipe, || false, |_| {})
            .map_err(|error| error.to_string())?;
        active_recipe_id = recipe.id.to_string();
        active_result_id = result.id.to_string();
        project
            .append_validated_result(recipe, result)
            .map_err(|error| error.to_string())?;
    }
    let active_settings = &sample
        .runs
        .last()
        .expect("catalog validation requires a run")
        .settings;
    let (nodes, edges) = sample_presentation(sample, &model);
    project.layouts.insert(
        "workspace".into(),
        json!({
            "activeDatasetId": dataset_id,
            "activeModelId": model_id,
            "activeRecipeId": active_recipe_id,
            "activeRunId": active_result_id,
            "selectedRunId": active_result_id,
            "nodes": nodes,
            "edges": edges,
            "analysisSettings": {
                "method": active_settings.method.as_str(),
                "weightingScheme": active_settings.weighting_scheme,
                "tolerance": active_settings.tolerance,
                "maxIterations": active_settings.max_iterations,
                "bootstrapSamples": active_settings.bootstrap_samples,
                "studentizedInnerSamples": active_settings.studentized_inner_samples,
                "permutationSamples": active_settings.permutation_samples,
                "seed": active_settings.seed,
                "workers": active_settings.workers,
                "confidenceLevel": active_settings.confidence_level,
                "preprocessing": active_settings.preprocessing,
                "missingData": active_settings.missing_data
            },
            "diagramMode": "sem",
            "diagramOverlaySettings": {"selectedRunId": active_result_id, "mode": "paths_r2"}
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
                "summary": dataset_spec.lineage_summary,
                "sourceColumn": null,
                "targetColumn": null
            }]
        }),
    );
    Ok(project)
}

/// Materializes a fresh strict schema-6 copy of a bundled General SEM sample
/// and returns the absolute archive path. The ordinary schema-5 sample builder
/// deliberately never executes this path.
pub(crate) fn materialize_bundled_general_sem_sample(sample_id: &str) -> Result<String, String> {
    let specification = bundled_general_sem_sample_specification(sample_id)?;
    materialize_bundled_general_sem_sample_v1(&specification)
}

fn load_catalog_sample_dataset<'a>(
    catalog: &'a BundledSampleCatalogV1,
    sample: &CatalogSampleV1,
) -> Result<(&'a CatalogDatasetV1, Dataset), String> {
    let dataset_spec = catalog
        .datasets
        .iter()
        .find(|dataset| dataset.id == sample.dataset_id)
        .ok_or_else(|| format!("sample {} references an unavailable dataset", sample.id))?;
    let embedded = embedded_dataset(&dataset_spec.id)?;
    let actual_sha256 = format!("{:x}", Sha256::digest(embedded.bytes));
    if actual_sha256 != dataset_spec.sha256 {
        return Err(format!(
            "bundled dataset {} failed its SHA-256 check",
            dataset_spec.id
        ));
    }
    let dataset = import_delimited_bytes(
        embedded.bytes,
        embedded.file_name,
        b',',
        &ImportOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    if let Some(expected) = &dataset_spec.expected_fingerprint {
        if &dataset.fingerprint.0 != expected {
            return Err(format!(
                "bundled dataset {} fingerprint mismatch",
                dataset_spec.id
            ));
        }
    }
    if dataset.schema.case_count != sample.acceptance.case_count
        || dataset.schema.columns.len() != sample.acceptance.column_count
    {
        return Err(format!(
            "bundled sample {} dataset dimensions do not match the catalog",
            sample.id
        ));
    }
    Ok((dataset_spec, dataset))
}

fn bundled_general_sem_sample_specification(
    sample_id: &str,
) -> Result<BundledGeneralSemSampleV1, String> {
    let catalog = bundled_sample_catalog()?;
    let sample = catalog
        .samples
        .iter()
        .find(|sample| sample.id == sample_id)
        .ok_or_else(|| format!("unknown bundled sample project {sample_id:?}"))?;
    if sample.project_kind != CatalogProjectKindV1::GeneralSemV1 {
        return Err(format!(
            "bundled sample {} is an ordinary editable project, not a strict General SEM sample",
            sample.id
        ));
    }
    let (dataset_spec, dataset) = load_catalog_sample_dataset(catalog, sample)?;
    let run = sample
        .runs
        .first()
        .ok_or_else(|| format!("sample {} has no resident run", sample.id))?;

    let catalog_model = catalog_model(catalog, sample)?;
    let product_constructs = sample
        .model
        .interactions
        .iter()
        .map(|interaction| interaction.product_construct.as_str())
        .collect::<BTreeSet<_>>();
    let source_model = ModelSpec {
        id: catalog_model.id,
        name: catalog_model.name.clone(),
        constructs: catalog_model
            .constructs
            .into_iter()
            .filter(|construct| !product_constructs.contains(construct.id.as_str()))
            .collect(),
        paths: catalog_model
            .paths
            .into_iter()
            .filter(|path| !product_constructs.contains(path.source.as_str()))
            .collect(),
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    };
    let template_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&sample.model).map_err(|error| error.to_string())?)
    );
    let mut metadata = dataset_spec.metadata.clone();
    metadata.extend(sample.metadata.clone());
    metadata.extend(run.metadata.clone());
    metadata.extend(BTreeMap::from([
        ("sample_project".into(), sample.sample_version.clone()),
        ("sample_id".into(), sample.id.clone()),
        ("sample_version".into(), sample.sample_version.clone()),
        (
            "sample_catalog_schema_version".into(),
            catalog.schema_version.to_string(),
        ),
        ("sample_dataset_asset".into(), dataset_spec.id.clone()),
        ("sample_template_sha256".into(), template_sha256),
        ("fixture".into(), dataset_spec.source_path.clone()),
        ("fixture_sha256".into(), dataset_spec.sha256.clone()),
        ("execution_surface".into(), "standard_multimod_v1".into()),
        ("general_sem_generation".into(), "general_sem_v1".into()),
    ]));

    Ok(BundledGeneralSemSampleV1 {
        sample_id: sample.id.clone(),
        project_name: sample.project_name.clone(),
        sample_version: sample.sample_version.clone(),
        project_id: Uuid::from_u128(
            sample.model.id.as_u128() ^ 0x5150_4c53_4753_454d_0000_0000_0000_0001,
        ),
        dataset,
        source_model,
        interactions: sample.model.interactions.clone(),
        recipe_id: run.id,
        created_at: parse_time(&run.created_at)?,
        settings: run.settings.clone(),
        method_config: run.method_config.clone(),
        metadata,
    })
}

fn catalog_model(
    catalog: &BundledSampleCatalogV1,
    sample: &CatalogSampleV1,
) -> Result<ModelSpec, String> {
    let mut constructs = catalog
        .construct_sets
        .get(&sample.model.construct_set_id)
        .ok_or_else(|| format!("unknown construct set {}", sample.model.construct_set_id))?
        .clone();
    constructs.extend(sample.model.extra_constructs.clone());
    Ok(ModelSpec {
        id: sample.model.id,
        name: sample.model.name.clone(),
        constructs,
        paths: sample.model.paths.clone(),
        controls: sample.model.controls.clone(),
        higher_order_constructs: sample.model.higher_order_constructs.clone(),
        interactions: sample.model.interactions.clone(),
    })
}

fn sample_presentation(
    sample: &CatalogSampleV1,
    model: &ModelSpec,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    if sample.presentation.positions.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let nodes = model.constructs.iter().filter_map(|construct| {
        let position = sample.presentation.positions.get(&construct.id)?;
        Some(json!({
            "id": construct.id,
            "type": "construct",
            "position": {"x": position.x, "y": position.y},
            "data": {
                "label": construct.name,
                "shortName": construct.short_name,
                "mode": match construct.mode { MeasurementMode::Reflective => "reflective", MeasurementMode::Formative => "formative" },
                "indicators": construct.indicators
            }
        }))
    }).collect();
    let edges = model
        .paths
        .iter()
        .map(|path| {
            json!({
                "id": format!("path-{}-{}", path.source, path.target),
                "source": path.source,
                "target": path.target,
                "type": sample.presentation.edge_type,
                "label": "Path",
                "markerEnd": {"type": "arrowclosed", "width": 16, "height": 16}
            })
        })
        .collect();
    (nodes, edges)
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| format!("invalid bundled sample timestamp: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_projects_general_sem::build_bundled_general_sem_sample_archive_v1;
    use qpls_core::{
        AnalysisPayload, RunStatus, SemDerivedTermV4, SemRelationV4, Severity, validate_recipe,
    };
    use qpls_estimation::PlsResult;
    use qpls_project::{
        ProjectModelPayloadV6, ProjectSemGenerationV6, load_project, load_project_archive_v6,
        save_project,
    };
    use std::{fs, path::Path};

    fn estimation(project: &Project) -> PlsResult {
        match &project.results.last().unwrap().payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => {
                serde_json::from_value(estimation.clone()).unwrap()
            }
            payload => panic!("bundled sample produced unexpected payload {payload:?}"),
        }
    }

    fn observed_values(project: &Project) -> BTreeMap<String, f64> {
        let estimation = estimation(project);
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
            actual.insert(
                format!("weight:{}:{}", estimate.construct, estimate.indicator),
                estimate.weight,
            );
        }
        for estimate in &estimation.moderation.estimates {
            actual.insert(
                format!("interaction:{}", estimate.interaction),
                estimate.interaction_effect,
            );
        }
        actual
    }

    #[test]
    fn bundled_catalog_is_single_source_valid_and_complete() {
        let catalog = bundled_sample_catalog().unwrap();
        assert_eq!(catalog.schema_version, 1);
        assert_eq!(catalog.samples.len(), 8);
        assert_eq!(catalog.default_sample_id, "corporate_reputation");
        assert_eq!(
            catalog
                .datasets
                .iter()
                .filter(|dataset| dataset.id == "organizational_identification_v1")
                .count(),
            1
        );
        for sample in &catalog.samples {
            assert!(!sample.label.trim().is_empty());
            assert!(!sample.detail.trim().is_empty());
            assert_eq!(sample.runs.len(), 1);
            if sample.project_kind == CatalogProjectKindV1::GeneralSemV1 {
                assert!(build_bundled_sample_project(&sample.id).is_err());
                continue;
            }
            let project = build_bundled_sample_project(&sample.id).unwrap();
            assert_eq!(
                project.datasets[0].schema.case_count,
                sample.acceptance.case_count
            );
            assert_eq!(
                project.models[0].constructs.len(),
                sample.acceptance.construct_count
            );
            assert_eq!(project.models[0].paths.len(), sample.acceptance.path_count);
            assert_eq!(project.results[0].status, RunStatus::Completed);
            assert_eq!(project.recipes[0].metadata["sample_id"], sample.id);
        }
        assert!(build_bundled_sample_project("not_advertised").is_err());
    }

    #[test]
    fn every_bundled_sample_round_trips_as_an_editable_project() {
        for sample in bundled_sample_catalog()
            .unwrap()
            .samples
            .iter()
            .filter(|sample| sample.project_kind == CatalogProjectKindV1::OrdinaryV1)
        {
            let project = build_bundled_sample_project(&sample.id).unwrap();
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join(format!("{}.qpls", sample.id));
            save_project(&path, &project).unwrap();
            let reopened = load_project(&path).unwrap();
            assert_eq!(reopened.manifest.name, sample.project_name);
            assert_eq!(reopened.models, project.models);
            assert_eq!(reopened.recipes, project.recipes);
            assert_eq!(reopened.results.len(), sample.runs.len());
            assert_eq!(reopened.layouts["workspace"]["diagramMode"], "sem");
        }
    }

    #[test]
    fn two_outcome_moderation_materializes_one_strict_joint_general_sem_authority() {
        let sample_id = "organizational_identification_moderation";
        let specification = bundled_general_sem_sample_specification(sample_id).unwrap();
        assert_eq!(specification.source_model.constructs.len(), 4);
        assert_eq!(specification.source_model.paths.len(), 4);
        assert!(specification.source_model.interactions.is_empty());
        assert_eq!(specification.interactions.len(), 2);

        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("oi-two-outcome-moderation.qpls");
        let receipt =
            build_bundled_general_sem_sample_archive_v1(&specification, &archive).unwrap();
        assert_eq!(receipt.archive_path, archive);

        let reopened = load_project_archive_v6(&archive).unwrap();
        assert_eq!(reopened.document.project_id, specification.project_id);
        assert_eq!(
            reopened.document.sem_generation,
            Some(ProjectSemGenerationV6::GeneralSemV1)
        );
        assert!(reopened.document.supports_general_sem_v1());
        assert_eq!(reopened.datasets.len(), 1);
        assert_eq!(reopened.document.models.len(), 1);
        assert_eq!(reopened.document.recipes.len(), 1);
        assert_eq!(reopened.document.canonical_result_documents.len(), 1);

        let ProjectModelPayloadV6::SemModelV4 { model, .. } = &reopened.document.models[0].payload
        else {
            panic!("bundled moderation archive did not retain SemModelV4 authority");
        };
        let interactions = model
            .derived_terms
            .iter()
            .filter_map(|term| match term {
                SemDerivedTermV4::InteractionV2 {
                    id,
                    output,
                    operands,
                    ..
                } => Some((id, output, operands)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(interactions.len(), 2);
        let expected_operands = vec![
            "construct:org_identification".to_owned(),
            "construct:org_prestige".to_owned(),
        ];
        let mut model_outcomes = BTreeSet::new();
        for (_, output, operands) in &interactions {
            assert_eq!(**operands, expected_operands);
            let outcome = model
                .relations
                .iter()
                .find_map(|relation| match relation {
                    SemRelationV4::Structural { source, target, .. } if source == *output => {
                        Some(target.clone())
                    }
                    _ => None,
                })
                .expect("each interaction output must target one outcome");
            model_outcomes.insert(outcome);
        }
        assert_eq!(
            model_outcomes,
            BTreeSet::from([
                "construct:affective_commitment_joy".to_owned(),
                "construct:affective_commitment_love".to_owned(),
            ])
        );

        let canonical = reopened.document.canonical_result_documents[0].canonical_document();
        assert!(!canonical.document_id.is_empty());
        let general_sem = canonical
            .general_sem_results
            .as_ref()
            .expect("moderation result must retain typed General SEM results");
        assert_eq!(general_sem.interaction_effects.len(), 2);
        assert_eq!(general_sem.conditional_effects.len(), 6);
        let canonical_outcomes = general_sem
            .interaction_effects
            .iter()
            .map(|effect| {
                assert_eq!(effect.focal_predictor_id, expected_operands[0]);
                assert_eq!(effect.moderator_id, expected_operands[1]);
                effect.outcome_id.clone()
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(canonical_outcomes, model_outcomes);

        let exact_path = |target: &str| {
            general_sem
                .joint_stage_structural_coefficients
                .iter()
                .find(|coefficient| {
                    coefficient.source_id == "construct:org_identification"
                        && coefficient.target_id == target
                })
                .unwrap_or_else(|| panic!("missing joint OI path to {target}"))
                .estimate
                .estimate
        };
        let exact_gamma = |interaction_id: &str| {
            general_sem
                .interaction_effects
                .iter()
                .find(|effect| effect.interaction_id == interaction_id)
                .unwrap_or_else(|| panic!("missing scientific gamma for {interaction_id}"))
                .scientific_rescaled_gamma
                .estimate
        };
        let close = |actual: f64, expected: f64| {
            assert!(
                (actual - expected).abs() <= 1e-11,
                "expected {expected:.12}, observed {actual:.12}"
            );
        };
        close(
            exact_path("construct:affective_commitment_joy"),
            0.549_354_789_561,
        );
        close(
            exact_path("construct:affective_commitment_love"),
            -0.404_304_393_331,
        );
        close(exact_gamma("oi_x_op_to_acj"), 0.050_447_855_377);
        close(exact_gamma("oi_x_op_to_acl"), -0.198_829_205_065);
    }

    #[test]
    fn catalog_reference_values_match_current_engine_at_declared_precision() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let mut mismatches = Vec::new();
        for sample in bundled_sample_catalog()
            .unwrap()
            .samples
            .iter()
            .filter(|sample| sample.project_kind == CatalogProjectKindV1::OrdinaryV1)
        {
            let Some(reference_path) = &sample.acceptance.reference_path else {
                continue;
            };
            let reference: serde_json::Value =
                serde_json::from_slice(&fs::read(root.join(reference_path)).unwrap()).unwrap();
            let expected = reference["values"].as_object().unwrap();
            assert_eq!(
                Some(expected.len()),
                sample.acceptance.expected_value_count,
                "{}",
                sample.id
            );
            let project = build_bundled_sample_project(&sample.id).unwrap();
            let actual = observed_values(&project);
            let decimals = reference["precision_decimals"].as_u64().unwrap_or(3) as i32;
            let scale = 10_f64.powi(decimals);
            for (parameter, expected) in expected {
                let expected = expected.as_f64().unwrap();
                let observed = *actual
                    .get(parameter)
                    .unwrap_or_else(|| panic!("{} did not produce {parameter}", sample.id));
                let rounded = (observed * scale).round() / scale;
                if rounded != expected {
                    mismatches.push(format!(
                        "{} {parameter}: expected {expected}, observed {observed}, rounded {rounded}",
                        sample.id
                    ));
                }
            }
        }
        assert!(
            mismatches.is_empty(),
            "bundled reference mismatches:\n{}",
            mismatches.join("\n")
        );
    }

    #[test]
    fn benchmark_recipes_retain_explicit_scientific_settings_and_provenance() {
        for id in ["corporate_reputation", "organizational_identification"] {
            let project = build_bundled_sample_project(id).unwrap();
            let recipe = &project.recipes[0];
            assert_eq!(recipe.method_config, Some(MethodConfig::PlsAlgorithm));
            assert_eq!(recipe.settings.workers, 1);
            assert_eq!(recipe.settings.tolerance, 1e-7);
            assert_eq!(recipe.settings.max_iterations, 3_000);
            assert!(
                validate_recipe(recipe)
                    .iter()
                    .all(|issue| issue.severity != Severity::Error)
            );
            assert_eq!(recipe.metadata["sample_catalog_schema_version"], "1");
            assert_eq!(recipe.metadata["fixture_sha256"].len(), 64);
        }
    }
}
