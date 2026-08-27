//! Bundled strict General SEM sample materialization.
//!
//! This module intentionally does not call the legacy `Project` moderation
//! estimator. It converts a basic catalog model into SemModelV4, adds authored
//! `InteractionV2` terms, executes the qualified simultaneous point engine,
//! and publishes one strictly reopenable schema-6 archive with its immutable
//! canonical result attached.

use crate::recipe_v4_general_sem_canonical_result::build_recipe_v4_general_sem_pls_canonical_result_v1;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe, AnalysisRecipeModelBindingV4,
    AnalysisRecipeV4, AnalysisSettings, GeneralSemConfigV1, InteractionHierarchyPolicyV2,
    InteractionMethod, InteractionMethodV4, InteractionTerm, LegacyBasicModelInterpretationV4,
    MethodConfig, MissingDataPolicyV4, ModelSpec, SemDataBindingV4, SemDerivedTermV4, SemModelV4,
    SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4, StructuralRelationRoleV4,
    compile_general_sem_pls_recipe_v1, confirm_legacy_recipe_estimand_v4,
    migrate_analysis_recipe_to_v4_pending,
    pls_general_multiple_moderation_point_capability_cell_v1,
};
use qpls_data::Dataset;
use qpls_project::{
    GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1,
    append_canonical_result_document_v2_file_v6, create_populated_general_sem_project_archive_v6,
    load_project_archive_v6,
};
use qpls_runner::run_compiled_general_sem_pls_recipe_v1;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration as StdDuration, SystemTime},
};
use uuid::Uuid;

const SAMPLE_TEMP_DIRECTORY: &str = "quickpls-bundled-general-sem-v1";
const STALE_SAMPLE_AGE: StdDuration = StdDuration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone)]
pub(crate) struct BundledGeneralSemSampleV1 {
    pub sample_id: String,
    pub project_name: String,
    pub sample_version: String,
    pub project_id: Uuid,
    pub dataset: Dataset,
    /// Basic legacy model without product placeholders. This is only the
    /// deterministic migration source for SemModelV4; it is never estimated.
    pub source_model: ModelSpec,
    pub interactions: Vec<InteractionTerm>,
    pub recipe_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub settings: AnalysisSettings,
    pub method_config: MethodConfig,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BundledGeneralSemSampleBuildReceiptV1 {
    pub archive_path: PathBuf,
}

pub(crate) fn materialize_bundled_general_sem_sample_v1(
    sample: &BundledGeneralSemSampleV1,
) -> Result<String, String> {
    let directory = owned_sample_temp_directory()?;
    let process_id = std::process::id();
    cleanup_stale_materialized_samples(&directory, process_id);
    let destination = directory.join(format!(
        "{}-{process_id}-{}.qpls",
        safe_file_stem(&sample.sample_id),
        Uuid::new_v4()
    ));
    match build_bundled_general_sem_sample_archive_v1(sample, &destination) {
        Ok(receipt) => Ok(receipt.archive_path.to_string_lossy().into_owned()),
        Err(error) => {
            // The unique destination was absent before this call and is scoped
            // to the exact owned directory, so removing a failed publication
            // cannot touch a user-selected project.
            if fs::symlink_metadata(&destination)
                .is_ok_and(|metadata| metadata.file_type().is_file())
            {
                let _ = fs::remove_file(&destination);
            }
            Err(error)
        }
    }
}

pub(crate) fn build_bundled_general_sem_sample_archive_v1(
    sample: &BundledGeneralSemSampleV1,
    destination: &Path,
) -> Result<BundledGeneralSemSampleBuildReceiptV1, String> {
    validate_sample_contract(sample, destination)?;
    let (mut recipe, mut model) = migrate_source_authority(sample)?;
    for interaction in &sample.interactions {
        add_catalog_interaction(&mut model, interaction)?;
    }
    model = model.canonicalized();
    model.ensure_valid().map_err(|error| error.to_string())?;
    recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
        model_id: model.id.clone(),
        scientific_sha256: model
            .scientific_sha256()
            .map_err(|error| error.to_string())?,
    };
    recipe.ensure_valid().map_err(|error| error.to_string())?;

    let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model))
        .map_err(|error| format!("bundled General SEM sample compilation failed: {error}"))?;
    let expected_cell = pls_general_multiple_moderation_point_capability_cell_v1();
    if artifact.capability_cell() != &expected_cell
        || artifact.plan().two_way_interactions().len() != sample.interactions.len()
    {
        return Err(
            "bundled General SEM sample did not compile to the qualified simultaneous multiple-moderation point cell"
                .into(),
        );
    }
    let result = run_compiled_general_sem_pls_recipe_v1(
        &sample.dataset,
        &recipe,
        &model,
        &artifact,
        || false,
        |_| {},
    )
    .map_err(|error| format!("bundled General SEM sample execution failed: {error}"))?;
    let interaction_result = result.interaction_point_estimation().ok_or_else(|| {
        "qualified bundled General SEM moderation run omitted its interaction point result"
            .to_owned()
    })?;
    interaction_result
        .ensure_valid_against_plan_v1(artifact.plan())
        .map_err(|error| format!("bundled General SEM interaction result is invalid: {error}"))?;

    let creation = create_populated_general_sem_project_archive_v6(
        destination,
        sample.project_id,
        &sample.project_name,
        sample.created_at,
        &sample.dataset,
        model.clone(),
        recipe.clone(),
    )
    .map_err(|error| format!("bundled General SEM archive creation failed: {error}"))?;
    if !creation.strict_reopen_validated {
        return Err("bundled General SEM archive creation did not pass strict reopen".into());
    }

    let completed_at = sample.created_at + Duration::seconds(1);
    let started_at = sample.created_at.to_rfc3339_opts(SecondsFormat::Secs, true);
    let completed_at = completed_at.to_rfc3339_opts(SecondsFormat::Secs, true);
    let job_id =
        Uuid::from_u128(sample.recipe_id.as_u128() ^ 0x5150_4c53_4753_454d_5255_4e00_0000_0001);
    let canonical = build_recipe_v4_general_sem_pls_canonical_result_v1(
        job_id,
        sample.project_id,
        sample.dataset.id,
        &started_at,
        &completed_at,
        &recipe,
        &model,
        &result,
    )
    .map_err(|errors| {
        format!(
            "bundled General SEM canonical projection failed: {}",
            errors.join("; ")
        )
    })?;
    let canonical_document_id = canonical.document_id.clone();
    let archive_canonical: qpls_project::CanonicalResultDocumentV2 = serde_json::from_value(
        serde_json::to_value(canonical)
            .map_err(|error| format!("canonical serialization failed: {error}"))?,
    )
    .map_err(|error| format!("archive canonical conversion failed: {error}"))?;
    archive_canonical
        .ensure_valid()
        .map_err(|error| format!("archive canonical validation failed: {error}"))?;
    let append = append_canonical_result_document_v2_file_v6(
        destination,
        &creation.destination_archive_sha256,
        archive_canonical,
    )
    .map_err(|error| format!("bundled General SEM canonical append failed: {error}"))?;
    if !append.post_write_validated || append.canonical_result_document_count != 1 {
        return Err("bundled General SEM canonical append did not pass strict validation".into());
    }

    let reopened = load_project_archive_v6(destination)
        .map_err(|error| format!("bundled General SEM strict reopen failed: {error}"))?;
    if reopened.document.project_id != sample.project_id
        || reopened.datasets.len() != 1
        || reopened.document.models.len() != 1
        || reopened.document.recipes.len() != 1
        || reopened.document.canonical_result_documents.len() != 1
        || reopened.document.models[0].model_id != model.id
        || reopened.document.recipes[0].id != recipe.id
        || reopened.document.canonical_result_documents[0].document_id() != canonical_document_id
    {
        return Err("materialized bundled General SEM archive failed identity validation".into());
    }

    Ok(BundledGeneralSemSampleBuildReceiptV1 {
        archive_path: destination.to_path_buf(),
    })
}

fn validate_sample_contract(
    sample: &BundledGeneralSemSampleV1,
    destination: &Path,
) -> Result<(), String> {
    if sample.sample_id.trim().is_empty()
        || sample.project_name.trim().is_empty()
        || sample.sample_version.trim().is_empty()
    {
        return Err("bundled General SEM sample identity is incomplete".into());
    }
    if !destination.is_absolute()
        || destination.extension().and_then(|value| value.to_str()) != Some("qpls")
    {
        return Err("bundled General SEM destination must be an absolute .qpls path".into());
    }
    if destination.exists() {
        return Err(format!(
            "bundled General SEM destination already exists: {}",
            destination.display()
        ));
    }
    if sample.interactions.is_empty() {
        return Err("bundled General SEM moderation requires at least one interaction".into());
    }
    if !sample.source_model.controls.is_empty()
        || !sample.source_model.higher_order_constructs.is_empty()
        || !sample.source_model.interactions.is_empty()
    {
        return Err(
            "bundled General SEM migration source must remain a basic direct-only model".into(),
        );
    }
    if sample.settings.method != AnalysisMethod::PlsPm
        || sample.settings.bootstrap_samples != 0
        || sample.settings.studentized_inner_samples != 0
        || sample.settings.permutation_samples != 0
        || sample.method_config != MethodConfig::PlsAlgorithm
    {
        return Err(
            "bundled General SEM moderation sample must request the qualified PLS point configuration"
                .into(),
        );
    }
    let construct_ids = sample
        .source_model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut interaction_ids = BTreeSet::new();
    let mut product_ids = BTreeSet::new();
    for interaction in &sample.interactions {
        if interaction.method != InteractionMethod::TwoStageProductScore
            || !interaction_ids.insert(interaction.id.as_str())
            || !product_ids.insert(interaction.product_construct.as_str())
        {
            return Err(format!(
                "bundled General SEM interaction {} is duplicate or not two-stage",
                interaction.id
            ));
        }
        if !construct_ids.contains(interaction.predictor.as_str())
            || !construct_ids.contains(interaction.moderator.as_str())
            || !construct_ids.contains(interaction.outcome.as_str())
        {
            return Err(format!(
                "bundled General SEM interaction {} references an unknown authored construct",
                interaction.id
            ));
        }
        for main_effect in [&interaction.predictor, &interaction.moderator] {
            if !sample
                .source_model
                .paths
                .iter()
                .any(|path| path.source == *main_effect && path.target == interaction.outcome)
            {
                return Err(format!(
                    "bundled General SEM interaction {} lacks required strong-hierarchy path {} -> {}",
                    interaction.id, main_effect, interaction.outcome
                ));
            }
        }
    }
    Ok(())
}

fn migrate_source_authority(
    sample: &BundledGeneralSemSampleV1,
) -> Result<(AnalysisRecipeV4, SemModelV4), String> {
    let source_recipe = AnalysisRecipe {
        schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        id: sample.recipe_id,
        created_at: sample.created_at,
        dataset_fingerprint: sample.dataset.fingerprint.0.clone(),
        model: sample.source_model.clone(),
        settings: sample.settings.clone(),
        method_config: Some(sample.method_config.clone()),
        metadata: sample.metadata.clone(),
    };
    let pending = migrate_analysis_recipe_to_v4_pending(&source_recipe)
        .map_err(|error| format!("bundled General SEM recipe migration failed: {error}"))?;
    let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
        &pending,
        &sample.source_model,
        &[],
        LegacyBasicModelInterpretationV4::PlsComposite,
    )
    .map_err(|error| format!("bundled General SEM estimand confirmation failed: {error}"))?;
    let SemDataBindingV4::Raw {
        dataset_id,
        missing_data,
        weight,
        cluster_variable,
        strata_variable,
    } = &mut model.data_binding
    else {
        return Err("migrated bundled General SEM model lost its raw-data binding".into());
    };
    *dataset_id = sample.dataset.id.to_string();
    *missing_data = MissingDataPolicyV4::ListwiseDeletion;
    *weight = None;
    *cluster_variable = None;
    *strata_variable = None;
    recipe.general_sem_config = Some(GeneralSemConfigV1::default());
    recipe.metadata.insert(
        "execution_surface".into(),
        GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1.into(),
    );
    recipe
        .metadata
        .insert("general_sem_generation".into(), "general_sem_v1".into());
    Ok((recipe, model))
}

fn add_catalog_interaction(
    model: &mut SemModelV4,
    interaction: &InteractionTerm,
) -> Result<(), String> {
    let predictor = format!("construct:{}", interaction.predictor);
    let moderator = format!("construct:{}", interaction.moderator);
    let outcome = format!("construct:{}", interaction.outcome);
    let focal_relation = model
        .relations
        .iter()
        .find_map(|relation| match relation {
            SemRelationV4::Structural {
                id, source, target, ..
            } if source == &predictor && target == &outcome => Some(id.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            format!(
                "bundled General SEM interaction {} has no focal relation {} -> {}",
                interaction.id, interaction.predictor, interaction.outcome
            )
        })?;
    let output = format!("derived:{}", interaction.product_construct);
    let relation_id = format!("structural:interaction:{}", interaction.id);
    let parameter_id = format!("regression:interaction:{}", interaction.id);
    model.variables.push(SemVariableV4::Derived {
        id: output.clone(),
        label: interaction.product_construct.clone(),
    });
    model.relations.push(SemRelationV4::Structural {
        id: relation_id,
        source: output.clone(),
        target: outcome.clone(),
        parameter: parameter_id.clone(),
        role: StructuralRelationRoleV4::Structural,
        intercept_parameter: None,
    });
    model.parameters.push(SemParameterV4::Free {
        id: parameter_id,
        label: format!(
            "{} -> {}",
            interaction.product_construct, interaction.outcome
        ),
        target: SemParameterTargetV4::Regression {
            source: output.clone(),
            target: outcome,
        },
        start: None,
        lower: None,
        upper: None,
        equality_label: None,
        group_overrides: Vec::new(),
    });
    model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
        id: interaction.id.clone(),
        output,
        operands: vec![predictor, moderator],
        focal_relation,
        method: InteractionMethodV4::TwoStage,
        hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
        product_indicator: None,
    });
    Ok(())
}

fn owned_sample_temp_directory() -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join(SAMPLE_TEMP_DIRECTORY);
    fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "could not create the QuickPLS bundled-sample temp directory {}: {error}",
            directory.display()
        )
    })?;
    let metadata = fs::symlink_metadata(&directory).map_err(|error| error.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("QuickPLS bundled-sample temp path is not an owned ordinary directory".into());
    }
    directory
        .canonicalize()
        .map_err(|error| format!("could not resolve bundled-sample temp directory: {error}"))
}

fn cleanup_stale_materialized_samples(directory: &Path, active_process_id: u32) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    // A sample created by this process may still be the active immutable
    // revision source, even when the app has remained open for over a day.
    let active_process_marker = format!("-{active_process_id}-");
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.file_type().is_file()
            || !is_materialized_sample_file_name(name)
            || name.contains(&active_process_marker)
        {
            continue;
        }
        let is_stale = metadata
            .modified()
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .is_some_and(|age| age >= STALE_SAMPLE_AGE);
        if is_stale {
            // This is deliberately non-recursive and limited to ordinary files
            // directly inside QuickPLS's exact owned temp directory.
            let _ = fs::remove_file(path);
        }
    }
}

fn is_materialized_sample_file_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".qpls") else {
        return false;
    };
    let Some(uuid_start) = stem.len().checked_sub(36) else {
        return false;
    };
    uuid_start > 1
        && stem.as_bytes().get(uuid_start - 1) == Some(&b'-')
        && Uuid::parse_str(&stem[uuid_start..]).is_ok()
}

fn safe_file_stem(value: &str) -> String {
    let stem = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if stem.is_empty() {
        "general-sem-sample".into()
    } else {
        stem
    }
}
