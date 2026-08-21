use super::archive_integrity::{MANIFEST_ENTRY_NAME, PROJECT_ENTRY_NAME};
use super::{
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V1,
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V2,
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V3,
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V4,
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V5,
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V6,
    CBSEM_CURRENT_MEAN_STRUCTURE_EXECUTION_ADAPTER_VERSION_V1,
    CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V1,
    CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V2, CanonicalMissingReasonV2,
    CanonicalResultCellV2, CanonicalResultDocumentAttachmentV2, CanonicalResultDocumentV2,
    CanonicalResultDocumentV2Error, CanonicalResultTableV2, MEAN_REPLACEMENT_CELLS_TABLE_ID_V1,
    MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1, MISSING_DATA_EXECUTION_TABLE_ID_V1,
    MissingDataExecutionDocumentV1Error, PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2,
    PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2, PlsScoreExecutionDocumentV2Error,
    ProjectDataLineageV1Error, validate_project_data_lineage_descriptors_v1,
    validate_project_data_lineage_resident_v1,
    validate_recipe_v4_cbsem_current_execution_document_v1,
    validate_recipe_v4_cbsem_missing_data_execution_document_v1,
    validate_recipe_v4_pls_score_execution_document_v2,
};
use super::{
    Project, ProjectError, ProjectManifest, load_project_archive_v6,
    reject_duplicate_json_object_keys,
};
use chrono::{DateTime, Utc};
use qpls_core::{
    AnalysisMethod, AnalysisRecipe, AnalysisRecipeModelBindingV4, AnalysisRecipeV4,
    AnalysisRecipeV4Error, CanonicalGeneralSemBootstrapIntervalV1,
    CanonicalGeneralSemInferenceTailV1, CbsemBootstrapAlgorithm, CbsemBootstrapInterval,
    CbsemBootstrapTestTail, CbsemEstimator, CbsemInput, CbsemModelType,
    CompiledCbsemParameterStatusV2, CompiledRecipePlanV4, GENERAL_SEM_EFFECTS_V1_METHOD_VERSION,
    GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1, GeneralSemBootstrapIntervalV1,
    GeneralSemInferenceTailV1, GeneralSemInferenceV1, LegacyBasicModelConversionErrorV4,
    LegacyBasicModelInterpretationV4, LegacyDisplayCovarianceV4, MethodConfig, ModelSpec,
    PLS_ALGORITHM_CAPABILITY_ID, PLS_ALGORITHM_CAPABILITY_VERSION, PLS_ALGORITHM_CELL_ID,
    PLS_NONLINEAR_EFFECTS_CAPABILITY_ID, PLS_NONLINEAR_EFFECTS_CAPABILITY_VERSION,
    PLS_NONLINEAR_EFFECTS_CELL_ID, RecipeV4CompilerTarget, SemDerivedTermV4, SemEndpointV4,
    SemModelV4, SemModelV4ValidationError, SemParameterTargetV4, SemVariableV4,
    compile_analysis_recipe_v4, compile_cbsem_exact_case_bootstrap_zero_null_eligibility_v1,
    compile_general_sem_pls_recipe_v1, confirm_legacy_recipe_estimand_v4,
    convert_legacy_basic_model_v4, sha256_serialized,
};
use qpls_data::DatasetDescriptor;
use qpls_estimation::{
    CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1, CBSEM_CFA_SCORE_LM_SCOPE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_BASE_POINT_DIGEST_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ACCELERATION_METHOD_V2,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ADJUSTMENT_METHOD_V2,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_BIAS_CORRECTION_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_QUANTILE_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_RETRY_POLICY_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_INTERVAL_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_ARCHIVE_VALIDATION_SCOPE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_INTERVAL_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_PIVOT_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_QUANTILE_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_DECISION_RULE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_NULL_HYPOTHESIS_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_PROBABILITY_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_SIGNIFICANCE_LEVEL_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_STATISTIC_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_TIE_POLICY_V1,
    CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3, CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
    CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1, CBSEM_FIT_METHOD_VERSION,
    CbsemCfaScoreLmBundleV1, CbsemCfaScoreLmOutcomeV1, CbsemCfaScoreLmRowV1,
    CbsemCfaScoreLmUnavailableReasonV1, CbsemExactCaseBootstrapBasePointDigestProjectionV1,
    CbsemExactCaseBootstrapBasePointParameterV1, CbsemExactCaseBootstrapBcaInferenceV1,
    CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1, CbsemExactCaseBootstrapBcaSidecarV1,
    CbsemExactCaseBootstrapBcaUnavailableReasonV1, CbsemExactCaseBootstrapDeleteOneFailureV1,
    CbsemExactCaseBootstrapDeleteOneWitnessV1, CbsemExactCaseBootstrapFailureKindV1,
    CbsemExactCaseBootstrapFailureV1, CbsemExactCaseBootstrapHypothesisTestInferenceV1,
    CbsemExactCaseBootstrapHypothesisTestOutcomeV1,
    CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1,
    CbsemExactCaseBootstrapHypothesisTestsV1, CbsemExactCaseBootstrapInferenceV1,
    CbsemExactCaseBootstrapParameterEstimateV1, CbsemExactCaseBootstrapParameterIntervalV1,
    CbsemExactCaseBootstrapParameterStandardErrorV1,
    CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1,
    CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1,
    CbsemExactCaseBootstrapRefitStandardErrorsV1, CbsemExactCaseBootstrapRefitV1,
    CbsemExactCaseBootstrapResultV1, CbsemExactCaseBootstrapStudentizedInferenceV1,
    CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1,
    CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1,
    CbsemExactCaseBootstrapStudentizedSidecarV1, CbsemExactCaseBootstrapWitnessV1,
    GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1, NONLINEAR_EFFECTS_METHOD_VERSION,
    PLS_METHOD_VERSION, PLS_SCORE_EXECUTION_METHOD_VERSION_V2,
    cbsem_exact_case_bootstrap_base_point_sha256_v1, cbsem_exact_case_bootstrap_index_digest_v1,
    cbsem_exact_case_bootstrap_sampling_positions_digest_v1,
    cbsem_exact_rmsea_90_percent_interval_v1,
};
use qpls_resampling::{
    CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_MAXIMUM_REPLICATES_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_REQUESTED_REPLICATES_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1,
    CbsemExactCaseBootstrapHypothesisTestPlanV1, bootstrap_indices,
    cbsem_exact_case_bootstrap_schedule_positions_digest_v1,
    recompute_cbsem_exact_case_bootstrap_bca_sidecar_v1,
    recompute_cbsem_exact_case_bootstrap_studentized_sidecar_v1, required_usable_refits,
    summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use statrs::distribution::{ChiSquared, ContinuousCDF};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

/// Staged archive contract. The live archive reader/writer remains schema v5
/// until its GUI, CLI, and estimator consumers are migrated together.
/// This contract phase deliberately preserves layout values verbatim. Live ZIP
/// integration, Arrow-entry preservation, native/TypeScript wiring, and any
/// presentation normalization belong to the next cutover phase.
pub const PROJECT_ARCHIVE_SCHEMA_V6_VERSION: u32 = 6;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectArchiveSchemaAccessV6 {
    HistoricalUpgradeCopyRequired,
    CurrentEditable,
    FutureReadOnly,
}

pub fn classify_project_archive_schema_v6(
    schema_version: u32,
) -> Result<ProjectArchiveSchemaAccessV6, ProjectArchiveV6Error> {
    match schema_version {
        0 => Err(ProjectArchiveV6Error::UnsupportedSchemaZero),
        1..=5 => Ok(ProjectArchiveSchemaAccessV6::HistoricalUpgradeCopyRequired),
        PROJECT_ARCHIVE_SCHEMA_V6_VERSION => Ok(ProjectArchiveSchemaAccessV6::CurrentEditable),
        _ => Ok(ProjectArchiveSchemaAccessV6::FutureReadOnly),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourcePreservationPolicyV6 {
    Required,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeWritePolicyV6 {
    NewArchiveOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectUpgradeLineageV6 {
    pub source_project_id: Uuid,
    pub source_archive_schema_version: u32,
    pub source_archive_sha256: String,
    pub source_archive_path: String,
    pub destination_archive_path: String,
    pub upgraded_at: DateTime<Utc>,
    pub source_preservation: SourcePreservationPolicyV6,
    pub write_policy: UpgradeWritePolicyV6,
    pub historical_results_immutable: bool,
}

/// Truthful project provenance. A newly authored schema-v6 project has no
/// synthetic upgrade lineage; an upgraded copy carries the complete immutable
/// source binding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectOriginV6 {
    NewProject,
    UpgradedCopy { lineage: ProjectUpgradeLineageV6 },
}

/// Explicit product-generation authority for newly authored schema-v6 projects.
///
/// Absence preserves the behavior of existing schema-v6 documents. The marker
/// is intentionally unavailable to upgraded copies so opening or saving legacy
/// work can never opt it into advanced General SEM semantics implicitly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectSemGenerationV6 {
    GeneralSemV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectModelPayloadV6 {
    SemModelV4 {
        model: SemModelV4,
        scientific_sha256: String,
    },
    /// An authoring-integrity-checked model that is intentionally not ready for
    /// recipe binding, compilation, or execution.
    SemModelV4Draft {
        model: SemModelV4,
        model_document_sha256: String,
    },
    LegacyEstimandUnspecified {
        legacy_model: ModelSpec,
        legacy_model_sha256: String,
        #[serde(default)]
        display_covariances: Vec<LegacyDisplayCovarianceV4>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        automatic_conversion_blocker: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProjectModelRecordV6 {
    pub model_id: String,
    pub payload: ProjectModelPayloadV6,
}

/// Original schema-1-through-3 recipe semantics retained independently from
/// newly authored RecipeV4 records. Fields are private to discourage mutation
/// of historical execution inputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImmutableHistoricalRecipeV6 {
    recipe_id: Uuid,
    source_recipe_schema_version: u32,
    recipe_document: Value,
    recipe_document_sha256: String,
}

impl ImmutableHistoricalRecipeV6 {
    fn from_recipe(recipe: &AnalysisRecipe) -> Self {
        let recipe_document = serde_json::to_value(recipe).expect("AnalysisRecipe must serialize");
        Self {
            recipe_id: recipe.id,
            source_recipe_schema_version: recipe.schema_version,
            recipe_document_sha256: sha256_json(&recipe_document),
            recipe_document,
        }
    }

    pub fn recipe_id(&self) -> Uuid {
        self.recipe_id
    }

    pub fn source_recipe_schema_version(&self) -> u32 {
        self.source_recipe_schema_version
    }

    pub fn recipe_document(&self) -> &Value {
        &self.recipe_document
    }

    pub fn recipe_document_sha256(&self) -> &str {
        &self.recipe_document_sha256
    }

    pub fn ensure_valid(&self) -> Result<(), ProjectArchiveV6Error> {
        if !(1..=3).contains(&self.source_recipe_schema_version)
            || self
                .recipe_document
                .get("schema_version")
                .and_then(Value::as_u64)
                != Some(u64::from(self.source_recipe_schema_version))
        {
            return Err(ProjectArchiveV6Error::HistoricalRecipeSchema(
                self.recipe_id,
            ));
        }
        if self.recipe_document.get("id").and_then(Value::as_str)
            != Some(self.recipe_id.to_string().as_str())
        {
            return Err(ProjectArchiveV6Error::HistoricalRecipeIdentity(
                self.recipe_id,
            ));
        }
        if sha256_json(&self.recipe_document) != self.recipe_document_sha256 {
            return Err(ProjectArchiveV6Error::HistoricalRecipeDigest(
                self.recipe_id,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HistoricalResultRecipeBindingV6 {
    Bound {
        source_recipe_id: Uuid,
        recipe_document_sha256: String,
    },
    UnboundLegacy,
}

impl Default for HistoricalResultRecipeBindingV6 {
    fn default() -> Self {
        Self::UnboundLegacy
    }
}

/// Historical result bytes are represented by a value plus a mandatory digest.
/// Fields are private so normal callers cannot edit a migrated result in place.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImmutableHistoricalResultV6 {
    result_id: Uuid,
    source_result_schema_version: u32,
    result: Value,
    result_sha256: String,
    #[serde(default)]
    source_recipe: HistoricalResultRecipeBindingV6,
}

impl ImmutableHistoricalResultV6 {
    fn from_result(
        result: &qpls_core::AnalysisResult,
        historical_recipes: &BTreeMap<Uuid, &ImmutableHistoricalRecipeV6>,
        source_provenance_can_bind: bool,
    ) -> Self {
        let value = serde_json::to_value(result).expect("AnalysisResult must serialize");
        let source_recipe = if !source_provenance_can_bind || result.provenance.recipe_id.is_nil() {
            HistoricalResultRecipeBindingV6::UnboundLegacy
        } else if let Some(recipe) = historical_recipes.get(&result.provenance.recipe_id) {
            HistoricalResultRecipeBindingV6::Bound {
                source_recipe_id: recipe.recipe_id(),
                recipe_document_sha256: recipe.recipe_document_sha256().to_owned(),
            }
        } else {
            HistoricalResultRecipeBindingV6::UnboundLegacy
        };
        Self {
            result_id: result.id,
            source_result_schema_version: result.schema_version,
            result_sha256: sha256_json(&value),
            result: value,
            source_recipe,
        }
    }

    pub fn result_id(&self) -> Uuid {
        self.result_id
    }

    pub fn source_result_schema_version(&self) -> u32 {
        self.source_result_schema_version
    }

    pub fn result(&self) -> &Value {
        &self.result
    }

    pub fn result_sha256(&self) -> &str {
        &self.result_sha256
    }

    pub fn source_recipe(&self) -> &HistoricalResultRecipeBindingV6 {
        &self.source_recipe
    }

    pub fn ensure_valid(&self) -> Result<(), ProjectArchiveV6Error> {
        if self.result.get("id").and_then(Value::as_str)
            != Some(self.result_id.to_string().as_str())
        {
            return Err(ProjectArchiveV6Error::HistoricalResultIdentity(
                self.result_id,
            ));
        }
        if self.result.get("schema_version").and_then(Value::as_u64)
            != Some(u64::from(self.source_result_schema_version))
        {
            return Err(ProjectArchiveV6Error::HistoricalResultSchema(
                self.result_id,
            ));
        }
        if sha256_json(&self.result) != self.result_sha256 {
            return Err(ProjectArchiveV6Error::HistoricalResultDigest(
                self.result_id,
            ));
        }
        Ok(())
    }

    fn ensure_recipe_binding(
        &self,
        historical_recipes: &BTreeMap<Uuid, &str>,
        source_provenance_can_bind: bool,
    ) -> Result<(), ProjectArchiveV6Error> {
        let embedded_recipe_id = self
            .result
            .get("provenance")
            .and_then(|provenance| provenance.get("recipe_id"))
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok());
        match &self.source_recipe {
            HistoricalResultRecipeBindingV6::Bound {
                source_recipe_id,
                recipe_document_sha256,
            } => {
                if !source_provenance_can_bind
                    || embedded_recipe_id != Some(*source_recipe_id)
                    || historical_recipes.get(source_recipe_id).copied()
                        != Some(recipe_document_sha256.as_str())
                {
                    return Err(ProjectArchiveV6Error::HistoricalResultRecipeBinding(
                        self.result_id,
                    ));
                }
            }
            HistoricalResultRecipeBindingV6::UnboundLegacy => {
                if source_provenance_can_bind
                    && embedded_recipe_id.is_some_and(|recipe_id| {
                        !recipe_id.is_nil() && historical_recipes.contains_key(&recipe_id)
                    })
                {
                    return Err(ProjectArchiveV6Error::HistoricalResultRecipeBinding(
                        self.result_id,
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectArchiveDocumentV6 {
    pub schema_version: u32,
    pub project_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    #[serde(default)]
    pub datasets: Vec<DatasetDescriptor>,
    #[serde(default)]
    pub models: Vec<ProjectModelRecordV6>,
    /// RecipeV4 is a separate current-authoring lane. Migration never inserts
    /// historical schema-1-through-3 recipes here or reuses their identifiers.
    #[serde(default)]
    pub recipes: Vec<AnalysisRecipeV4>,
    #[serde(default)]
    pub historical_recipes: Vec<ImmutableHistoricalRecipeV6>,
    #[serde(default)]
    pub layouts: BTreeMap<String, Value>,
    #[serde(default)]
    pub historical_results: Vec<ImmutableHistoricalResultV6>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub canonical_result_documents: Vec<CanonicalResultDocumentAttachmentV2>,
    pub origin: ProjectOriginV6,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sem_generation: Option<ProjectSemGenerationV6>,
}

/// Compatibility-only reader for the staged internal schema-v6 foundation.
/// New serialization emits `origin`; the former top-level `upgrade_lineage`
/// field is accepted only while reading and is normalized to upgraded_copy.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectArchiveDocumentV6Wire {
    schema_version: u32,
    project_id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    modified_at: DateTime<Utc>,
    #[serde(default)]
    datasets: Vec<DatasetDescriptor>,
    #[serde(default)]
    models: Vec<ProjectModelRecordV6>,
    #[serde(default)]
    recipes: Vec<AnalysisRecipeV4>,
    #[serde(default)]
    historical_recipes: Vec<ImmutableHistoricalRecipeV6>,
    #[serde(default)]
    layouts: BTreeMap<String, Value>,
    #[serde(default)]
    historical_results: Vec<ImmutableHistoricalResultV6>,
    #[serde(default)]
    canonical_result_documents: Vec<CanonicalResultDocumentAttachmentV2>,
    #[serde(default)]
    origin: Option<ProjectOriginV6>,
    #[serde(default)]
    upgrade_lineage: Option<ProjectUpgradeLineageV6>,
    #[serde(default)]
    sem_generation: Option<ProjectSemGenerationV6>,
}

impl<'de> Deserialize<'de> for ProjectArchiveDocumentV6 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ProjectArchiveDocumentV6Wire::deserialize(deserializer)?;
        let origin = match (wire.origin, wire.upgrade_lineage) {
            (Some(origin), None) => origin,
            (None, Some(lineage)) => ProjectOriginV6::UpgradedCopy { lineage },
            (Some(_), Some(_)) => {
                return Err(<D::Error as serde::de::Error>::custom(
                    "project schema v6 cannot contain both origin and legacy upgrade_lineage",
                ));
            }
            (None, None) => {
                return Err(<D::Error as serde::de::Error>::missing_field("origin"));
            }
        };
        Ok(Self {
            schema_version: wire.schema_version,
            project_id: wire.project_id,
            name: wire.name,
            created_at: wire.created_at,
            modified_at: wire.modified_at,
            datasets: wire.datasets,
            models: wire.models,
            recipes: wire.recipes,
            historical_recipes: wire.historical_recipes,
            layouts: wire.layouts,
            historical_results: wire.historical_results,
            canonical_result_documents: wire.canonical_result_documents,
            origin,
            sem_generation: wire.sem_generation,
        })
    }
}

impl ProjectArchiveDocumentV6 {
    /// Creates a blank project that is explicitly authorized for the advanced
    /// General SEM authoring and estimator-capability workflow.
    pub fn new_general_sem_v1(
        project_id: Uuid,
        name: impl Into<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
            project_id,
            name: name.into(),
            created_at,
            modified_at: created_at,
            datasets: Vec::new(),
            models: Vec::new(),
            recipes: Vec::new(),
            historical_recipes: Vec::new(),
            layouts: BTreeMap::new(),
            historical_results: Vec::new(),
            canonical_result_documents: Vec::new(),
            origin: ProjectOriginV6::NewProject,
            sem_generation: Some(ProjectSemGenerationV6::GeneralSemV1),
        }
    }

    pub fn supports_general_sem_v1(&self) -> bool {
        matches!(self.origin, ProjectOriginV6::NewProject)
            && self.sem_generation == Some(ProjectSemGenerationV6::GeneralSemV1)
    }

    pub fn upgrade_lineage(&self) -> Option<&ProjectUpgradeLineageV6> {
        match &self.origin {
            ProjectOriginV6::NewProject => None,
            ProjectOriginV6::UpgradedCopy { lineage } => Some(lineage),
        }
    }

    pub fn ensure_valid(&self) -> Result<(), ProjectArchiveV6Error> {
        if self.schema_version != PROJECT_ARCHIVE_SCHEMA_V6_VERSION {
            return Err(ProjectArchiveV6Error::Schema(self.schema_version));
        }
        if let ProjectOriginV6::UpgradedCopy { lineage } = &self.origin {
            validate_upgrade_lineage(lineage)?;
            if lineage.source_project_id != self.project_id {
                return Err(ProjectArchiveV6Error::UpgradeProjectIdentity);
            }
            if self.sem_generation.is_some() {
                return Err(ProjectArchiveV6Error::GeneralSemGenerationRequiresNewProject);
            }
        }
        // Schema v6 stores dataset descriptors, not Arrow buffers. This gate
        // validates identity, graph, shape, and transform-receipt bindings but
        // deliberately does not claim raw-row transformation replay.
        validate_project_data_lineage_descriptors_v1(&self.datasets, &self.layouts)?;

        let mut model_ids = BTreeSet::new();
        let mut scientific_models = BTreeMap::<&str, (&SemModelV4, &str)>::new();
        let mut pending_models = BTreeMap::<&str, (&ModelSpec, &str)>::new();
        for record in &self.models {
            if record.model_id.trim().is_empty() || !model_ids.insert(record.model_id.as_str()) {
                return Err(ProjectArchiveV6Error::DuplicateOrEmptyModelId(
                    record.model_id.clone(),
                ));
            }
            match &record.payload {
                ProjectModelPayloadV6::SemModelV4 {
                    model,
                    scientific_sha256,
                } => {
                    model.ensure_valid()?;
                    if model.id != record.model_id
                        || model.scientific_sha256()? != *scientific_sha256
                    {
                        return Err(ProjectArchiveV6Error::ModelDigestOrIdentity(
                            record.model_id.clone(),
                        ));
                    }
                    scientific_models.insert(&record.model_id, (model, scientific_sha256));
                }
                ProjectModelPayloadV6::SemModelV4Draft {
                    model,
                    model_document_sha256,
                } => {
                    model.ensure_authoring_integrity()?;
                    if model.id != record.model_id
                        || model.model_document_sha256()? != *model_document_sha256
                    {
                        return Err(ProjectArchiveV6Error::ModelDigestOrIdentity(
                            record.model_id.clone(),
                        ));
                    }
                }
                ProjectModelPayloadV6::LegacyEstimandUnspecified {
                    legacy_model,
                    legacy_model_sha256,
                    display_covariances,
                    ..
                } => {
                    if legacy_model.id.to_string() != record.model_id
                        || sha256_serialized(legacy_model) != *legacy_model_sha256
                    {
                        return Err(ProjectArchiveV6Error::ModelDigestOrIdentity(
                            record.model_id.clone(),
                        ));
                    }
                    validate_display_covariances(legacy_model, display_covariances)?;
                    pending_models.insert(&record.model_id, (legacy_model, legacy_model_sha256));
                }
            }
        }

        let mut recipe_ids = BTreeSet::new();
        let mut historical_recipe_digests = BTreeMap::<Uuid, &str>::new();
        for recipe in &self.historical_recipes {
            recipe.ensure_valid()?;
            if !recipe_ids.insert(recipe.recipe_id()) {
                return Err(ProjectArchiveV6Error::DuplicateRecipeId(recipe.recipe_id()));
            }
            historical_recipe_digests.insert(recipe.recipe_id(), recipe.recipe_document_sha256());
        }
        for recipe in &self.recipes {
            recipe.ensure_valid()?;
            if recipe.general_sem_config.is_some() && !self.supports_general_sem_v1() {
                return Err(ProjectArchiveV6Error::GeneralSemFeatureRequiresGeneration {
                    subject: format!("analysis recipe {}", recipe.id),
                });
            }
            if !recipe_ids.insert(recipe.id) {
                return Err(ProjectArchiveV6Error::DuplicateRecipeId(recipe.id));
            }
            match &recipe.model_binding {
                AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 { model, .. } => {
                    ensure_general_sem_v1_model_authority(self, model)?;
                }
                AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                    model_id,
                    scientific_sha256,
                } => {
                    let Some((model, stored_sha256)) = scientific_models.get(model_id.as_str())
                    else {
                        return Err(ProjectArchiveV6Error::RecipeModelReference {
                            recipe_id: recipe.id,
                            model_id: model_id.clone(),
                        });
                    };
                    ensure_general_sem_v1_model_authority(self, model)?;
                    if *stored_sha256 != scientific_sha256 {
                        return Err(ProjectArchiveV6Error::RecipeModelDigest(recipe.id));
                    }
                }
                AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
                    legacy_model_id,
                    legacy_model_sha256,
                } => {
                    let Some((_, stored_sha256)) = pending_models.get(legacy_model_id.as_str())
                    else {
                        return Err(ProjectArchiveV6Error::RecipeModelReference {
                            recipe_id: recipe.id,
                            model_id: legacy_model_id.clone(),
                        });
                    };
                    if *stored_sha256 != legacy_model_sha256 {
                        return Err(ProjectArchiveV6Error::RecipeModelDigest(recipe.id));
                    }
                }
            }
        }

        let mut result_ids = BTreeSet::new();
        let source_provenance_can_bind = match &self.origin {
            ProjectOriginV6::NewProject => true,
            ProjectOriginV6::UpgradedCopy { lineage } => lineage.source_archive_schema_version >= 3,
        };
        for result in &self.historical_results {
            result.ensure_valid()?;
            result.ensure_recipe_binding(&historical_recipe_digests, source_provenance_can_bind)?;
            if !result_ids.insert(result.result_id()) {
                return Err(ProjectArchiveV6Error::DuplicateHistoricalResultId(
                    result.result_id(),
                ));
            }
        }
        let expected_project_id = self.project_id.to_string();
        let mut canonical_document_ids = BTreeSet::new();
        let mut canonical_run_ids = BTreeSet::new();
        for attachment in &self.canonical_result_documents {
            attachment.ensure_valid(&expected_project_id)?;
            let canonical = attachment.canonical_document();
            if canonical.general_sem_results.is_some() {
                if !self.supports_general_sem_v1() {
                    return Err(ProjectArchiveV6Error::GeneralSemFeatureRequiresGeneration {
                        subject: format!("canonical result document {}", canonical.document_id),
                    });
                }
                let recipe = self
                    .recipes
                    .iter()
                    .find(|recipe| recipe.id.to_string() == canonical.provenance.recipe_id)
                    .ok_or_else(|| {
                        invalid_general_sem_authority(format!(
                            "resident Recipe-v4 {} is unavailable",
                            canonical.provenance.recipe_id
                        ))
                    })?;
                let model = match &recipe.model_binding {
                    AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 { model, .. } => model,
                    AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                        model_id, ..
                    } => scientific_models
                        .get(model_id.as_str())
                        .map(|(model, _)| *model)
                        .ok_or_else(|| {
                            invalid_general_sem_authority(format!(
                                "resident SemModelV4 {model_id} is unavailable"
                            ))
                        })?,
                    AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
                        legacy_model_id,
                        ..
                    } => {
                        return Err(invalid_general_sem_authority(format!(
                            "General SEM result cannot bind legacy estimand-unspecified model {legacy_model_id}"
                        )));
                    }
                };
                let dataset = self
                    .datasets
                    .iter()
                    .find(|dataset| dataset.id.to_string() == canonical.provenance.dataset_id)
                    .ok_or_else(|| {
                        invalid_general_sem_authority(format!(
                            "resident dataset {} is unavailable",
                            canonical.provenance.dataset_id
                        ))
                    })?;
                validate_general_sem_result_authority_v1(canonical, recipe, model, dataset)?;
            }
            if is_exact_recipe_v4_cbsem_result(canonical) {
                let is_score_lm_current = matches!(
                    canonical.provenance.engine_version.as_str(),
                    CBSEM_EXACT_ADAPTER_LISTWISE_SCORE_LM_CURRENT
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_HISTORICAL
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_CURRENT
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_STUDENTIZED_CURRENT
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_BCA_CURRENT
                );
                let has_score_lm_artifacts = canonical.tables.iter().any(|table| {
                    matches!(
                        table.id.as_str(),
                        CBSEM_SCORE_LM_TABLE_ID_V1 | "modification_indices"
                    )
                }) || canonical
                    .sections
                    .iter()
                    .any(|section| section.id == CBSEM_SCORE_LM_SECTION_ID_V1);
                if !is_score_lm_current && has_score_lm_artifacts {
                    return Err(invalid_cbsem_rmsea(
                        "legacy CB-SEM adapter carries a score/LM or heuristic MI artifact",
                    ));
                }
                let has_exact_bootstrap_artifacts = canonical.tables.iter().any(|table| {
                    matches!(
                        table.id.as_str(),
                        CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID
                            | CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID
                            | CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID
                            | CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID
                    )
                }) || canonical
                    .sections
                    .iter()
                    .any(|section| section.id == CBSEM_EXACT_BOOTSTRAP_SECTION_ID);
                let is_exact_bootstrap_adapter = matches!(
                    canonical.provenance.engine_version.as_str(),
                    CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_HISTORICAL
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_CURRENT
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_STUDENTIZED_CURRENT
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_BCA_CURRENT
                );
                if !is_exact_bootstrap_adapter && has_exact_bootstrap_artifacts {
                    return Err(invalid_cbsem_rmsea(
                        "non-bootstrap CB-SEM adapters cannot carry exact case-bootstrap artifacts",
                    ));
                }
                let has_exact_bootstrap_hypothesis_artifacts = canonical
                    .tables
                    .iter()
                    .any(|table| table.id == CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID)
                    || canonical
                        .sections
                        .iter()
                        .any(|section| section.id == CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID);
                if !matches!(
                    canonical.provenance.engine_version.as_str(),
                    CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_CURRENT
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_STUDENTIZED_CURRENT
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_BCA_CURRENT
                ) && has_exact_bootstrap_hypothesis_artifacts
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v2-v9 cannot carry exact case-bootstrap hypothesis artifacts",
                    ));
                }
                let has_missing_data_tables = canonical.tables.iter().any(|table| {
                    matches!(
                        table.id.as_str(),
                        MISSING_DATA_EXECUTION_TABLE_ID_V1
                            | MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1
                            | MEAN_REPLACEMENT_CELLS_TABLE_ID_V1
                    )
                });
                let current_adapter = matches!(
                    canonical.provenance.engine_version.as_str(),
                    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V1
                        | CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V2
                        | CBSEM_CURRENT_MEAN_STRUCTURE_EXECUTION_ADAPTER_VERSION_V1
                        | CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V2
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_HISTORICAL
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_CURRENT
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_STUDENTIZED_CURRENT
                        | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_BCA_CURRENT
                );
                let recipe = self
                    .recipes
                    .iter()
                    .find(|recipe| recipe.id.to_string() == canonical.provenance.recipe_id);
                if let Some(recipe) = recipe {
                    let model = match &recipe.model_binding {
                        AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 { model, .. } => model,
                        AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                            model_id,
                            ..
                        } => scientific_models
                            .get(model_id.as_str())
                            .map(|(model, _)| *model)
                            .ok_or_else(|| {
                                ProjectArchiveV6Error::CanonicalCbsemModelUnavailable(
                                    model_id.clone(),
                                )
                            })?,
                        AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
                            legacy_model_id,
                            ..
                        } => {
                            return Err(ProjectArchiveV6Error::CanonicalCbsemModelUnavailable(
                                legacy_model_id.clone(),
                            ));
                        }
                    };
                    let dataset = self
                        .datasets
                        .iter()
                        .find(|dataset| dataset.id.to_string() == canonical.provenance.dataset_id)
                        .ok_or_else(|| {
                            ProjectArchiveV6Error::CanonicalCbsemDatasetUnavailable(
                                canonical.provenance.dataset_id.clone(),
                            )
                        })?;
                    if canonical.provenance.engine_version
                        == CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V2
                    {
                        // v7 first receives the same deterministic current-
                        // adapter cross-binding as v5/v6, then its stricter
                        // descriptor/receipt/table validation.
                        validate_recipe_v4_cbsem_missing_data_execution_document_v1(
                            canonical, recipe, model, dataset,
                        )?;
                    } else if current_adapter {
                        validate_recipe_v4_cbsem_current_execution_document_v1(
                            canonical, recipe, model, dataset,
                        )?;
                    } else {
                        validate_recipe_v4_cbsem_missing_data_execution_document_v1(
                            canonical, recipe, model, dataset,
                        )?;
                    }
                    validate_recipe_v4_cbsem_score_lm_document_v1(
                        canonical, recipe, model, dataset,
                    )?;
                    validate_recipe_v4_cbsem_exact_bootstrap_document_v1(
                        canonical, recipe, model, dataset,
                    )?;
                } else if current_adapter
                    || has_missing_data_tables
                    || matches!(
                        canonical.provenance.engine_version.as_str(),
                        CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V1
                            | CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V2
                    )
                {
                    return Err(ProjectArchiveV6Error::CanonicalCbsemRecipeUnavailable(
                        canonical.provenance.recipe_id.clone(),
                    ));
                }
                let has_studentized_bootstrap_artifacts = canonical.tables.iter().any(|table| {
                    matches!(
                        table.id.as_str(),
                        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID
                            | CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID
                            | CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID
                            | CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID
                    )
                }) || canonical
                    .sections
                    .iter()
                    .any(|section| section.id == CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID);
                if canonical.provenance.engine_version
                    != CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_STUDENTIZED_CURRENT
                    && has_studentized_bootstrap_artifacts
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v2-v10/v12 cannot carry studentized bootstrap artifacts",
                    ));
                }
                let has_bca_bootstrap_artifacts = canonical.tables.iter().any(|table| {
                    matches!(
                        table.id.as_str(),
                        CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID
                            | CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID
                            | CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID
                            | CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID
                    )
                }) || canonical
                    .sections
                    .iter()
                    .any(|section| section.id == CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID);
                if canonical.provenance.engine_version
                    != CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_BCA_CURRENT
                    && has_bca_bootstrap_artifacts
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v2-v11 cannot carry BCa bootstrap artifacts",
                    ));
                }
                validate_recipe_v4_cbsem_rmsea_fit_document_v1(canonical)?;
            }
            let has_pls_nonlinear_artifacts = has_recipe_v4_pls_nonlinear_artifacts_v1(canonical);
            let is_exact_pls_nonlinear = is_exact_recipe_v4_pls_nonlinear_result(canonical);
            if has_pls_nonlinear_artifacts && !is_exact_pls_nonlinear {
                return Err(invalid_pls_nonlinear(
                    "reserved Recipe-v4 nonlinear artifacts require the exact nonlinear primary capability cell",
                ));
            }
            if is_exact_recipe_v4_pls_result(canonical) {
                if is_exact_pls_nonlinear {
                    let recipe = self
                        .recipes
                        .iter()
                        .find(|recipe| recipe.id.to_string() == canonical.provenance.recipe_id)
                        .ok_or_else(|| {
                            ProjectArchiveV6Error::CanonicalPlsRecipeUnavailable(
                                canonical.provenance.recipe_id.clone(),
                            )
                        })?;
                    let model = match &recipe.model_binding {
                        AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 { model, .. } => model,
                        AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                            model_id,
                            ..
                        } => scientific_models
                            .get(model_id.as_str())
                            .map(|(model, _)| *model)
                            .ok_or_else(|| {
                                ProjectArchiveV6Error::CanonicalPlsModelUnavailable(
                                    model_id.clone(),
                                )
                            })?,
                        AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
                            legacy_model_id,
                            ..
                        } => {
                            return Err(ProjectArchiveV6Error::CanonicalPlsModelUnavailable(
                                legacy_model_id.clone(),
                            ));
                        }
                    };
                    let dataset = self
                        .datasets
                        .iter()
                        .find(|dataset| dataset.id.to_string() == canonical.provenance.dataset_id)
                        .ok_or_else(|| {
                            ProjectArchiveV6Error::CanonicalPlsDatasetUnavailable(
                                canonical.provenance.dataset_id.clone(),
                            )
                        })?;
                    validate_recipe_v4_pls_nonlinear_document_v1(
                        canonical, recipe, model, dataset,
                    )?;
                } else {
                    let has_score_tables = canonical.tables.iter().any(|table| {
                        matches!(
                            table.id.as_str(),
                            PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2
                                | PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2
                        )
                    });
                    if canonical.provenance.method_version == PLS_METHOD_VERSION
                        && !has_score_tables
                    {
                        // Historical plain pls_pm_v1 attachments predate the
                        // explicit SemModelV4 archive binding. Their bytes remain
                        // readable, but an explicit modern recipe is still
                        // recomputed so a v2 result cannot be relabelled as v1.
                        if canonical.provenance.engine_version
                            == crate::RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2
                        {
                            return Err(ProjectArchiveV6Error::CanonicalPlsAdapterMismatch(
                                canonical.provenance.engine_version.clone(),
                            ));
                        }
                        if let Some(recipe) = self
                            .recipes
                            .iter()
                            .find(|recipe| recipe.id.to_string() == canonical.provenance.recipe_id)
                        {
                            match &recipe.model_binding {
                                AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                                    model, ..
                                } => {
                                    validate_recipe_v4_pls_score_execution_document_v2(
                                        canonical, recipe, model,
                                    )?;
                                }
                                AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                                    model_id,
                                    ..
                                } => {
                                    let model = scientific_models
                                        .get(model_id.as_str())
                                        .map(|(model, _)| *model)
                                        .ok_or_else(|| {
                                            ProjectArchiveV6Error::CanonicalPlsModelUnavailable(
                                                model_id.clone(),
                                            )
                                        })?;
                                    validate_recipe_v4_pls_score_execution_document_v2(
                                        canonical, recipe, model,
                                    )?;
                                }
                                AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
                                    ..
                                } => {}
                            }
                        }
                    } else if canonical.provenance.method_version
                        != PLS_SCORE_EXECUTION_METHOD_VERSION_V2
                    {
                        return Err(ProjectArchiveV6Error::CanonicalPlsMethodUnsupported(
                            canonical.provenance.method_version.clone(),
                        ));
                    } else {
                        let recipe = self
                            .recipes
                            .iter()
                            .find(|recipe| recipe.id.to_string() == canonical.provenance.recipe_id)
                            .ok_or_else(|| {
                                ProjectArchiveV6Error::CanonicalPlsRecipeUnavailable(
                                    canonical.provenance.recipe_id.clone(),
                                )
                            })?;
                        let model = match &recipe.model_binding {
                            AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 { model, .. } => model,
                            AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                                model_id,
                                ..
                            } => scientific_models
                                .get(model_id.as_str())
                                .map(|(model, _)| *model)
                                .ok_or_else(|| {
                                    ProjectArchiveV6Error::CanonicalPlsModelUnavailable(
                                        model_id.clone(),
                                    )
                                })?,
                            AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
                                legacy_model_id,
                                ..
                            } => {
                                return Err(ProjectArchiveV6Error::CanonicalPlsModelUnavailable(
                                    legacy_model_id.clone(),
                                ));
                            }
                        };
                        validate_recipe_v4_pls_score_execution_document_v2(
                            canonical, recipe, model,
                        )?;
                    }
                }
            }
            if !canonical_document_ids.insert(attachment.document_id()) {
                return Err(ProjectArchiveV6Error::DuplicateCanonicalResultDocumentId(
                    attachment.document_id().to_owned(),
                ));
            }
            if !canonical_run_ids.insert(attachment.run_id()) {
                return Err(ProjectArchiveV6Error::DuplicateCanonicalResultRunId(
                    attachment.run_id().to_owned(),
                ));
            }
        }
        Ok(())
    }
}

fn invalid_general_sem_authority(message: impl Into<String>) -> ProjectArchiveV6Error {
    ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message.into())
}

// qpls-project cannot depend on qpls-runner without reversing the runtime
// layering. The runner-backed test below locks these archive-recognition
// values to qpls-runner's exported adapter constants so executor identity
// changes cannot drift silently across the persistence boundary.
const GENERAL_SEM_PLS_POINT_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_general_sem_pls_recipe_v1_point_execution_v1";
const GENERAL_SEM_PLS_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_general_sem_pls_recipe_v1_percentile_bootstrap_execution_v1";
const GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_point_execution_v1";
const GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1: &str = "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_percentile_bootstrap_execution_v1";

fn project_capability_cell_v2(
    reference: &qpls_core::CapabilityCellReferenceV2,
) -> crate::CapabilityCellReferenceV2 {
    crate::CapabilityCellReferenceV2 {
        registry_schema_version: reference.registry_schema_version,
        capability_id: reference.capability_id.clone(),
        cell_id: reference.cell_id.clone(),
        capability_version: reference.capability_version.clone(),
    }
}

/// Rebuilds the exact General SEM PLS authority from the schema-6 residents.
/// A canonical attachment is an immutable result, not an alternate source of
/// scientific truth, so every persisted identity must reconcile with this
/// deterministic recompilation before the archive is accepted.
fn validate_general_sem_result_authority_v1(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    dataset: &DatasetDescriptor,
) -> Result<(), ProjectArchiveV6Error> {
    let results = document.general_sem_results.as_ref().ok_or_else(|| {
        invalid_general_sem_authority("General SEM authority validation requires typed results")
    })?;
    let artifact = compile_general_sem_pls_recipe_v1(recipe, Some(model)).map_err(|error| {
        invalid_general_sem_authority(format!(
            "resident General SEM Recipe-v4 recompilation failed: {error}"
        ))
    })?;
    let model_scientific_sha256 = model
        .scientific_sha256()
        .map_err(|error| invalid_general_sem_authority(error.to_string()))?;
    let dataset_id = dataset.id.to_string();
    let resident_config = recipe.general_sem_config.as_ref().ok_or_else(|| {
        invalid_general_sem_authority("the resident Recipe-v4 does not contain GeneralSemConfigV1")
    })?;
    let has_interactions = !artifact.plan().two_way_interactions().is_empty();
    let (expected_method_version, expected_engine_version) =
        match (has_interactions, resident_config.inference) {
            (true, GeneralSemInferenceV1::None) => (
                GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
                GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1,
            ),
            (true, GeneralSemInferenceV1::CaseBootstrap { .. }) => (
                qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
                GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
            ),
            (false, GeneralSemInferenceV1::None) => (
                GENERAL_SEM_EFFECTS_V1_METHOD_VERSION,
                GENERAL_SEM_PLS_POINT_EXECUTION_ADAPTER_VERSION_V1,
            ),
            (false, GeneralSemInferenceV1::CaseBootstrap { .. }) => (
                GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1,
                GENERAL_SEM_PLS_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
            ),
        };

    if document.provenance.recipe_id != recipe.id.to_string()
        || document.provenance.recipe_digest != artifact.recipe_analytical_sha256()
        || document.provenance.model_id != model.id
        || document.provenance.model_digest != model_scientific_sha256
        || document.provenance.dataset_id != dataset_id
        || document.provenance.dataset_id != artifact.plan().base_plan().dataset_id()
        || document.provenance.dataset_fingerprint != dataset.fingerprint.0
        || document.provenance.dataset_fingerprint != recipe.dataset_fingerprint
        || document.provenance.capability_cell
            != project_capability_cell_v2(artifact.capability_cell())
        || document.provenance.method_version != expected_method_version
        || document.provenance.engine_version != expected_engine_version
        || document.provenance.seed != Some(recipe.settings.seed)
        || usize::try_from(document.provenance.workers).ok() != Some(recipe.settings.workers)
    {
        return Err(invalid_general_sem_authority(
            "canonical provenance differs from the resident General SEM recipe, model, dataset, method/engine, or deterministic compilation",
        ));
    }

    validate_general_sem_pls_interaction_authority_v1(document, results, &artifact, recipe)?;

    match (&recipe.general_sem_config, &results.inference_receipt) {
        (Some(config), None) if matches!(config.inference, GeneralSemInferenceV1::None) => Ok(()),
        (Some(config), Some(receipt)) => {
            let GeneralSemInferenceV1::CaseBootstrap {
                resamples,
                seed,
                confidence_level,
                interval,
                tail,
            } = config.inference
            else {
                return Err(invalid_general_sem_authority(
                    "a General SEM inference receipt requires case-bootstrap inference in the resident recipe",
                ));
            };
            let expected_interval = match interval {
                GeneralSemBootstrapIntervalV1::Percentile => {
                    CanonicalGeneralSemBootstrapIntervalV1::PercentileType7
                }
                GeneralSemBootstrapIntervalV1::Bca => CanonicalGeneralSemBootstrapIntervalV1::Bca,
            };
            let expected_tail = match tail {
                GeneralSemInferenceTailV1::TwoSided => CanonicalGeneralSemInferenceTailV1::TwoSided,
                GeneralSemInferenceTailV1::OneSidedLower => {
                    CanonicalGeneralSemInferenceTailV1::OneSidedLower
                }
                GeneralSemInferenceTailV1::OneSidedUpper => {
                    CanonicalGeneralSemInferenceTailV1::OneSidedUpper
                }
            };
            let receipt_seed = receipt.seed.parse::<u64>().ok();
            if receipt.compilation_artifact_identity_sha256 != artifact.artifact_identity_sha256()
                || receipt.compiled_plan_sha256 != artifact.plan().deterministic_sha256()
                || receipt.general_sem_config_sha256 != artifact.general_sem_config_sha256()
                || receipt.recipe_analytical_sha256 != artifact.recipe_analytical_sha256()
                || receipt.model_scientific_sha256 != model_scientific_sha256
                || receipt.source_dataset_fingerprint != dataset.fingerprint.0
                || receipt.resamples_requested != resamples
                || receipt.seed != seed.to_string()
                || receipt_seed != Some(seed)
                || receipt.confidence_level.to_bits() != confidence_level.to_bits()
                || receipt.interval != expected_interval
                || receipt.tail != expected_tail
                || usize::try_from(receipt.workers).ok() != Some(recipe.settings.workers)
                || recipe.settings.bootstrap_samples != resamples
                || recipe.settings.seed != seed
                || recipe.settings.confidence_level.to_bits() != confidence_level.to_bits()
                || recipe.settings.bootstrap_test_tail != qpls_core::PlsBootstrapTestTail::TwoSided
                || recipe.settings.studentized_inner_samples != 0
            {
                return Err(invalid_general_sem_authority(
                    "General SEM inference receipt differs from the resident compiled artifact, plan, config, recipe, model, dataset, or bootstrap settings",
                ));
            }
            Ok(())
        }
        (Some(_), None) => Err(invalid_general_sem_authority(
            "the resident case-bootstrap recipe requires a General SEM inference receipt",
        )),
        (None, _) => Err(invalid_general_sem_authority(
            "the resident Recipe-v4 does not contain GeneralSemConfigV1",
        )),
    }
}

fn validate_general_sem_pls_interaction_authority_v1(
    document: &CanonicalResultDocumentV2,
    results: &qpls_core::CanonicalGeneralSemResultsV1,
    artifact: &qpls_core::CompiledGeneralSemPlsRecipeV1,
    recipe: &AnalysisRecipeV4,
) -> Result<(), ProjectArchiveV6Error> {
    let compiled_interactions = artifact.plan().two_way_interactions();
    if compiled_interactions.is_empty() {
        if !results.interaction_effects.is_empty()
            || !results.joint_stage_structural_coefficients.is_empty()
        {
            return Err(invalid_general_sem_authority(
                "canonical joint-stage or interaction effects exist without compiled PLS interaction authority",
            ));
        }
        return Ok(());
    }

    if !results.specific_indirect_effects.is_empty()
        || !results.aggregate_effects.is_empty()
        || !results.higher_order_stages.is_empty()
        || !results.cbsem_fit.is_empty()
        || !results.identification_diagnostics.is_empty()
    {
        let message = if recipe.general_sem_config.as_ref().is_some_and(|config| {
            matches!(
                config.inference,
                GeneralSemInferenceV1::CaseBootstrap { .. }
            )
        }) {
            "the gamma-only General SEM PLS interaction-bootstrap cell must not persist mediation, higher-order, CB-SEM fit, or identification result sections"
        } else {
            "the point-only General SEM PLS interaction cell must not persist inference, mediation, higher-order, CB-SEM fit, or identification result sections"
        };
        return Err(invalid_general_sem_authority(message));
    }

    let expected_stage_one_digest = artifact
        .plan()
        .stage_one_projection_scientific_sha256()
        .ok_or_else(|| {
            invalid_general_sem_authority(
                "compiled PLS interactions require an exact stage-one projection digest",
            )
        })?;
    if results.interaction_effects.len() != compiled_interactions.len() {
        return Err(invalid_general_sem_authority(format!(
            "canonical interaction-effect count {} differs from compiled interaction count {}",
            results.interaction_effects.len(),
            compiled_interactions.len()
        )));
    }
    let persisted_by_interaction_id = results
        .interaction_effects
        .iter()
        .map(|effect| (effect.interaction_id.as_str(), effect))
        .collect::<BTreeMap<_, _>>();
    if persisted_by_interaction_id.len() != results.interaction_effects.len() {
        return Err(invalid_general_sem_authority(
            "canonical interaction effects do not have unique interaction identities",
        ));
    }

    for compiled in compiled_interactions {
        let persisted = persisted_by_interaction_id
            .get(compiled.interaction_id())
            .ok_or_else(|| {
                invalid_general_sem_authority(format!(
                    "compiled interaction {} has no canonical interaction effect",
                    compiled.interaction_id()
                ))
            })?;
        if persisted.effect_id != compiled.interaction_effect_relation_id()
            || persisted.focal_relation_id != compiled.focal_relation_id()
            || persisted.interaction_effect_relation_id != compiled.interaction_effect_relation_id()
            || persisted.interaction_effect_parameter_id
                != compiled.interaction_effect_parameter_id()
            || persisted.focal_predictor_id != compiled.focal_predictor_id()
            || persisted.moderator_id != compiled.moderator_id()
            || persisted.outcome_id != compiled.outcome_id()
            || persisted.generated_product_column_id != compiled.generated_product_column_id()
        {
            return Err(invalid_general_sem_authority(format!(
                "canonical interaction effect {} differs from its compiled interaction contract",
                compiled.interaction_id()
            )));
        }
        if persisted.stage_one_model_scientific_sha256 != expected_stage_one_digest {
            return Err(invalid_general_sem_authority(format!(
                "canonical interaction effect {} carries a stage-one projection digest that differs from the compiled plan",
                compiled.interaction_id()
            )));
        }
    }
    validate_general_sem_pls_moderation_document_v1(document, results, artifact, recipe)
}

const GENERAL_SEM_MODERATION_SECTION_ID_V1: &str = "general_sem_moderation";
const GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1: &str = "general_sem_interaction_effects";
const GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1: &str = "general_sem_conditional_slopes";
const GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1: &str = "general_sem_interaction_plots";
const GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1: &str = "general_sem_moderation_bootstrap";
const GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1: &str =
    "general_sem_moderation_gamma_inference";
const GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1: &str =
    "general_sem_moderation_bootstrap_receipt";
const GENERAL_SEM_MODERATION_EXCLUSION_IDS_V1: &[&str] = &[
    "moderation_point_estimation_only",
    "joint_stage_two_effects_and_fit_not_available",
];
const GENERAL_SEM_MODERATION_BOOTSTRAP_EXCLUSION_IDS_V1: &[&str] = &[
    "moderation_bootstrap_scientific_gamma_only",
    "moderation_beta_joint_coefficients_slopes_plots_point_only",
    "joint_stage_two_effects_and_fit_not_available",
];
const GENERAL_SEM_STRUCTURAL_PATH_COLUMNS_V1: &[&str] = &[
    "relation_id",
    "parameter_id",
    "source",
    "target",
    "coefficient",
];
const GENERAL_SEM_CONTROL_COLUMNS_V1: &[&str] = &[
    "relation_id",
    "parameter_id",
    "source",
    "target",
    "label",
    "coefficient",
];
const GENERAL_SEM_INTERACTION_EFFECT_COLUMNS_V1: &[&str] = &[
    "effect_id",
    "interaction_id",
    "focal_relation_id",
    "interaction_effect_relation_id",
    "interaction_effect_parameter_id",
    "focal_predictor_id",
    "moderator_id",
    "outcome_id",
    "generated_product_column_id",
    "stage_one_model_scientific_sha256",
    "observation_count",
    "standardized_product_coefficient",
    "scientific_rescaled_gamma",
    "product_mean",
    "product_sample_sd",
    "construction_method",
    "product_scale_version",
    "hierarchy_policy",
    "hierarchy_policy_version",
    "conditioning_policy_version",
    "method_version",
];
const GENERAL_SEM_CONDITIONAL_SLOPE_COLUMNS_V1: &[&str] = &[
    "effect_id",
    "interaction_id",
    "interaction_effect_id",
    "focal_relation_id",
    "probe_id",
    "probe_value_index",
    "moderator_id",
    "outcome_id",
    "moderator_value",
    "estimate",
    "conditioning_policy_version",
];
const GENERAL_SEM_INTERACTION_PLOT_COLUMNS_V1: &[&str] = &[
    "plot_id",
    "interaction_id",
    "interaction_effect_id",
    "focal_relation_id",
    "focal_predictor_id",
    "moderator_id",
    "outcome_id",
    "series_id",
    "probe_id",
    "probe_value_index",
    "moderator_value",
    "focal_value",
    "predicted_value",
    "lower",
    "upper",
];
const GENERAL_SEM_MODERATION_GAMMA_INFERENCE_COLUMNS_V1: &[&str] = &[
    "effect_id",
    "interaction_id",
    "focal_relation_id",
    "interaction_effect_relation_id",
    "interaction_effect_parameter_id",
    "generated_product_column_id",
    "focal_predictor_id",
    "moderator_id",
    "outcome_id",
    "stage_one_model_scientific_sha256",
    "product_scale_version",
    "point_method_version",
    "estimate",
    "bootstrap_mean",
    "bootstrap_bias",
    "standard_error",
    "lower",
    "upper",
    "p_value",
    "bootstrap_usable_replicates",
    "bootstrap_two_sided_exceedances",
];
const GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_COLUMNS_V1: &[&str] = &[
    "capability_id",
    "cell_id",
    "capability_version",
    "method_version",
    "point_method_version",
    "resampling_operation_version",
    "resampling_stream_version",
    "quantile_method_version",
    "standard_error_method_version",
    "summation_method_version",
    "p_value_method_version",
    "failure_policy_version",
    "sign_alignment_method_version",
    "product_scale_version",
    "gamma_target_version",
    "compilation_artifact_identity_sha256",
    "compiled_plan_sha256",
    "general_sem_config_sha256",
    "recipe_analytical_sha256",
    "model_scientific_sha256",
    "stage_one_model_scientific_sha256",
    "source_dataset_fingerprint",
    "complete_case_frame_sha256",
    "usable_replicate_indices_sha256",
    "gamma_target_identity_set_sha256",
    "interval",
    "tail",
    "confidence_level",
    "resamples_requested",
    "resamples_usable",
    "minimum_usable_resamples",
    "seed",
    "workers",
    "complete_model_reestimated_per_replicate",
    "shared_stage_one_reestimated_per_replicate",
    "score_vectors_sign_aligned_before_products",
    "product_scaling_recomputed_per_replicate",
    "joint_stage_two_reestimated_per_replicate",
    "complete_joint_point_contract_validated_per_replicate",
    "failed_replicate_count",
];

fn moderation_base_capability_cell_v1(
    recipe: &AnalysisRecipeV4,
) -> crate::CapabilityCellReferenceV2 {
    project_capability_cell_v2(
        &RecipeV4CompilerTarget::PlsPlanV2.capability_cell_for_method(recipe.settings.method),
    )
}

fn sort_project_capability_cells_v1(cells: &mut [crate::CapabilityCellReferenceV2]) {
    cells.sort_by(|left, right| {
        (
            left.capability_id.as_str(),
            left.cell_id.as_str(),
            left.capability_version.as_str(),
        )
            .cmp(&(
                right.capability_id.as_str(),
                right.cell_id.as_str(),
                right.capability_version.as_str(),
            ))
    });
}

fn moderation_table<'a>(
    document: &'a CanonicalResultDocumentV2,
    id: &str,
) -> Result<&'a CanonicalResultTableV2, ProjectArchiveV6Error> {
    document
        .tables
        .iter()
        .find(|table| table.id == id)
        .ok_or_else(|| {
            invalid_general_sem_authority(format!(
                "the exact moderation canonical result omits table {id}"
            ))
        })
}

fn validate_moderation_table_columns_v1(
    table: &CanonicalResultTableV2,
    expected: &[&str],
) -> Result<(), ProjectArchiveV6Error> {
    if table
        .columns
        .iter()
        .map(|column| column.id.as_str())
        .eq(expected.iter().copied())
        && table
            .rows
            .iter()
            .all(|row| row.cells.len() == expected.len())
    {
        Ok(())
    } else {
        Err(invalid_general_sem_authority(format!(
            "moderation table {} has drifted columns or row widths",
            table.id
        )))
    }
}

fn moderation_text_cell_v1<'a>(
    cell: &'a CanonicalResultCellV2,
    expected: &str,
    context: &str,
) -> Result<&'a str, ProjectArchiveV6Error> {
    match cell {
        CanonicalResultCellV2::Text { value } if value == expected => Ok(value),
        _ => Err(invalid_general_sem_authority(format!(
            "{context} differs from its typed moderation authority"
        ))),
    }
}

fn moderation_nonempty_text_cell_v1(
    cell: &CanonicalResultCellV2,
    context: &str,
) -> Result<(), ProjectArchiveV6Error> {
    match cell {
        CanonicalResultCellV2::Text { value } if !value.trim().is_empty() => Ok(()),
        _ => Err(invalid_general_sem_authority(format!(
            "{context} must be a nonempty text cell"
        ))),
    }
}

fn moderation_number_cell_v1(
    cell: &CanonicalResultCellV2,
    expected: f64,
    context: &str,
) -> Result<(), ProjectArchiveV6Error> {
    match cell {
        CanonicalResultCellV2::Number {
            value,
            display: None,
        } if value.to_bits() == expected.to_bits() => Ok(()),
        _ => Err(invalid_general_sem_authority(format!(
            "{context} differs bitwise from its typed moderation authority"
        ))),
    }
}

fn moderation_optional_number_cell_v1(
    cell: &CanonicalResultCellV2,
    expected: Option<f64>,
    context: &str,
) -> Result<(), ProjectArchiveV6Error> {
    match (cell, expected) {
        (
            CanonicalResultCellV2::Number {
                value,
                display: None,
            },
            Some(expected),
        ) if value.to_bits() == expected.to_bits() => Ok(()),
        (
            CanonicalResultCellV2::Missing {
                reason: CanonicalMissingReasonV2::NotEstimated,
                display: None,
            },
            None,
        ) => Ok(()),
        _ => Err(invalid_general_sem_authority(format!(
            "{context} differs from its typed moderation authority"
        ))),
    }
}

fn moderation_boolean_cell_v1(
    cell: &CanonicalResultCellV2,
    expected: bool,
    context: &str,
) -> Result<(), ProjectArchiveV6Error> {
    match cell {
        CanonicalResultCellV2::Boolean { value } if *value == expected => Ok(()),
        _ => Err(invalid_general_sem_authority(format!(
            "{context} differs from its typed moderation authority"
        ))),
    }
}

fn moderation_estimate_has_inference_v1(value: &qpls_core::CanonicalGeneralSemEstimateV1) -> bool {
    value.bootstrap_mean.is_some()
        || value.bootstrap_bias.is_some()
        || value.standard_error.is_some()
        || value.lower.is_some()
        || value.upper.is_some()
        || value.p_value.is_some()
        || value.bootstrap_usable_replicates.is_some()
        || value.bootstrap_two_sided_exceedances.is_some()
}

fn moderation_estimate_has_complete_inference_v1(
    value: &qpls_core::CanonicalGeneralSemEstimateV1,
) -> bool {
    value.bootstrap_mean.is_some()
        && value.bootstrap_bias.is_some()
        && value.standard_error.is_some()
        && value.lower.is_some()
        && value.upper.is_some()
        && value.p_value.is_some()
        && value.bootstrap_usable_replicates.is_some()
        && value.bootstrap_two_sided_exceedances.is_some()
}

fn validate_general_sem_moderation_inference_surface_v1(
    results: &qpls_core::CanonicalGeneralSemResultsV1,
    is_bootstrap: bool,
) -> Result<(), ProjectArchiveV6Error> {
    let every_gamma_is_complete = results.interaction_effects.iter().all(|effect| {
        moderation_estimate_has_complete_inference_v1(&effect.scientific_rescaled_gamma)
    });
    let any_gamma_inference = results
        .interaction_effects
        .iter()
        .any(|effect| moderation_estimate_has_inference_v1(&effect.scientific_rescaled_gamma));
    if (is_bootstrap && !every_gamma_is_complete) || (!is_bootstrap && any_gamma_inference) {
        return Err(invalid_general_sem_authority(
            "the exact moderation bootstrap inventory must infer every scientific gamma and point inference must infer none",
        ));
    }

    let point_only_inference = results
        .joint_stage_structural_coefficients
        .iter()
        .any(|coefficient| moderation_estimate_has_inference_v1(&coefficient.estimate))
        || results.interaction_effects.iter().any(|effect| {
            moderation_estimate_has_inference_v1(&effect.standardized_product_coefficient)
        })
        || results
            .conditional_effects
            .iter()
            .any(|effect| moderation_estimate_has_inference_v1(&effect.value))
        || results.interaction_plots.iter().any(|plot| {
            plot.series.iter().any(|series| {
                series
                    .points
                    .iter()
                    .any(|point| point.lower.is_some() || point.upper.is_some())
            })
        });
    if point_only_inference {
        return Err(invalid_general_sem_authority(
            "moderation beta, joint coefficients, conditional slopes, and plots must remain point-only",
        ));
    }
    Ok(())
}

fn validate_general_sem_pls_moderation_document_v1(
    document: &CanonicalResultDocumentV2,
    results: &qpls_core::CanonicalGeneralSemResultsV1,
    artifact: &qpls_core::CompiledGeneralSemPlsRecipeV1,
    recipe: &AnalysisRecipeV4,
) -> Result<(), ProjectArchiveV6Error> {
    let plan = artifact.plan();
    let moderation = project_capability_cell_v2(artifact.capability_cell());
    let base = moderation_base_capability_cell_v1(recipe);
    let bootstrap = project_capability_cell_v2(
        &qpls_core::pls_general_multiple_moderation_bootstrap_capability_cell_v1(),
    );
    let is_bootstrap = results.inference_receipt.is_some();
    if results.inference_receipt.as_ref().is_some_and(|receipt| {
        receipt.capability_cell
            != qpls_core::pls_general_multiple_moderation_bootstrap_capability_cell_v1()
    }) {
        return Err(invalid_general_sem_authority(
            "the exact moderation inference receipt must use the supplemental moderation-bootstrap cell",
        ));
    }
    validate_general_sem_moderation_inference_surface_v1(results, is_bootstrap)?;
    let mut expected_document_cells = vec![base.clone(), moderation.clone()];
    if is_bootstrap {
        expected_document_cells.push(bootstrap.clone());
    }
    sort_project_capability_cells_v1(&mut expected_document_cells);
    if document.capability_cells.as_deref() != Some(expected_document_cells.as_slice()) {
        return Err(invalid_general_sem_authority(if is_bootstrap {
            "the exact moderation document capability set differs from [base PLS, point moderation, supplemental moderation bootstrap]"
        } else {
            "the exact moderation document capability set differs from [base PLS, moderation]"
        }));
    }

    let expected_title = if is_bootstrap {
        "General SEM simultaneous two-way PLS moderation bootstrap inference"
    } else {
        "General SEM simultaneous two-way PLS moderation point estimates"
    };
    let expected_default_section = if is_bootstrap {
        GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1
    } else {
        GENERAL_SEM_MODERATION_SECTION_ID_V1
    };
    let expected_default_table = if is_bootstrap {
        GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1
    } else {
        GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1
    };
    if document.title != expected_title
        || document.presentation.default_section_id.as_deref() != Some(expected_default_section)
        || document.presentation.default_table_id.as_deref() != Some(expected_default_table)
    {
        return Err(invalid_general_sem_authority(
            "the exact moderation title or default presentation target has drifted",
        ));
    }

    let section_ids = document
        .sections
        .iter()
        .map(|section| section.id.as_str())
        .collect::<Vec<_>>();
    let mut expected_section_ids = vec![
        "run_details",
        "measurement_model",
        "structural_model",
        GENERAL_SEM_MODERATION_SECTION_ID_V1,
    ];
    if is_bootstrap {
        expected_section_ids.push(GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1);
    }
    if section_ids != expected_section_ids {
        return Err(invalid_general_sem_authority(
            "the exact moderation section inventory or order has drifted",
        ));
    }

    let has_algorithm_convergence = matches!(
        recipe.settings.weighting_scheme,
        qpls_core::WeightingScheme::Path | qpls_core::WeightingScheme::Factor
    );
    let has_fixed_scoring = plan
        .base_plan()
        .blocks()
        .iter()
        .any(|block| block.fixed_scoring().is_some());
    let has_score_execution = matches!(
        recipe.method_config.as_ref(),
        Some(MethodConfig::PlsAlgorithmConfiguredV2(_))
    ) || has_fixed_scoring;
    let has_controls = plan
        .base_plan()
        .paths()
        .iter()
        .any(|path| path.role() == qpls_core::StructuralRelationRoleV4::Control);

    let mut expected_run_details = vec!["estimation_summary"];
    if has_algorithm_convergence {
        expected_run_details.extend([
            crate::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1,
            crate::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1,
        ]);
    }
    if has_score_execution {
        expected_run_details.push(PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2);
    }
    if has_fixed_scoring {
        expected_run_details.push(crate::PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1);
    }
    let mut expected_measurement = vec!["outer_model"];
    if has_score_execution {
        expected_measurement.push(PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2);
    }
    let mut expected_structural = vec!["structural_paths"];
    if has_controls {
        expected_structural.push(crate::PLS_CONTROL_ESTIMATES_TABLE_ID_V2);
    }
    let expected_moderation_tables = [
        GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1,
        GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1,
        GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1,
    ];
    let expected_bootstrap_tables = [
        GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1,
        GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
    ];
    let expected_chart_ids = (0..results.interaction_plots.len())
        .map(|index| format!("general_sem_interaction_chart_{index:04}"))
        .collect::<Vec<_>>();
    if document.sections[0]
        .table_ids
        .iter()
        .map(String::as_str)
        .ne(expected_run_details.iter().copied())
        || document.sections[1]
            .table_ids
            .iter()
            .map(String::as_str)
            .ne(expected_measurement.iter().copied())
        || document.sections[2]
            .table_ids
            .iter()
            .map(String::as_str)
            .ne(expected_structural.iter().copied())
        || document.sections[3]
            .table_ids
            .iter()
            .map(String::as_str)
            .ne(expected_moderation_tables)
        || document.sections[..3]
            .iter()
            .any(|section| !section.chart_ids.is_empty())
        || document.sections[3].chart_ids != expected_chart_ids
        || (is_bootstrap
            && (document.sections[4]
                .table_ids
                .iter()
                .map(String::as_str)
                .ne(expected_bootstrap_tables)
                || !document.sections[4].chart_ids.is_empty()))
    {
        return Err(invalid_general_sem_authority(
            "the exact moderation section table/chart membership or order has drifted",
        ));
    }
    for section in &document.sections[..2] {
        if section.capability_cells.as_deref() != Some(std::slice::from_ref(&base)) {
            return Err(invalid_general_sem_authority(
                "stage-one moderation sections must be owned only by the base PLS cell",
            ));
        }
    }
    for section in &document.sections[2..4] {
        if section.capability_cells.as_deref() != Some(std::slice::from_ref(&moderation)) {
            return Err(invalid_general_sem_authority(
                "joint-stage moderation sections must be owned only by the moderation cell",
            ));
        }
    }
    if is_bootstrap
        && document.sections[4].capability_cells.as_deref()
            != Some(std::slice::from_ref(&bootstrap))
    {
        return Err(invalid_general_sem_authority(
            "moderation-bootstrap sections must be owned only by the supplemental bootstrap cell",
        ));
    }

    let mut expected_table_ids = vec!["estimation_summary", "outer_model", "structural_paths"];
    if has_score_execution {
        expected_table_ids.extend([
            PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2,
            PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2,
        ]);
    }
    if has_fixed_scoring {
        expected_table_ids.push(crate::PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1);
    }
    if has_algorithm_convergence {
        expected_table_ids.extend([
            crate::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1,
            crate::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1,
        ]);
    }
    if has_controls {
        expected_table_ids.push(crate::PLS_CONTROL_ESTIMATES_TABLE_ID_V2);
    }
    expected_table_ids.extend(expected_moderation_tables);
    if is_bootstrap {
        expected_table_ids.extend(expected_bootstrap_tables);
    }
    if document
        .tables
        .iter()
        .map(|table| table.id.as_str())
        .ne(expected_table_ids.iter().copied())
    {
        return Err(invalid_general_sem_authority(
            "the exact moderation canonical table inventory or order has drifted",
        ));
    }

    let stage_one_table_ids = expected_run_details
        .iter()
        .chain(expected_measurement.iter())
        .copied()
        .collect::<BTreeSet<_>>();
    for table in &document.tables {
        let expected_owner = if stage_one_table_ids.contains(table.id.as_str()) {
            &base
        } else if matches!(
            table.id.as_str(),
            GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1
                | GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1
        ) {
            &bootstrap
        } else {
            &moderation
        };
        if table.capability_cells.as_deref() != Some(std::slice::from_ref(expected_owner)) {
            return Err(invalid_general_sem_authority(format!(
                "moderation table {} has a drifted capability owner",
                table.id
            )));
        }
    }
    let table_reference_counts = document
        .sections
        .iter()
        .flat_map(|section| section.table_ids.iter())
        .fold(BTreeMap::<&str, usize>::new(), |mut counts, table_id| {
            *counts.entry(table_id.as_str()).or_default() += 1;
            counts
        });
    if expected_table_ids
        .iter()
        .any(|table_id| table_reference_counts.get(table_id).copied() != Some(1))
    {
        return Err(invalid_general_sem_authority(
            "each exact moderation table must belong to exactly one section",
        ));
    }

    let expected_exclusions = if is_bootstrap {
        GENERAL_SEM_MODERATION_BOOTSTRAP_EXCLUSION_IDS_V1
    } else {
        GENERAL_SEM_MODERATION_EXCLUSION_IDS_V1
    };
    if document
        .exclusions
        .iter()
        .map(|exclusion| exclusion.id.as_str())
        .ne(expected_exclusions.iter().copied())
        || document.exclusions.iter().any(|exclusion| {
            exclusion.capability_cell.as_ref()
                != Some(if is_bootstrap {
                    &bootstrap
                } else {
                    &moderation
                })
        })
    {
        return Err(invalid_general_sem_authority(
            "the exact moderation exclusion inventory or capability owner has drifted",
        ));
    }

    validate_general_sem_joint_stage_structural_ledger_v1(document, results, artifact)?;
    validate_general_sem_moderation_result_tables_v1(document, results, &moderation)?;
    validate_general_sem_moderation_charts_v1(document, results)?;
    if is_bootstrap {
        validate_general_sem_moderation_bootstrap_tables_v1(
            document, results, artifact, &bootstrap,
        )?;
    }
    Ok(())
}

fn validate_general_sem_moderation_bootstrap_tables_v1(
    document: &CanonicalResultDocumentV2,
    results: &qpls_core::CanonicalGeneralSemResultsV1,
    artifact: &qpls_core::CompiledGeneralSemPlsRecipeV1,
    bootstrap_cell: &crate::CapabilityCellReferenceV2,
) -> Result<(), ProjectArchiveV6Error> {
    let receipt = results.inference_receipt.as_ref().ok_or_else(|| {
        invalid_general_sem_authority(
            "moderation-bootstrap tables require the typed moderation inference receipt",
        )
    })?;
    let stage_one_digest = artifact
        .plan()
        .stage_one_projection_scientific_sha256()
        .ok_or_else(|| {
            invalid_general_sem_authority(
                "moderation bootstrap requires the compiled interaction-free stage-one digest",
            )
        })?;
    let mut expected_effect_ids = artifact
        .plan()
        .two_way_interactions()
        .iter()
        .map(|interaction| interaction.interaction_effect_relation_id().to_string())
        .collect::<Vec<_>>();
    expected_effect_ids.sort();
    let expected_identity_digest = qpls_core::general_sem_effect_identity_set_sha256_v1(
        &qpls_core::canonical_general_sem_effect_identities_v1(results),
    );
    if artifact.compiler_version()
        != qpls_core::GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1
        || receipt.capability_cell
            != qpls_core::pls_general_multiple_moderation_bootstrap_capability_cell_v1()
        || receipt.method_version
            != qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
        || receipt.resampling_operation_version
            != qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
        || receipt.effect_ids != expected_effect_ids
        || receipt.effect_identity_set_sha256 != expected_identity_digest
        || !receipt.complete_model_reestimated_per_replicate
    {
        return Err(invalid_general_sem_authority(
            "the moderation-bootstrap receipt differs from its recompiled cell, target inventory, identity digest, or full-model contract",
        ));
    }

    let gamma_table =
        moderation_table(document, GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1)?;
    validate_moderation_table_columns_v1(
        gamma_table,
        GENERAL_SEM_MODERATION_GAMMA_INFERENCE_COLUMNS_V1,
    )?;
    if gamma_table.capability_cells.as_deref() != Some(std::slice::from_ref(bootstrap_cell))
        || gamma_table.rows.len() != results.interaction_effects.len()
    {
        return Err(invalid_general_sem_authority(
            "the moderation gamma-inference table has drifted ownership or row inventory",
        ));
    }
    for (index, (row, effect)) in gamma_table
        .rows
        .iter()
        .zip(results.interaction_effects.iter())
        .enumerate()
    {
        if row.id != format!("moderation_gamma_inference_{index:04}") {
            return Err(invalid_general_sem_authority(
                "the moderation gamma-inference row identity has drifted",
            ));
        }
        for (cell, expected, context) in [
            (&row.cells[0], effect.effect_id.as_str(), "gamma effect id"),
            (
                &row.cells[1],
                effect.interaction_id.as_str(),
                "gamma interaction id",
            ),
            (
                &row.cells[2],
                effect.focal_relation_id.as_str(),
                "gamma focal relation",
            ),
            (
                &row.cells[3],
                effect.interaction_effect_relation_id.as_str(),
                "gamma interaction relation",
            ),
            (
                &row.cells[4],
                effect.interaction_effect_parameter_id.as_str(),
                "gamma interaction parameter",
            ),
            (
                &row.cells[5],
                effect.generated_product_column_id.as_str(),
                "gamma generated product",
            ),
            (
                &row.cells[6],
                effect.focal_predictor_id.as_str(),
                "gamma focal predictor",
            ),
            (
                &row.cells[7],
                effect.moderator_id.as_str(),
                "gamma moderator",
            ),
            (&row.cells[8], effect.outcome_id.as_str(), "gamma outcome"),
            (
                &row.cells[9],
                effect.stage_one_model_scientific_sha256.as_str(),
                "gamma stage-one digest",
            ),
            (
                &row.cells[10],
                effect.product_scale_version.as_str(),
                "gamma product scale",
            ),
            (
                &row.cells[11],
                effect.method_version.as_str(),
                "gamma point method",
            ),
        ] {
            moderation_text_cell_v1(cell, expected, context)?;
        }
        let estimate = &effect.scientific_rescaled_gamma;
        for (cell, expected, context) in [
            (&row.cells[12], Some(estimate.estimate), "gamma estimate"),
            (
                &row.cells[13],
                estimate.bootstrap_mean,
                "gamma bootstrap mean",
            ),
            (
                &row.cells[14],
                estimate.bootstrap_bias,
                "gamma bootstrap bias",
            ),
            (
                &row.cells[15],
                estimate.standard_error,
                "gamma standard error",
            ),
            (&row.cells[16], estimate.lower, "gamma lower interval"),
            (&row.cells[17], estimate.upper, "gamma upper interval"),
            (&row.cells[18], estimate.p_value, "gamma p value"),
            (
                &row.cells[19],
                estimate.bootstrap_usable_replicates.map(f64::from),
                "gamma usable replicates",
            ),
            (
                &row.cells[20],
                estimate.bootstrap_two_sided_exceedances.map(f64::from),
                "gamma two-sided exceedances",
            ),
        ] {
            let Some(expected) = expected else {
                return Err(invalid_general_sem_authority(format!(
                    "{context} is missing from the typed gamma inference"
                )));
            };
            moderation_number_cell_v1(cell, expected, context)?;
        }
    }

    let receipt_table = moderation_table(
        document,
        GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
    )?;
    validate_moderation_table_columns_v1(
        receipt_table,
        GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_COLUMNS_V1,
    )?;
    if receipt_table.capability_cells.as_deref() != Some(std::slice::from_ref(bootstrap_cell))
        || receipt_table.rows.len() != 1
        || receipt_table.rows[0].id != "moderation_bootstrap_receipt"
    {
        return Err(invalid_general_sem_authority(
            "the moderation-bootstrap receipt table has drifted ownership or row inventory",
        ));
    }
    let row = &receipt_table.rows[0];
    for (cell, expected, context) in [
        (
            &row.cells[0],
            receipt.capability_cell.capability_id.as_str(),
            "bootstrap capability id",
        ),
        (
            &row.cells[1],
            receipt.capability_cell.cell_id.as_str(),
            "bootstrap cell id",
        ),
        (
            &row.cells[2],
            receipt.capability_cell.capability_version.as_str(),
            "bootstrap capability version",
        ),
        (
            &row.cells[3],
            receipt.method_version.as_str(),
            "bootstrap method version",
        ),
        (
            &row.cells[4],
            qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
            "bootstrap point method version",
        ),
        (
            &row.cells[5],
            receipt.resampling_operation_version.as_str(),
            "bootstrap operation version",
        ),
        (
            &row.cells[6],
            receipt.resampling_stream_version.as_str(),
            "bootstrap stream version",
        ),
        (
            &row.cells[7],
            receipt.quantile_method_version.as_str(),
            "bootstrap quantile version",
        ),
        (
            &row.cells[8],
            receipt.standard_error_method_version.as_str(),
            "bootstrap standard-error version",
        ),
        (
            &row.cells[9],
            receipt.summation_method_version.as_str(),
            "bootstrap summation version",
        ),
        (
            &row.cells[10],
            receipt.p_value_method_version.as_str(),
            "bootstrap p-value version",
        ),
        (
            &row.cells[11],
            receipt.failure_policy_version.as_str(),
            "bootstrap failure-policy version",
        ),
        (
            &row.cells[12],
            qpls_resampling::GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1,
            "bootstrap sign-alignment version",
        ),
        (
            &row.cells[13],
            qpls_core::GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
            "bootstrap product-scale version",
        ),
        (
            &row.cells[14],
            qpls_resampling::GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1,
            "bootstrap gamma-target version",
        ),
        (
            &row.cells[15],
            receipt.compilation_artifact_identity_sha256.as_str(),
            "bootstrap artifact digest",
        ),
        (
            &row.cells[16],
            receipt.compiled_plan_sha256.as_str(),
            "bootstrap plan digest",
        ),
        (
            &row.cells[17],
            receipt.general_sem_config_sha256.as_str(),
            "bootstrap config digest",
        ),
        (
            &row.cells[18],
            receipt.recipe_analytical_sha256.as_str(),
            "bootstrap recipe digest",
        ),
        (
            &row.cells[19],
            receipt.model_scientific_sha256.as_str(),
            "bootstrap model digest",
        ),
        (
            &row.cells[20],
            stage_one_digest,
            "bootstrap stage-one digest",
        ),
        (
            &row.cells[21],
            receipt.source_dataset_fingerprint.as_str(),
            "bootstrap dataset fingerprint",
        ),
        (
            &row.cells[22],
            receipt.complete_case_frame_sha256.as_str(),
            "bootstrap complete-case digest",
        ),
        (
            &row.cells[23],
            receipt.usable_replicate_indices_sha256.as_str(),
            "bootstrap usable-index digest",
        ),
        (
            &row.cells[24],
            receipt.effect_identity_set_sha256.as_str(),
            "bootstrap gamma-identity digest",
        ),
        (&row.cells[25], "percentile_type7", "bootstrap interval"),
        (&row.cells[26], "two_sided", "bootstrap tail"),
        (&row.cells[31], receipt.seed.as_str(), "bootstrap seed"),
    ] {
        moderation_text_cell_v1(cell, expected, context)?;
    }
    for (cell, expected, context) in [
        (
            &row.cells[27],
            receipt.confidence_level,
            "bootstrap confidence level",
        ),
        (
            &row.cells[28],
            f64::from(receipt.resamples_requested),
            "bootstrap requested replicates",
        ),
        (
            &row.cells[29],
            f64::from(receipt.resamples_usable),
            "bootstrap usable replicates",
        ),
        (
            &row.cells[30],
            f64::from(receipt.minimum_usable_resamples),
            "bootstrap minimum usable replicates",
        ),
        (
            &row.cells[32],
            f64::from(receipt.workers),
            "bootstrap workers",
        ),
        (
            &row.cells[39],
            receipt.failed_replicates.len() as f64,
            "bootstrap failed-replicate count",
        ),
    ] {
        moderation_number_cell_v1(cell, expected, context)?;
    }
    for (index, context) in [
        "complete model reestimated",
        "shared stage one reestimated",
        "score vectors sign aligned",
        "product scaling recomputed",
        "joint stage two reestimated",
        "complete joint point contract validated",
    ]
    .into_iter()
    .enumerate()
    {
        moderation_boolean_cell_v1(&row.cells[33 + index], true, context)?;
    }
    Ok(())
}

fn validate_general_sem_joint_stage_structural_ledger_v1(
    document: &CanonicalResultDocumentV2,
    results: &qpls_core::CanonicalGeneralSemResultsV1,
    artifact: &qpls_core::CompiledGeneralSemPlsRecipeV1,
) -> Result<(), ProjectArchiveV6Error> {
    let interaction_relation_ids = artifact
        .plan()
        .two_way_interactions()
        .iter()
        .map(|interaction| interaction.interaction_effect_relation_id())
        .collect::<BTreeSet<_>>();
    let plan_paths = artifact
        .plan()
        .topology()
        .structural_relations()
        .iter()
        .filter(|relation| !interaction_relation_ids.contains(relation.relation_id()))
        .collect::<Vec<_>>();
    let ledger = &results.joint_stage_structural_coefficients;
    if ledger.len() != plan_paths.len() {
        return Err(invalid_general_sem_authority(format!(
            "joint-stage structural ledger count {} differs from compiled path count {}",
            ledger.len(),
            plan_paths.len()
        )));
    }
    for (compiled, coefficient) in plan_paths.iter().zip(ledger) {
        let expected_role = match compiled.role() {
            qpls_core::StructuralRelationRoleV4::Structural => {
                qpls_core::CanonicalStructuralRelationRoleV1::Structural
            }
            qpls_core::StructuralRelationRoleV4::Control => {
                qpls_core::CanonicalStructuralRelationRoleV1::Control
            }
        };
        if coefficient.relation_id != compiled.relation_id()
            || coefficient.parameter_id != compiled.parameter_id()
            || coefficient.trace.model_id != artifact.plan().model_id()
            || coefficient.trace.capability_cell != *artifact.capability_cell()
            || coefficient.source_id != compiled.source()
            || coefficient.target_id != compiled.target()
            || coefficient.role != expected_role
            || coefficient.stage != qpls_core::CanonicalStructuralEstimateStageV1::JointStageTwo
            || coefficient.method_version
                != GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
        {
            return Err(invalid_general_sem_authority(format!(
                "joint-stage structural coefficient {} differs from its compiled relation/parameter contract",
                compiled.relation_id()
            )));
        }
    }

    let structural_table = moderation_table(document, "structural_paths")?;
    validate_moderation_table_columns_v1(structural_table, GENERAL_SEM_STRUCTURAL_PATH_COLUMNS_V1)?;
    let structural_coefficients = ledger
        .iter()
        .filter(|coefficient| {
            coefficient.role == qpls_core::CanonicalStructuralRelationRoleV1::Structural
        })
        .collect::<Vec<_>>();
    if structural_table.rows.len() != structural_coefficients.len() {
        return Err(invalid_general_sem_authority(
            "structural_paths row count differs from the typed joint-stage structural ledger",
        ));
    }
    for (index, (row, coefficient)) in structural_table
        .rows
        .iter()
        .zip(structural_coefficients)
        .enumerate()
    {
        if row.id != format!("joint_path_{index:04}") {
            return Err(invalid_general_sem_authority(
                "structural_paths row identities are not canonical joint-stage order",
            ));
        }
        moderation_text_cell_v1(&row.cells[0], &coefficient.relation_id, "relation_id")?;
        moderation_text_cell_v1(&row.cells[1], &coefficient.parameter_id, "parameter_id")?;
        moderation_text_cell_v1(&row.cells[2], &coefficient.source_id, "source")?;
        moderation_text_cell_v1(&row.cells[3], &coefficient.target_id, "target")?;
        moderation_number_cell_v1(
            &row.cells[4],
            coefficient.estimate.estimate,
            "structural coefficient",
        )?;
    }

    let control_coefficients = ledger
        .iter()
        .filter(|coefficient| {
            coefficient.role == qpls_core::CanonicalStructuralRelationRoleV1::Control
        })
        .collect::<Vec<_>>();
    if control_coefficients.is_empty() {
        if document
            .tables
            .iter()
            .any(|table| table.id == crate::PLS_CONTROL_ESTIMATES_TABLE_ID_V2)
        {
            return Err(invalid_general_sem_authority(
                "control table exists without typed joint-stage control coefficients",
            ));
        }
    } else {
        let control_table = moderation_table(document, crate::PLS_CONTROL_ESTIMATES_TABLE_ID_V2)?;
        validate_moderation_table_columns_v1(control_table, GENERAL_SEM_CONTROL_COLUMNS_V1)?;
        if control_table.rows.len() != control_coefficients.len() {
            return Err(invalid_general_sem_authority(
                "control table row count differs from the typed joint-stage control ledger",
            ));
        }
        for (index, (row, coefficient)) in control_table
            .rows
            .iter()
            .zip(control_coefficients)
            .enumerate()
        {
            if row.id != format!("joint_control_{index:04}") {
                return Err(invalid_general_sem_authority(
                    "control table row identities are not canonical joint-stage order",
                ));
            }
            moderation_text_cell_v1(&row.cells[0], &coefficient.relation_id, "control relation")?;
            moderation_text_cell_v1(
                &row.cells[1],
                &coefficient.parameter_id,
                "control parameter",
            )?;
            moderation_text_cell_v1(&row.cells[2], &coefficient.source_id, "control source")?;
            moderation_text_cell_v1(&row.cells[3], &coefficient.target_id, "control target")?;
            moderation_nonempty_text_cell_v1(&row.cells[4], "control label")?;
            moderation_number_cell_v1(
                &row.cells[5],
                coefficient.estimate.estimate,
                "control coefficient",
            )?;
        }
    }

    for interaction in &results.interaction_effects {
        let focal_coefficient = ledger
            .iter()
            .find(|coefficient| coefficient.relation_id == interaction.focal_relation_id)
            .ok_or_else(|| {
                invalid_general_sem_authority(format!(
                    "interaction {} has no joint-stage focal relation coefficient",
                    interaction.interaction_id
                ))
            })?;
        let zero_probe = results
            .conditional_effects
            .iter()
            .find(|effect| {
                effect.interaction_effect_id.as_deref() == Some(interaction.effect_id.as_str())
                    && effect.probe_value_index == 1
                    && effect.moderator_value.to_bits() == 0.0_f64.to_bits()
            })
            .ok_or_else(|| {
                invalid_general_sem_authority(format!(
                    "interaction {} omits its frozen zero-moderator slope",
                    interaction.interaction_id
                ))
            })?;
        if zero_probe.value.estimate.to_bits() != focal_coefficient.estimate.estimate.to_bits() {
            return Err(invalid_general_sem_authority(format!(
                "interaction {} zero-probe slope differs from its joint-stage focal coefficient",
                interaction.interaction_id
            )));
        }
    }
    Ok(())
}

fn validate_general_sem_moderation_result_tables_v1(
    document: &CanonicalResultDocumentV2,
    results: &qpls_core::CanonicalGeneralSemResultsV1,
    moderation: &crate::CapabilityCellReferenceV2,
) -> Result<(), ProjectArchiveV6Error> {
    let interactions = results
        .interaction_effects
        .iter()
        .map(|effect| (effect.interaction_id.as_str(), effect))
        .collect::<BTreeMap<_, _>>();
    if interactions.len() != results.interaction_effects.len()
        || results.conditional_effect_probes.len() != interactions.len()
        || results.conditional_effects.len() != interactions.len() * 3
        || results.interaction_plots.len() != interactions.len()
    {
        return Err(invalid_general_sem_authority(
            "typed moderation effects, probes, conditional slopes, and plots are not one-to-one",
        ));
    }

    let interaction_table =
        moderation_table(document, GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1)?;
    validate_moderation_table_columns_v1(
        interaction_table,
        GENERAL_SEM_INTERACTION_EFFECT_COLUMNS_V1,
    )?;
    if interaction_table.rows.len() != results.interaction_effects.len() {
        return Err(invalid_general_sem_authority(
            "interaction-effect table row count differs from the typed interaction ledger",
        ));
    }
    for (index, (row, effect)) in interaction_table
        .rows
        .iter()
        .zip(&results.interaction_effects)
        .enumerate()
    {
        if row.id != format!("interaction_effect_{index:04}") {
            return Err(invalid_general_sem_authority(
                "interaction-effect rows are not in canonical typed order",
            ));
        }
        for (cell, expected, context) in [
            (
                &row.cells[0],
                effect.effect_id.as_str(),
                "interaction effect ID",
            ),
            (
                &row.cells[1],
                effect.interaction_id.as_str(),
                "interaction ID",
            ),
            (
                &row.cells[2],
                effect.focal_relation_id.as_str(),
                "focal relation",
            ),
            (
                &row.cells[3],
                effect.interaction_effect_relation_id.as_str(),
                "interaction relation",
            ),
            (
                &row.cells[4],
                effect.interaction_effect_parameter_id.as_str(),
                "interaction parameter",
            ),
            (
                &row.cells[5],
                effect.focal_predictor_id.as_str(),
                "focal predictor",
            ),
            (&row.cells[6], effect.moderator_id.as_str(), "moderator"),
            (&row.cells[7], effect.outcome_id.as_str(), "outcome"),
            (
                &row.cells[8],
                effect.generated_product_column_id.as_str(),
                "generated product column",
            ),
            (
                &row.cells[9],
                effect.stage_one_model_scientific_sha256.as_str(),
                "stage-one projection digest",
            ),
        ] {
            moderation_text_cell_v1(cell, expected, context)?;
        }
        moderation_number_cell_v1(
            &row.cells[10],
            f64::from(effect.observation_count),
            "interaction observation count",
        )?;
        moderation_number_cell_v1(
            &row.cells[11],
            effect.standardized_product_coefficient.estimate,
            "standardized product coefficient",
        )?;
        moderation_number_cell_v1(
            &row.cells[12],
            effect.scientific_rescaled_gamma.estimate,
            "scientific rescaled gamma",
        )?;
        moderation_number_cell_v1(
            &row.cells[13],
            effect.unstandardized_product_mean,
            "product mean",
        )?;
        moderation_number_cell_v1(
            &row.cells[14],
            effect.unstandardized_product_sample_standard_deviation,
            "product sample standard deviation",
        )?;
        moderation_text_cell_v1(&row.cells[15], "two_stage", "construction method")?;
        moderation_text_cell_v1(
            &row.cells[16],
            &effect.product_scale_version,
            "product-scale version",
        )?;
        moderation_text_cell_v1(&row.cells[17], "strong", "hierarchy policy")?;
        moderation_text_cell_v1(
            &row.cells[18],
            &effect.hierarchy_policy_version,
            "hierarchy-policy version",
        )?;
        moderation_text_cell_v1(
            &row.cells[19],
            &effect.conditioning_policy_version,
            "conditioning-policy version",
        )?;
        moderation_text_cell_v1(&row.cells[20], &effect.method_version, "method version")?;
    }

    let conditional_table = moderation_table(document, GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1)?;
    validate_moderation_table_columns_v1(
        conditional_table,
        GENERAL_SEM_CONDITIONAL_SLOPE_COLUMNS_V1,
    )?;
    if conditional_table.rows.len() != results.conditional_effects.len() {
        return Err(invalid_general_sem_authority(
            "conditional-slope table row count differs from typed conditional effects",
        ));
    }
    for (index, (row, effect)) in conditional_table
        .rows
        .iter()
        .zip(&results.conditional_effects)
        .enumerate()
    {
        let interaction = interactions
            .get(effect.interaction_id.as_str())
            .ok_or_else(|| {
                invalid_general_sem_authority("conditional effect has no interaction")
            })?;
        let interaction_effect_id = effect.interaction_effect_id.as_deref().ok_or_else(|| {
            invalid_general_sem_authority("conditional effect omits interaction-effect identity")
        })?;
        if row.id != format!("conditional_slope_{index:04}") {
            return Err(invalid_general_sem_authority(
                "conditional-slope rows are not in canonical typed order",
            ));
        }
        for (cell, expected, context) in [
            (
                &row.cells[0],
                effect.effect_id.as_str(),
                "conditional effect ID",
            ),
            (
                &row.cells[1],
                effect.interaction_id.as_str(),
                "conditional interaction",
            ),
            (
                &row.cells[2],
                interaction_effect_id,
                "conditional interaction effect",
            ),
            (
                &row.cells[3],
                effect.focal_relation_id.as_str(),
                "conditional focal relation",
            ),
            (&row.cells[4], effect.probe_id.as_str(), "conditional probe"),
        ] {
            moderation_text_cell_v1(cell, expected, context)?;
        }
        moderation_number_cell_v1(
            &row.cells[5],
            f64::from(effect.probe_value_index),
            "conditional probe index",
        )?;
        moderation_text_cell_v1(&row.cells[6], &effect.moderator_id, "conditional moderator")?;
        moderation_text_cell_v1(
            &row.cells[7],
            &interaction.outcome_id,
            "conditional outcome",
        )?;
        moderation_number_cell_v1(
            &row.cells[8],
            effect.moderator_value,
            "conditional moderator value",
        )?;
        moderation_number_cell_v1(
            &row.cells[9],
            effect.value.estimate,
            "conditional slope estimate",
        )?;
        moderation_text_cell_v1(
            &row.cells[10],
            &interaction.conditioning_policy_version,
            "conditional policy version",
        )?;
    }

    let plot_table = moderation_table(document, GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1)?;
    validate_moderation_table_columns_v1(plot_table, GENERAL_SEM_INTERACTION_PLOT_COLUMNS_V1)?;
    let plot_points = results
        .interaction_plots
        .iter()
        .flat_map(|plot| {
            plot.series.iter().flat_map(move |series| {
                series.points.iter().map(move |point| (plot, series, point))
            })
        })
        .collect::<Vec<_>>();
    if plot_table.rows.len() != plot_points.len() {
        return Err(invalid_general_sem_authority(
            "interaction plot table row count differs from typed plot points",
        ));
    }
    for (index, (row, (plot, series, point))) in plot_table.rows.iter().zip(plot_points).enumerate()
    {
        let interaction_effect_id = plot.interaction_effect_id.as_deref().ok_or_else(|| {
            invalid_general_sem_authority("interaction plot omits interaction-effect identity")
        })?;
        if row.id != format!("interaction_plot_point_{index:04}") {
            return Err(invalid_general_sem_authority(
                "interaction plot rows are not in canonical typed order",
            ));
        }
        for (cell, expected, context) in [
            (&row.cells[0], plot.plot_id.as_str(), "plot ID"),
            (
                &row.cells[1],
                plot.interaction_id.as_str(),
                "plot interaction",
            ),
            (
                &row.cells[2],
                interaction_effect_id,
                "plot interaction effect",
            ),
            (
                &row.cells[3],
                plot.focal_relation_id.as_str(),
                "plot focal relation",
            ),
            (
                &row.cells[4],
                plot.focal_predictor_id.as_str(),
                "plot focal predictor",
            ),
            (&row.cells[5], plot.moderator_id.as_str(), "plot moderator"),
            (&row.cells[6], plot.outcome_id.as_str(), "plot outcome"),
            (&row.cells[7], series.series_id.as_str(), "plot series"),
            (&row.cells[8], series.probe_id.as_str(), "plot probe"),
        ] {
            moderation_text_cell_v1(cell, expected, context)?;
        }
        moderation_number_cell_v1(
            &row.cells[9],
            f64::from(series.probe_value_index),
            "plot probe index",
        )?;
        moderation_number_cell_v1(
            &row.cells[10],
            series.moderator_value,
            "plot moderator value",
        )?;
        moderation_number_cell_v1(&row.cells[11], point.focal_value, "plot focal value")?;
        moderation_number_cell_v1(
            &row.cells[12],
            point.predicted_value,
            "plot predicted value",
        )?;
        moderation_optional_number_cell_v1(&row.cells[13], point.lower, "plot lower bound")?;
        moderation_optional_number_cell_v1(&row.cells[14], point.upper, "plot upper bound")?;
    }

    for table in [interaction_table, conditional_table, plot_table] {
        if table.capability_cells.as_deref() != Some(std::slice::from_ref(moderation)) {
            return Err(invalid_general_sem_authority(
                "typed moderation tables must be owned only by the exact moderation cell",
            ));
        }
    }
    Ok(())
}

fn validate_general_sem_moderation_charts_v1(
    document: &CanonicalResultDocumentV2,
    results: &qpls_core::CanonicalGeneralSemResultsV1,
) -> Result<(), ProjectArchiveV6Error> {
    if document.charts.len() != results.interaction_plots.len() {
        return Err(invalid_general_sem_authority(
            "moderation chart count differs from typed interaction plot count",
        ));
    }
    for (plot_index, (chart, plot)) in document
        .charts
        .iter()
        .zip(&results.interaction_plots)
        .enumerate()
    {
        let expected_x_axis_label = format!("{} (standardized)", plot.focal_predictor_id);
        let expected_y_axis_label = format!("{} (predicted standardized)", plot.outcome_id);
        if chart.id != format!("general_sem_interaction_chart_{plot_index:04}")
            || chart.title != format!("Interaction {}", plot.interaction_id)
            || chart.kind != crate::CanonicalChartKindV2::Line
            || chart.source_table_id.as_deref() != Some(GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1)
            || chart.series.len() != plot.series.len()
            || chart.display.palette.is_some()
            || chart.display.show_legend != Some(true)
            || chart.display.show_values != Some(false)
            || chart.display.x_axis_label.as_deref() != Some(expected_x_axis_label.as_str())
            || chart.display.y_axis_label.as_deref() != Some(expected_y_axis_label.as_str())
        {
            return Err(invalid_general_sem_authority(format!(
                "chart {} differs from its typed interaction plot inventory",
                chart.id
            )));
        }
        for (series, typed_series) in chart.series.iter().zip(&plot.series) {
            if series.id != typed_series.series_id
                || series.label
                    != format!(
                        "{} = {:.4}",
                        plot.moderator_id, typed_series.moderator_value
                    )
                || series.group.as_deref() != Some(plot.interaction_id.as_str())
                || series.points.len() != typed_series.points.len()
            {
                return Err(invalid_general_sem_authority(format!(
                    "chart {} series {} differs from its typed plot series",
                    chart.id, series.id
                )));
            }
            for (point, typed_point) in series.points.iter().zip(&typed_series.points) {
                let crate::CanonicalChartXValueV2::Number(x) = &point.x else {
                    return Err(invalid_general_sem_authority(format!(
                        "chart {} contains a nonnumeric focal coordinate",
                        chart.id
                    )));
                };
                if x.to_bits() != typed_point.focal_value.to_bits()
                    || point.y.to_bits() != typed_point.predicted_value.to_bits()
                    || !same_optional_f64_bits_v1(point.lower, typed_point.lower)
                    || !same_optional_f64_bits_v1(point.upper, typed_point.upper)
                    || point.label.is_some()
                {
                    return Err(invalid_general_sem_authority(format!(
                        "chart {} point differs bitwise from its typed interaction plot",
                        chart.id
                    )));
                }
            }
        }
    }
    Ok(())
}

fn same_optional_f64_bits_v1(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        (None, None) => true,
        _ => false,
    }
}

fn is_exact_recipe_v4_pls_result(document: &CanonicalResultDocumentV2) -> bool {
    is_exact_recipe_v4_pls_base_result(document)
        || is_exact_recipe_v4_pls_nonlinear_result(document)
}

fn is_exact_recipe_v4_pls_base_result(document: &CanonicalResultDocumentV2) -> bool {
    document.provenance.capability_cell == recipe_v4_pls_base_capability_cell_v1()
}

fn is_exact_recipe_v4_pls_nonlinear_result(document: &CanonicalResultDocumentV2) -> bool {
    document.provenance.capability_cell == recipe_v4_pls_nonlinear_capability_cell_v1()
}

const RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7: &str =
    "compiled_recipe_v4_pls_plan_v2_execution_v7";
const PLS_NONLINEAR_SECTION_ID_V1: &str = "nonlinear_relationships";
const PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1: &str = "nonlinear_quadratic_diagnostics";
const PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1: &str = "nonlinear_equation_fit";
const PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1: &str = "nonlinear_method_scope";
const PLS_NONLINEAR_TERM_V1: &str = "centered_squared_construct_score_v1";
const PLS_NONLINEAR_ENGINE_WARNING_V1: &str = "Nonlinear effects are validated for the documented QuickPLS v1.2.3 fixed-score quadratic diagnostic scope; diagnostics use fixed PLS construct scores and centered squared score terms.";
const PLS_NONLINEAR_DIAGNOSTIC_COLUMNS_V1: &[&str] = &[
    "source",
    "target",
    "linear_coefficient",
    "quadratic_coefficient",
    "standard_error",
    "t_statistic",
    "p_value_two_sided",
    "warning",
];
const PLS_NONLINEAR_EQUATION_FIT_COLUMNS_V1: &[&str] = &[
    "target",
    "linear_r_squared",
    "augmented_r_squared",
    "delta_r_squared",
];
const PLS_NONLINEAR_METHOD_SCOPE_COLUMNS_V1: &[&str] = &["method_version", "term", "warning"];

fn recipe_v4_pls_base_capability_cell_v1() -> crate::CapabilityCellReferenceV2 {
    crate::CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_ALGORITHM_CAPABILITY_ID.into(),
        cell_id: PLS_ALGORITHM_CELL_ID.into(),
        capability_version: PLS_ALGORITHM_CAPABILITY_VERSION.into(),
    }
}

fn recipe_v4_pls_nonlinear_capability_cell_v1() -> crate::CapabilityCellReferenceV2 {
    crate::CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_NONLINEAR_EFFECTS_CAPABILITY_ID.into(),
        cell_id: PLS_NONLINEAR_EFFECTS_CELL_ID.into(),
        capability_version: PLS_NONLINEAR_EFFECTS_CAPABILITY_VERSION.into(),
    }
}

fn has_recipe_v4_pls_nonlinear_artifacts_v1(document: &CanonicalResultDocumentV2) -> bool {
    let nonlinear = recipe_v4_pls_nonlinear_capability_cell_v1();
    document.provenance.method_version == NONLINEAR_EFFECTS_METHOD_VERSION
        || document.provenance.engine_version
            == RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        || document.sections.iter().any(|section| {
            section.id == PLS_NONLINEAR_SECTION_ID_V1
                || section
                    .capability_cells
                    .as_ref()
                    .is_some_and(|cells| cells.contains(&nonlinear))
        })
        || document.tables.iter().any(|table| {
            matches!(
                table.id.as_str(),
                PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1
                    | PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1
                    | PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1
            ) || table
                .capability_cells
                .as_ref()
                .is_some_and(|cells| cells.contains(&nonlinear))
        })
        || document
            .capability_cells
            .as_ref()
            .is_some_and(|cells| cells.contains(&nonlinear))
}

fn invalid_pls_nonlinear(message: impl Into<String>) -> ProjectArchiveV6Error {
    ProjectArchiveV6Error::CanonicalPlsNonlinear(message.into())
}

fn pls_nonlinear_table<'a>(
    document: &'a CanonicalResultDocumentV2,
    id: &str,
) -> Result<&'a CanonicalResultTableV2, ProjectArchiveV6Error> {
    let matches = document
        .tables
        .iter()
        .filter(|table| table.id == id)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [table] => Ok(*table),
        _ => Err(invalid_pls_nonlinear(format!(
            "canonical result must contain exactly one {id} table"
        ))),
    }
}

fn validate_pls_nonlinear_columns_v1(
    table: &CanonicalResultTableV2,
    expected: &[&str],
) -> Result<(), ProjectArchiveV6Error> {
    if table
        .columns
        .iter()
        .map(|column| column.id.as_str())
        .eq(expected.iter().copied())
        && table
            .rows
            .iter()
            .all(|row| row.cells.len() == expected.len())
    {
        Ok(())
    } else {
        Err(invalid_pls_nonlinear(format!(
            "canonical table {} has drifted columns or row widths",
            table.id
        )))
    }
}

fn pls_nonlinear_text_v1(cell: &CanonicalResultCellV2) -> Result<&str, ProjectArchiveV6Error> {
    match cell {
        CanonicalResultCellV2::Text { value } if !value.trim().is_empty() => Ok(value),
        _ => Err(invalid_pls_nonlinear(
            "canonical nonlinear text cell is empty or not text",
        )),
    }
}

fn pls_nonlinear_number_v1(cell: &CanonicalResultCellV2) -> Result<f64, ProjectArchiveV6Error> {
    match cell {
        CanonicalResultCellV2::Number { value, .. } if value.is_finite() => Ok(*value),
        _ => Err(invalid_pls_nonlinear(
            "canonical nonlinear number cell is non-finite or not numeric",
        )),
    }
}

fn validate_pls_nonlinear_optional_warning_v1(
    cell: &CanonicalResultCellV2,
) -> Result<(), ProjectArchiveV6Error> {
    match cell {
        CanonicalResultCellV2::Text { value } if !value.trim().is_empty() => Ok(()),
        CanonicalResultCellV2::Missing {
            reason: CanonicalMissingReasonV2::NotEstimated,
            display: None,
        } => Ok(()),
        _ => Err(invalid_pls_nonlinear(
            "nonlinear diagnostic warning must be text or not_estimated",
        )),
    }
}

fn validate_recipe_v4_pls_nonlinear_base_projection_v1(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
) -> Result<(), ProjectArchiveV6Error> {
    let mut base_recipe = recipe.clone();
    base_recipe.settings.method = AnalysisMethod::PlsPm;
    base_recipe.method_config = Some(MethodConfig::PlsAlgorithm);
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let base_cell = recipe_v4_pls_base_capability_cell_v1();
    let base_artifact =
        compile_analysis_recipe_v4(&base_recipe, Some(model), target, target.capability_cell())
            .map_err(|error| {
                invalid_pls_nonlinear(format!("base PLS projection recompilation failed: {error}"))
            })?;
    let mut base_document = document.clone();
    base_document
        .sections
        .retain(|section| section.id != PLS_NONLINEAR_SECTION_ID_V1);
    base_document.tables.retain(|table| {
        !matches!(
            table.id.as_str(),
            PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1
                | PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1
                | PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1
        )
    });
    base_document.title = "PLS-SEM results".into();
    base_document.provenance.capability_cell = base_cell.clone();
    base_document.provenance.method_version = PLS_METHOD_VERSION.into();
    base_document.provenance.engine_version =
        crate::LEGACY_RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION_V1.into();
    base_document.provenance.recipe_digest =
        base_artifact.receipt().recipe_analytical_sha256().into();
    base_document.capability_cells = Some(vec![base_cell]);
    base_document.presentation.default_section_id = Some("structural_model".into());
    base_document.presentation.default_table_id = Some("structural_paths".into());
    validate_recipe_v4_pls_score_execution_document_v2(&base_document, &base_recipe, model)
        .map_err(ProjectArchiveV6Error::CanonicalPlsScoreExecution)
}

fn validate_recipe_v4_pls_nonlinear_document_v1(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    dataset: &DatasetDescriptor,
) -> Result<(), ProjectArchiveV6Error> {
    if recipe.settings.method != AnalysisMethod::NonlinearEffects
        || !matches!(
            recipe.method_config.as_ref(),
            Some(MethodConfig::NonlinearEffects)
        )
    {
        return Err(invalid_pls_nonlinear(
            "resident Recipe-v4 does not request the exact nonlinear method",
        ));
    }
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let base = recipe_v4_pls_base_capability_cell_v1();
    let nonlinear = recipe_v4_pls_nonlinear_capability_cell_v1();
    let artifact = compile_analysis_recipe_v4(
        recipe,
        Some(model),
        target,
        target.capability_cell_for_method(AnalysisMethod::NonlinearEffects),
    )
    .map_err(|error| {
        invalid_pls_nonlinear(format!("nonlinear Recipe-v4 recompilation failed: {error}"))
    })?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
        unreachable!("the exact PLS target must return a PLS plan")
    };
    let receipt = artifact.receipt();
    let receipt_fingerprint = recorded_dataset_sha256(receipt.dataset_fingerprint())
        .ok_or_else(|| invalid_pls_nonlinear("compiled dataset fingerprint is not SHA-256"))?;
    let resident_fingerprint = recorded_dataset_sha256(&dataset.fingerprint.0)
        .ok_or_else(|| invalid_pls_nonlinear("resident dataset fingerprint is not SHA-256"))?;
    if document.provenance.method_version != NONLINEAR_EFFECTS_METHOD_VERSION
        || document.provenance.engine_version
            != RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        || document.provenance.recipe_id != receipt.recipe_id().to_string()
        || document.provenance.recipe_digest != receipt.recipe_analytical_sha256()
        || document.provenance.model_id != receipt.model_id()
        || document.provenance.model_digest != receipt.model_scientific_sha256()
        || document.provenance.dataset_id != plan.dataset_id()
        || document.provenance.dataset_id != dataset.id.to_string()
        || document.provenance.dataset_fingerprint != receipt_fingerprint
        || document.provenance.dataset_fingerprint != resident_fingerprint
        || document.provenance.capability_cell != nonlinear
        || document.provenance.seed != Some(recipe.settings.seed)
        || usize::try_from(document.provenance.workers).ok() != Some(recipe.settings.workers)
        || document.title != "PLS nonlinear quadratic diagnostics"
        || document.presentation.default_section_id.as_deref() != Some(PLS_NONLINEAR_SECTION_ID_V1)
        || document.presentation.default_table_id.as_deref()
            != Some(PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1)
    {
        return Err(invalid_pls_nonlinear(
            "nonlinear canonical provenance differs from resident Recipe-v4/model/dataset recompilation",
        ));
    }
    let expected_cells = [nonlinear.clone(), base.clone()];
    if document.capability_cells.as_deref() != Some(expected_cells.as_slice()) {
        return Err(invalid_pls_nonlinear(
            "nonlinear capability_cells must be ordered [primary nonlinear, base PLS]",
        ));
    }
    let section_ids = document
        .sections
        .iter()
        .map(|section| section.id.as_str())
        .collect::<Vec<_>>();
    if section_ids
        != [
            "run_details",
            "measurement_model",
            "structural_model",
            PLS_NONLINEAR_SECTION_ID_V1,
        ]
    {
        return Err(invalid_pls_nonlinear(
            "nonlinear section order or ownership boundary differs",
        ));
    }
    if !document.charts.is_empty()
        || document
            .sections
            .iter()
            .any(|section| !section.chart_ids.is_empty())
    {
        return Err(invalid_pls_nonlinear(
            "nonlinear adapter v7 canonical results must not contain charts",
        ));
    }
    let has_controls = plan
        .paths()
        .iter()
        .any(|path| path.role() == qpls_core::StructuralRelationRoleV4::Control);
    let expected_run_details = [
        "estimation_summary",
        crate::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1,
    ];
    let expected_measurement = ["outer_model"];
    let mut expected_structural = vec!["structural_paths", "effects", "r_squared"];
    if has_controls {
        expected_structural.push(crate::PLS_CONTROL_ESTIMATES_TABLE_ID_V2);
    }
    if document.sections[0]
        .table_ids
        .iter()
        .map(String::as_str)
        .ne(expected_run_details)
        || document.sections[1]
            .table_ids
            .iter()
            .map(String::as_str)
            .ne(expected_measurement)
        || document.sections[2]
            .table_ids
            .iter()
            .map(String::as_str)
            .ne(expected_structural.iter().copied())
    {
        return Err(invalid_pls_nonlinear(
            "nonlinear base PLS section table membership or order differs",
        ));
    }
    for section in &document.sections[..3] {
        if section.capability_cells.as_deref() != Some(std::slice::from_ref(&base)) {
            return Err(invalid_pls_nonlinear(
                "base PLS sections must be owned only by the base PLS cell",
            ));
        }
    }
    let nonlinear_section = &document.sections[3];
    if nonlinear_section.capability_cells.as_deref() != Some(std::slice::from_ref(&nonlinear))
        || nonlinear_section.table_ids
            != [
                PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1,
                PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1,
                PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1,
            ]
    {
        return Err(invalid_pls_nonlinear(
            "nonlinear_relationships has a drifted table order or capability owner",
        ));
    }
    for table_id in [
        PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1,
        PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1,
        PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1,
    ] {
        let references = document
            .sections
            .iter()
            .flat_map(|section| &section.table_ids)
            .filter(|candidate| candidate.as_str() == table_id)
            .count();
        if references != 1 {
            return Err(invalid_pls_nonlinear(format!(
                "nonlinear table {table_id} must belong exactly once to nonlinear_relationships"
            )));
        }
    }
    if document.tables.iter().any(|table| {
        matches!(
            table.id.as_str(),
            PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2 | PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2
        )
    }) {
        return Err(invalid_pls_nonlinear(
            "nonlinear adapter v7 cannot carry score-execution tables",
        ));
    }
    let nonlinear_ids = document
        .tables
        .iter()
        .filter(|table| {
            matches!(
                table.id.as_str(),
                PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1
                    | PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1
                    | PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1
            )
        })
        .map(|table| table.id.as_str())
        .collect::<Vec<_>>();
    let table_ids = document
        .tables
        .iter()
        .map(|table| table.id.as_str())
        .collect::<Vec<_>>();
    let mut expected_table_ids = vec![
        "estimation_summary",
        "outer_model",
        "structural_paths",
        "effects",
        "r_squared",
    ];
    if has_controls {
        expected_table_ids.push(crate::PLS_CONTROL_ESTIMATES_TABLE_ID_V2);
    }
    expected_table_ids.extend([
        crate::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1,
        PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1,
        PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1,
        PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1,
    ]);
    if nonlinear_ids
        != [
            PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1,
            PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1,
            PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1,
        ]
        || !table_ids.ends_with(&[
            PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1,
            PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1,
            PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1,
        ])
        || table_ids != expected_table_ids
    {
        return Err(invalid_pls_nonlinear(
            "nonlinear tables must occur exactly once at the canonical table tail",
        ));
    }
    for table in &document.tables {
        let nonlinear_table = matches!(
            table.id.as_str(),
            PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1
                | PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1
                | PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1
        );
        let expected = if nonlinear_table { &nonlinear } else { &base };
        if table.capability_cells.as_deref() != Some(std::slice::from_ref(expected)) {
            return Err(invalid_pls_nonlinear(format!(
                "canonical table {} has a drifted capability owner",
                table.id
            )));
        }
    }

    let diagnostics = pls_nonlinear_table(document, PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1)?;
    let equations = pls_nonlinear_table(document, PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1)?;
    let scope = pls_nonlinear_table(document, PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1)?;
    validate_pls_nonlinear_columns_v1(diagnostics, PLS_NONLINEAR_DIAGNOSTIC_COLUMNS_V1)?;
    validate_pls_nonlinear_columns_v1(equations, PLS_NONLINEAR_EQUATION_FIT_COLUMNS_V1)?;
    validate_pls_nonlinear_columns_v1(scope, PLS_NONLINEAR_METHOD_SCOPE_COLUMNS_V1)?;
    if diagnostics.rows.is_empty() {
        return Err(invalid_pls_nonlinear(
            "nonlinear_quadratic_diagnostics must not be empty",
        ));
    }

    let structural = pls_nonlinear_table(document, "structural_paths")?;
    validate_pls_nonlinear_columns_v1(structural, &["source", "target", "coefficient"])?;
    let mut structural_coefficients = BTreeMap::new();
    let mut structural_order = Vec::with_capacity(structural.rows.len());
    for (index, row) in structural.rows.iter().enumerate() {
        let source = pls_nonlinear_text_v1(&row.cells[0])?.to_owned();
        let target_name = pls_nonlinear_text_v1(&row.cells[1])?.to_owned();
        let source_target = (source.clone(), target_name.clone());
        if row.id != format!("path_{index:04}")
            || structural_order
                .last()
                .is_some_and(|previous| previous >= &source_target)
            || structural_coefficients
                .insert(
                    (target_name, source),
                    pls_nonlinear_number_v1(&row.cells[2])?,
                )
                .is_some()
        {
            return Err(invalid_pls_nonlinear(
                "structural_paths identity, order, or endpoint uniqueness differs",
            ));
        }
        structural_order.push(source_target);
    }
    let mut expected_paths = plan
        .paths()
        .iter()
        .map(|path| (path.target().to_owned(), path.source().to_owned()))
        .collect::<Vec<_>>();
    expected_paths.sort();
    if expected_paths.windows(2).any(|pair| pair[0] >= pair[1])
        || structural_coefficients.keys().cloned().collect::<Vec<_>>() != expected_paths
    {
        return Err(invalid_pls_nonlinear(
            "structural_paths differs from the recompiled PLS path family",
        ));
    }

    let mut diagnostic_keys = Vec::with_capacity(diagnostics.rows.len());
    let mut targets = BTreeSet::new();
    for (index, row) in diagnostics.rows.iter().enumerate() {
        let source = pls_nonlinear_text_v1(&row.cells[0])?.to_owned();
        let target_name = pls_nonlinear_text_v1(&row.cells[1])?.to_owned();
        let key = (target_name.clone(), source);
        if row.id != format!("nonlinear_quadratic_diagnostic_{index:04}")
            || diagnostic_keys
                .last()
                .is_some_and(|previous| previous >= &key)
        {
            return Err(invalid_pls_nonlinear(
                "nonlinear diagnostics are not strictly ordered by (target, source)",
            ));
        }
        let linear = pls_nonlinear_number_v1(&row.cells[2])?;
        let quadratic = pls_nonlinear_number_v1(&row.cells[3])?;
        let standard_error = pls_nonlinear_number_v1(&row.cells[4])?;
        let t_statistic = pls_nonlinear_number_v1(&row.cells[5])?;
        let p_value = pls_nonlinear_number_v1(&row.cells[6])?;
        validate_pls_nonlinear_optional_warning_v1(&row.cells[7])?;
        if standard_error <= 0.0
            || t_statistic.to_bits() != (quadratic / standard_error).to_bits()
            || !(0.0..=1.0).contains(&p_value)
            || structural_coefficients
                .get(&key)
                .is_none_or(|coefficient| coefficient.to_bits() != linear.to_bits())
        {
            return Err(invalid_pls_nonlinear(
                "nonlinear diagnostic numerical invariants differ",
            ));
        }
        diagnostic_keys.push(key);
        targets.insert(target_name);
    }
    if diagnostic_keys.len() != structural_coefficients.len()
        || diagnostic_keys
            .iter()
            .any(|key| !structural_coefficients.contains_key(key))
    {
        return Err(invalid_pls_nonlinear(
            "nonlinear diagnostic endpoints differ from structural_paths",
        ));
    }
    if equations.rows.len() != targets.len() {
        return Err(invalid_pls_nonlinear(
            "nonlinear equation-fit targets differ from diagnostics",
        ));
    }
    for (index, (row, expected_target)) in equations.rows.iter().zip(targets).enumerate() {
        if row.id != format!("nonlinear_equation_fit_{index:04}")
            || pls_nonlinear_text_v1(&row.cells[0])? != expected_target.as_str()
        {
            return Err(invalid_pls_nonlinear(
                "nonlinear equation-fit row identity or target order differs",
            ));
        }
        let linear = pls_nonlinear_number_v1(&row.cells[1])?;
        let augmented = pls_nonlinear_number_v1(&row.cells[2])?;
        let delta = pls_nonlinear_number_v1(&row.cells[3])?;
        if !(0.0..=1.0).contains(&linear)
            || !(0.0..=1.0).contains(&augmented)
            || delta.to_bits() != (augmented - linear).max(0.0).to_bits()
        {
            return Err(invalid_pls_nonlinear(
                "nonlinear equation-fit R-squared invariants differ",
            ));
        }
    }
    if scope.rows.len() != 1
        || scope.rows[0].id != PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1
        || pls_nonlinear_text_v1(&scope.rows[0].cells[0])? != NONLINEAR_EFFECTS_METHOD_VERSION
        || pls_nonlinear_text_v1(&scope.rows[0].cells[1])? != PLS_NONLINEAR_TERM_V1
        || pls_nonlinear_text_v1(&scope.rows[0].cells[2])? != PLS_NONLINEAR_ENGINE_WARNING_V1
    {
        return Err(invalid_pls_nonlinear(
            "nonlinear_method_scope differs from the exact method-v1 contract",
        ));
    }
    validate_recipe_v4_pls_nonlinear_base_projection_v1(document, recipe, model)
}

fn is_exact_recipe_v4_cbsem_result(document: &CanonicalResultDocumentV2) -> bool {
    let cell = &document.provenance.capability_cell;
    cell.registry_schema_version == 2
        && ((cell.capability_id == "smartpls.cbsem"
            && cell.cell_id == "qpls3.cbsem.ml"
            && cell.capability_version == "cbsem_ml_v1")
            || (cell.capability_id == "smartpls.cbsem_bootstrapping"
                && cell.cell_id == "qpls3.cbsem.bootstrap"
                && cell.capability_version == "cbsem_exact_case_bootstrap_v1"))
}

const CBSEM_EXACT_ADAPTER_LISTWISE_HISTORICAL: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v2";
const CBSEM_EXACT_ADAPTER_MEAN_HISTORICAL: &str = "compiled_recipe_v4_cbsem_plan_v2_execution_v3";
const CBSEM_EXACT_ADAPTER_LISTWISE_CURRENT: &str = "compiled_recipe_v4_cbsem_plan_v2_execution_v5";
const CBSEM_EXACT_ADAPTER_MEAN_CURRENT: &str = "compiled_recipe_v4_cbsem_plan_v2_execution_v6";
const CBSEM_EXACT_ADAPTER_LISTWISE_SCORE_LM_CURRENT: &str =
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V2;
const CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_HISTORICAL: &str =
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V3;
const CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_CURRENT: &str =
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V4;
const CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_STUDENTIZED_CURRENT: &str =
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V5;
const CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_BCA_CURRENT: &str =
    CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V6;
const CBSEM_EXACT_BOOTSTRAP_SECTION_ID: &str = "bootstrap_inference";
const CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID: &str = "exact_case_bootstrap_summary";
const CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID: &str = "exact_case_bootstrap_parameter_intervals";
const CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID: &str = "exact_case_bootstrap_successful_refits";
const CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID: &str = "exact_case_bootstrap_failures";
const CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID: &str = "bootstrap_hypothesis_tests";
const CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID: &str = "exact_case_bootstrap_hypothesis_tests";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID: &str = "bootstrap_studentized_inference";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID: &str =
    "exact_case_bootstrap_studentized_summary";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID: &str =
    "exact_case_bootstrap_studentized_point_standard_errors";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID: &str =
    "exact_case_bootstrap_studentized_parameter_intervals";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID: &str =
    "exact_case_bootstrap_studentized_refit_standard_errors";
const CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID: &str = "bootstrap_bca_inference";
const CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID: &str = "exact_case_bootstrap_bca_summary";
const CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID: &str =
    "exact_case_bootstrap_bca_parameter_intervals";
const CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID: &str =
    "exact_case_bootstrap_bca_successful_delete_one_refits";
const CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID: &str = "exact_case_bootstrap_bca_failures";
const CBSEM_EXACT_BOOTSTRAP_BCA_ARCHIVE_SCOPE: &str =
    "ledger_identity_digest_and_arithmetic_replay_only_no_raw_base_or_delete_one_ml_replay_v1";
const CBSEM_EXACT_BOOTSTRAP_ARCHIVE_SCOPE: &str =
    "schedule_and_arithmetic_only_no_raw_refit_replay_or_source_row_digest_recomputation";
const CBSEM_EXACT_BOOTSTRAP_SUMMARY_COLUMNS: &[&str] = &[
    "method_version",
    "estimator_method_version",
    "source_dataset_id",
    "source_dataset_fingerprint",
    "outer_recipe_analytical_identity_sha256",
    "base_point_result_sha256",
    "compiler_analytical_identity_sha256",
    "plan_sha256",
    "model_scientific_sha256",
    "complete_case_sample_size",
    "complete_case_universe_digest_method",
    "complete_case_universe_sha256",
    "covariance_denominator",
    "sample_indices_digest_method",
    "sampling_positions_digest_method",
    "interval_method",
    "confidence_level",
    "requested_replicates",
    "attempted_refits",
    "usable_replicates",
    "failed_replicates",
    "minimum_usable_fraction",
    "minimum_usable_replicates",
    "seed_decimal",
    "stream_token",
    "retry_policy",
    "max_attempts_per_replicate",
    "parameter_ids_json",
    "inference_status",
    "unavailable_reason_code",
    "unavailable_message",
    "archive_validation_scope",
];
const CBSEM_EXACT_BOOTSTRAP_INTERVAL_COLUMNS: &[&str] = &[
    "parameter_id",
    "original",
    "bootstrap_mean",
    "bias",
    "standard_error",
    "percentile_lower",
    "percentile_upper",
    "usable_replicates",
];
const CBSEM_EXACT_BOOTSTRAP_REFIT_COLUMNS: &[&str] = &[
    "replicate_index",
    "sampling_positions_sha256",
    "sample_indices_sha256",
    "parameter_estimates_json",
    "iterations",
    "objective",
    "gradient_norm",
];
const CBSEM_EXACT_BOOTSTRAP_FAILURE_COLUMNS: &[&str] = &[
    "replicate_index",
    "sampling_positions_sha256",
    "sample_indices_sha256",
    "kind",
    "message",
];
const CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS: &[&str] = &[
    "method_version",
    "null_hypothesis",
    "statistic",
    "tie_policy",
    "probability_method",
    "decision_rule",
    "selected_test_tail",
    "null_value",
    "significance_level",
    "usable_replicates",
    "inference_status",
    "global_unavailable_reason_code",
    "global_unavailable_message",
    "parameter_id",
    "parameter_status",
    "point_estimate",
    "two_sided_exceedances",
    "greater_or_equal_exceedances",
    "less_or_equal_exceedances",
    "p_value_two_sided",
    "p_value_greater",
    "p_value_less",
    "selected_exceedances",
    "selected_p_value",
    "reject_null",
    "unavailable_reason",
];
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_COLUMNS: &[&str] = &[
    "method_version",
    "standard_error_method_version",
    "expected_information_method",
    "pivot_method",
    "quantile_method",
    "interval_method",
    "archive_validation_scope",
    "confidence_level",
    "minimum_usable_fraction",
    "minimum_usable_replicates",
    "studentized_usable_replicates",
    "parameter_ids_json",
    "inference_status",
    "unavailable_reason_code",
    "unavailable_message",
];
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERROR_COLUMNS: &[&str] = &[
    "method_version",
    "parameter_id",
    "status",
    "information_method",
    "standard_error",
    "unavailable_reason",
];
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_COLUMNS: &[&str] = &[
    "parameter_id",
    "status",
    "point_estimate",
    "point_standard_error",
    "lower_pivot_quantile",
    "upper_pivot_quantile",
    "interval_lower",
    "interval_upper",
    "usable_replicates",
    "unavailable_reason",
];
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERROR_COLUMNS: &[&str] = &[
    "replicate_index",
    "status",
    "information_method",
    "standard_errors_json",
    "unavailable_reason",
];
const CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_COLUMNS: &[&str] = &[
    "method_version",
    "base_bootstrap_method_version",
    "outer_recipe_analytical_identity_sha256",
    "base_point_result_sha256",
    "compiler_analytical_identity_sha256",
    "plan_sha256",
    "model_scientific_sha256",
    "delete_one_refit_method_version",
    "delete_one_sampling_positions_digest_method",
    "delete_one_sample_indices_digest_method",
    "bias_correction_method",
    "acceleration_method",
    "adjusted_probability_method",
    "quantile_method",
    "retry_policy",
    "archive_validation_scope",
    "confidence_level",
    "bootstrap_usable_replicates",
    "minimum_bootstrap_usable_replicates",
    "delete_one_case_count",
    "successful_delete_one_refits",
    "failed_delete_one_refits",
    "parameter_ids_json",
    "inference_status",
    "unavailable_reason_code",
    "unavailable_message",
];
const CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_COLUMNS: &[&str] = &[
    "parameter_id",
    "status",
    "point_estimate",
    "bias_correction",
    "acceleration",
    "adjusted_lower_probability",
    "adjusted_upper_probability",
    "interval_lower",
    "interval_upper",
    "usable_replicates",
    "unavailable_reason",
];
const CBSEM_EXACT_BOOTSTRAP_BCA_REFIT_COLUMNS: &[&str] = &[
    "omitted_complete_case_position",
    "omitted_source_row_index",
    "retained_sampling_positions_sha256",
    "retained_sample_indices_sha256",
    "parameter_estimates_json",
    "iterations",
    "objective",
    "gradient_norm",
];
const CBSEM_EXACT_BOOTSTRAP_BCA_FAILURE_COLUMNS: &[&str] = &[
    "omitted_complete_case_position",
    "omitted_source_row_index",
    "retained_sampling_positions_sha256",
    "retained_sample_indices_sha256",
    "kind",
    "message",
];

const CBSEM_HISTORICAL_FIT_COLUMNS: &[&str] = &[
    "chi_square",
    "degrees_of_freedom",
    "p_value",
    "cfi",
    "tli",
    "rmsea",
    "srmr",
    "aic",
    "bic",
];

const CBSEM_CURRENT_FIT_COLUMNS: &[&str] = &[
    "fit_method_version",
    "chi_square",
    "degrees_of_freedom",
    "p_value",
    "cfi",
    "tli",
    "rmsea",
    "rmsea_interval_method_version",
    "rmsea_interval_confidence_level",
    "rmsea_ci_lower",
    "rmsea_ci_upper",
    "srmr",
    "aic",
    "bic",
];

fn invalid_cbsem_rmsea(message: impl Into<String>) -> ProjectArchiveV6Error {
    ProjectArchiveV6Error::CanonicalCbsemRmseaFit(message.into())
}

fn exact_cbsem_optional_number(cell: &CanonicalResultCellV2, expected: Option<f64>) -> bool {
    match (cell, expected) {
        (
            CanonicalResultCellV2::Number {
                value,
                display: None,
            },
            Some(expected),
        ) => value.to_bits() == expected.to_bits(),
        (
            CanonicalResultCellV2::Missing {
                reason: CanonicalMissingReasonV2::NotEstimated,
                display: None,
            },
            None,
        ) => true,
        _ => false,
    }
}

/// Validates both the explicitly allowlisted historical fit table and the
/// current typed noncentral-chi-square RMSEA interval contract.
pub fn validate_recipe_v4_cbsem_rmsea_fit_document_v1(
    document: &CanonicalResultDocumentV2,
) -> Result<(), ProjectArchiveV6Error> {
    let current = match (
        document.provenance.method_version.as_str(),
        document.provenance.engine_version.as_str(),
    ) {
        (
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
            CBSEM_EXACT_ADAPTER_LISTWISE_HISTORICAL
            | CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V1,
        )
        | (CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4, CBSEM_EXACT_ADAPTER_MEAN_HISTORICAL) => {
            false
        }
        (
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
            CBSEM_EXACT_ADAPTER_LISTWISE_CURRENT
            | CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V2
            | CBSEM_EXACT_ADAPTER_LISTWISE_SCORE_LM_CURRENT
            | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_HISTORICAL
            | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_CURRENT
            | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_STUDENTIZED_CURRENT
            | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_BCA_CURRENT,
        )
        | (CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4, CBSEM_EXACT_ADAPTER_MEAN_CURRENT) => true,
        _ => {
            return Err(invalid_cbsem_rmsea(
                "unsupported exact CB-SEM estimator/adapter identity",
            ));
        }
    };

    let fit_tables = document
        .tables
        .iter()
        .filter(|table| table.id == "fit_indices")
        .collect::<Vec<_>>();
    let [fit] = fit_tables.as_slice() else {
        return Err(invalid_cbsem_rmsea("fit_indices must occur exactly once"));
    };
    let expected_columns = if current {
        CBSEM_CURRENT_FIT_COLUMNS
    } else {
        CBSEM_HISTORICAL_FIT_COLUMNS
    };
    if fit
        .columns
        .iter()
        .map(|column| column.id.as_str())
        .ne(expected_columns.iter().copied())
    {
        return Err(invalid_cbsem_rmsea(
            "fit_indices columns do not match the adapter generation",
        ));
    }
    if fit.rows.len() != 1 || fit.rows[0].id != "model" {
        return Err(invalid_cbsem_rmsea(
            "fit_indices must contain exactly the model row",
        ));
    }
    if !current {
        return Ok(());
    }
    let cells = &fit.rows[0].cells;
    if cells.len() != CBSEM_CURRENT_FIT_COLUMNS.len() {
        return Err(invalid_cbsem_rmsea("fit_indices row width drifted"));
    }
    let CanonicalResultCellV2::Text { value: fit_method } = &cells[0] else {
        return Err(invalid_cbsem_rmsea("fit method identity is not text"));
    };
    let CanonicalResultCellV2::Number {
        value: chi_square,
        display: None,
    } = &cells[1]
    else {
        return Err(invalid_cbsem_rmsea(
            "chi-square is not canonical numeric data",
        ));
    };
    let CanonicalResultCellV2::Number {
        value: degrees_of_freedom,
        display: None,
    } = &cells[2]
    else {
        return Err(invalid_cbsem_rmsea(
            "degrees of freedom are not canonical numeric data",
        ));
    };
    if chi_square.to_bits() == (-0.0_f64).to_bits() {
        return Err(invalid_cbsem_rmsea(
            "chi-square cannot use negative-zero encoding",
        ));
    }
    let CanonicalResultCellV2::Text {
        value: interval_method,
    } = &cells[7]
    else {
        return Err(invalid_cbsem_rmsea(
            "RMSEA interval method identity is not text",
        ));
    };
    let CanonicalResultCellV2::Number {
        value: confidence_level,
        display: None,
    } = &cells[8]
    else {
        return Err(invalid_cbsem_rmsea(
            "RMSEA interval confidence is not canonical numeric data",
        ));
    };
    if fit_method != CBSEM_FIT_METHOD_VERSION
        || interval_method != CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1
        || confidence_level.to_bits() != 0.90_f64.to_bits()
    {
        return Err(invalid_cbsem_rmsea(
            "RMSEA interval typed attribution drifted",
        ));
    }
    if !degrees_of_freedom.is_finite()
        || *degrees_of_freedom < i64::MIN as f64
        || *degrees_of_freedom > i64::MAX as f64
    {
        return Err(invalid_cbsem_rmsea("degrees of freedom are invalid"));
    }
    let degrees_of_freedom_i64 = *degrees_of_freedom as i64;
    if degrees_of_freedom.to_bits() != (degrees_of_freedom_i64 as f64).to_bits() {
        return Err(invalid_cbsem_rmsea(
            "degrees of freedom are not an exact integer",
        ));
    }

    let summaries = document
        .tables
        .iter()
        .filter(|table| table.id == "estimation_summary")
        .collect::<Vec<_>>();
    let [summary] = summaries.as_slice() else {
        return Err(invalid_cbsem_rmsea(
            "estimation_summary must occur exactly once",
        ));
    };
    let sample_positions = summary
        .columns
        .iter()
        .enumerate()
        .filter_map(|(index, column)| (column.id == "sample_size").then_some(index))
        .collect::<Vec<_>>();
    let [sample_position] = sample_positions.as_slice() else {
        return Err(invalid_cbsem_rmsea(
            "sample_size column must occur exactly once",
        ));
    };
    let [summary_row] = summary.rows.as_slice() else {
        return Err(invalid_cbsem_rmsea(
            "estimation_summary must contain exactly one row",
        ));
    };
    let Some(CanonicalResultCellV2::Number {
        value: sample_size,
        display: None,
    }) = summary_row.cells.get(*sample_position)
    else {
        return Err(invalid_cbsem_rmsea(
            "sample_size is not canonical numeric data",
        ));
    };
    if !sample_size.is_finite() || *sample_size < 0.0 || *sample_size > usize::MAX as f64 {
        return Err(invalid_cbsem_rmsea("sample_size is invalid"));
    }
    let sample_size_usize = *sample_size as usize;
    if sample_size.to_bits() != (sample_size_usize as f64).to_bits() {
        return Err(invalid_cbsem_rmsea("sample_size is not an exact integer"));
    }
    let expected = cbsem_exact_rmsea_90_percent_interval_v1(
        *chi_square,
        degrees_of_freedom_i64,
        sample_size_usize,
    )
    .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
    if !exact_cbsem_optional_number(&cells[6], expected.0)
        || !exact_cbsem_optional_number(&cells[9], expected.1)
        || !exact_cbsem_optional_number(&cells[10], expected.2)
    {
        return Err(invalid_cbsem_rmsea(
            "RMSEA point or interval values do not match exact recomputation",
        ));
    }
    Ok(())
}

const CBSEM_SCORE_LM_TABLE_ID_V1: &str = "modification_index_score_tests";
const CBSEM_SCORE_LM_SECTION_ID_V1: &str = "modification_indices";
const CBSEM_SCORE_LM_COLUMNS_V1: &[&str] = &[
    "method_version",
    "scope",
    "parameter_id",
    "kind",
    "lhs",
    "rhs",
    "status",
    "score",
    "efficient_score",
    "candidate_information",
    "efficient_information",
    "modification_index",
    "expected_parameter_change",
    "degrees_of_freedom",
    "p_value",
    "unavailable_reason",
];

fn exact_score_lm_number(cell: &CanonicalResultCellV2) -> Option<f64> {
    match cell {
        CanonicalResultCellV2::Number {
            value,
            display: None,
        } if value.is_finite() && value.to_bits() != (-0.0_f64).to_bits() => Some(*value),
        _ => None,
    }
}

fn exact_score_lm_text(cell: &CanonicalResultCellV2) -> Option<&str> {
    match cell {
        CanonicalResultCellV2::Text { value } => Some(value),
        _ => None,
    }
}

fn exact_score_lm_missing(cell: &CanonicalResultCellV2, reason: CanonicalMissingReasonV2) -> bool {
    matches!(
        cell,
        CanonicalResultCellV2::Missing {
            reason: actual,
            display: None,
        } if *actual == reason
    )
}

fn recorded_dataset_sha256(value: &str) -> Option<&str> {
    let candidate = value.rsplit_once(':').map_or(value, |(_, suffix)| suffix);
    (candidate.len() == 64
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    .then_some(candidate)
}

fn validate_recipe_v4_cbsem_score_lm_document_v1(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    dataset: &DatasetDescriptor,
) -> Result<(), ProjectArchiveV6Error> {
    let is_current_score_lm = matches!(
        document.provenance.engine_version.as_str(),
        CBSEM_EXACT_ADAPTER_LISTWISE_SCORE_LM_CURRENT
            | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_HISTORICAL
            | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_CURRENT
            | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_STUDENTIZED_CURRENT
            | CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_BCA_CURRENT
    );
    let score_tables = document
        .tables
        .iter()
        .filter(|table| {
            matches!(
                table.id.as_str(),
                CBSEM_SCORE_LM_TABLE_ID_V1 | "modification_indices"
            )
        })
        .collect::<Vec<_>>();
    let score_sections = document
        .sections
        .iter()
        .filter(|section| section.id == CBSEM_SCORE_LM_SECTION_ID_V1)
        .collect::<Vec<_>>();
    if !is_current_score_lm {
        if score_tables.is_empty() && score_sections.is_empty() {
            return Ok(());
        }
        return Err(invalid_cbsem_rmsea(
            "legacy CB-SEM adapter carries a score/LM or heuristic MI table",
        ));
    }
    let [table] = score_tables.as_slice() else {
        return Err(invalid_cbsem_rmsea(
            "current score/LM adapters require exactly one canonical table",
        ));
    };
    if table.id != CBSEM_SCORE_LM_TABLE_ID_V1 {
        return Err(invalid_cbsem_rmsea(
            "current score/LM adapters cannot substitute the legacy heuristic modification-index table",
        ));
    }
    let [section] = score_sections.as_slice() else {
        return Err(invalid_cbsem_rmsea(
            "current score/LM adapters require exactly one modification-indices section",
        ));
    };
    if section.table_ids.len() != 1
        || section.table_ids[0] != CBSEM_SCORE_LM_TABLE_ID_V1
        || !section.chart_ids.is_empty()
    {
        return Err(invalid_cbsem_rmsea(
            "current modification-indices section membership drifted",
        ));
    }
    if table
        .columns
        .iter()
        .map(|column| column.id.as_str())
        .ne(CBSEM_SCORE_LM_COLUMNS_V1.iter().copied())
    {
        return Err(invalid_cbsem_rmsea("current score/LM column order drifted"));
    }

    let target = RecipeV4CompilerTarget::CbsemPlanV2;
    let artifact =
        compile_analysis_recipe_v4(recipe, Some(model), target, target.capability_cell())
            .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
    if document.provenance.recipe_id != recipe.id.to_string()
        || document.provenance.model_id.as_str() != model.id.as_str()
        || document.provenance.model_digest
            != model
                .scientific_sha256()
                .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?
        || document.provenance.dataset_id != dataset.id.to_string()
        || recorded_dataset_sha256(artifact.receipt().dataset_fingerprint())
            != Some(document.provenance.dataset_fingerprint.as_str())
        || recorded_dataset_sha256(&dataset.fingerprint.0)
            != Some(document.provenance.dataset_fingerprint.as_str())
    {
        return Err(invalid_cbsem_rmsea(
            "current score/LM recipe/model/dataset provenance is not cross-bound",
        ));
    }
    let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
        return Err(invalid_cbsem_rmsea(
            "current score/LM recipe did not compile to the CB-SEM plan",
        ));
    };
    let parameters = plan
        .parameters()
        .iter()
        .map(|parameter| (parameter.id(), parameter))
        .collect::<BTreeMap<_, _>>();
    let observed_sources = plan
        .variables()
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } => Some((id.as_str(), source_column.as_str())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let mut expected = plan
        .covariances()
        .iter()
        .filter_map(|covariance| {
            let parameter = parameters.get(covariance.parameter_id())?;
            let CompiledCbsemParameterStatusV2::Fixed { value } = parameter.specification() else {
                return None;
            };
            if !covariance.is_residual_covariance() || value.to_bits() != 0.0_f64.to_bits() {
                return None;
            }
            let (SemEndpointV4::ResidualOf(left), SemEndpointV4::ResidualOf(right)) =
                (covariance.left(), covariance.right())
            else {
                return None;
            };
            let SemParameterTargetV4::Covariance {
                left: parameter_left,
                right: parameter_right,
            } = parameter.target()
            else {
                return None;
            };
            if parameter_left != covariance.left() || parameter_right != covariance.right() {
                return None;
            }
            let left_source = *observed_sources.get(left.as_str())?;
            let right_source = *observed_sources.get(right.as_str())?;
            let (lhs, rhs) = if left_source <= right_source {
                (left_source, right_source)
            } else {
                (right_source, left_source)
            };
            Some((parameter.id(), lhs, rhs))
        })
        .collect::<Vec<_>>();
    expected.sort_by(|left, right| left.0.cmp(right.0));
    if table.rows.len() != expected.len() {
        return Err(invalid_cbsem_rmsea(
            "v8 score/LM row count differs from compiled fixed-zero residual covariances",
        ));
    }
    let chi_square =
        ChiSquared::new(1.0).map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
    for (index, (row, (parameter_id, lhs, rhs))) in table.rows.iter().zip(expected).enumerate() {
        if row.id != format!("score_lm_{index:04}")
            || row.cells.len() != CBSEM_SCORE_LM_COLUMNS_V1.len()
        {
            return Err(invalid_cbsem_rmsea(
                "v8 score/LM row identity or width drifted",
            ));
        }
        let cells = &row.cells;
        if exact_score_lm_text(&cells[0]) != Some(CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1)
            || exact_score_lm_text(&cells[1]) != Some(CBSEM_CFA_SCORE_LM_SCOPE_V1)
            || exact_score_lm_text(&cells[2]) != Some(parameter_id)
            || exact_score_lm_text(&cells[3]) != Some("residual_covariance")
            || exact_score_lm_text(&cells[4]) != Some(lhs)
            || exact_score_lm_text(&cells[5]) != Some(rhs)
        {
            return Err(invalid_cbsem_rmsea(
                "v8 score/LM method, scope, or compiled parameter binding drifted",
            ));
        }
        match exact_score_lm_text(&cells[6]) {
            Some("available") => {
                let values = cells[7..=14]
                    .iter()
                    .map(exact_score_lm_number)
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        invalid_cbsem_rmsea("available score/LM row has null or non-finite values")
                    })?;
                let efficient_score = values[1];
                let candidate_information = values[2];
                let efficient_information = values[3];
                let modification_index = values[4];
                let expected_parameter_change = values[5];
                let degrees_of_freedom = values[6];
                let p_value = values[7];
                let expected_mi = efficient_score * efficient_score / efficient_information;
                let expected_epc = efficient_score / efficient_information;
                let expected_p = (1.0 - chi_square.cdf(expected_mi)).clamp(0.0, 1.0);
                if candidate_information <= 0.0
                    || efficient_information <= 0.0
                    || modification_index.to_bits() != expected_mi.to_bits()
                    || expected_parameter_change.to_bits() != expected_epc.to_bits()
                    || degrees_of_freedom.to_bits() != 1.0_f64.to_bits()
                    || p_value.to_bits() != expected_p.to_bits()
                    || !exact_score_lm_missing(&cells[15], CanonicalMissingReasonV2::NotApplicable)
                {
                    return Err(invalid_cbsem_rmsea(
                        "available score/LM arithmetic, p-value, df, or null contract drifted",
                    ));
                }
            }
            Some("unavailable") => {
                if !cells[7..=14].iter().all(|cell| {
                    exact_score_lm_missing(cell, CanonicalMissingReasonV2::NotEstimated)
                }) || !matches!(
                    exact_score_lm_text(&cells[15]),
                    Some(
                        "nuisance_information_unavailable"
                            | "efficient_information_non_positive"
                            | "non_finite_computation"
                    )
                ) {
                    return Err(invalid_cbsem_rmsea(
                        "unavailable score/LM status, reason, or null contract drifted",
                    ));
                }
            }
            _ => return Err(invalid_cbsem_rmsea("v8 score/LM status is unsupported")),
        }
    }
    Ok(())
}

fn exact_bootstrap_table<'a>(
    document: &'a CanonicalResultDocumentV2,
    id: &str,
) -> Result<&'a CanonicalResultTableV2, ProjectArchiveV6Error> {
    let matching = document
        .tables
        .iter()
        .filter(|table| table.id == id)
        .collect::<Vec<_>>();
    let [table] = matching.as_slice() else {
        return Err(invalid_cbsem_rmsea(format!(
            "v9 requires exactly one {id} table"
        )));
    };
    Ok(*table)
}

fn exact_bootstrap_cell<'a>(
    table: &'a CanonicalResultTableV2,
    row: usize,
    column: &str,
) -> Option<&'a CanonicalResultCellV2> {
    let column = table
        .columns
        .iter()
        .position(|candidate| candidate.id == column)?;
    table.rows.get(row)?.cells.get(column)
}

fn exact_bootstrap_number(cell: &CanonicalResultCellV2) -> Option<f64> {
    match cell {
        CanonicalResultCellV2::Number {
            value,
            display: None,
        } if value.is_finite() && value.to_bits() != (-0.0_f64).to_bits() => Some(*value),
        _ => None,
    }
}

fn exact_bootstrap_text(cell: &CanonicalResultCellV2) -> Option<&str> {
    match cell {
        CanonicalResultCellV2::Text { value } => Some(value),
        _ => None,
    }
}

fn exact_bootstrap_boolean(cell: &CanonicalResultCellV2) -> Option<bool> {
    match cell {
        CanonicalResultCellV2::Boolean { value } => Some(*value),
        _ => None,
    }
}

fn exact_bootstrap_u32(cell: &CanonicalResultCellV2) -> Option<u32> {
    let value = exact_bootstrap_number(cell)?;
    (value >= 0.0 && value.fract() == 0.0 && value <= f64::from(u32::MAX)).then_some(value as u32)
}

fn exact_bootstrap_usize(cell: &CanonicalResultCellV2) -> Option<usize> {
    let value = exact_bootstrap_number(cell)?;
    (value >= 0.0 && value.fract() == 0.0 && value <= usize::MAX as f64).then_some(value as usize)
}

fn exact_bootstrap_i64(cell: &CanonicalResultCellV2) -> Option<i64> {
    let value = exact_bootstrap_number(cell)?;
    (value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64)
        .then_some(value as i64)
}

fn exact_bootstrap_optional_number(cell: &CanonicalResultCellV2) -> Option<Option<f64>> {
    match cell {
        CanonicalResultCellV2::Missing {
            reason: CanonicalMissingReasonV2::NotEstimated,
            display: None,
        } => Some(None),
        _ => exact_bootstrap_number(cell).map(Some),
    }
}

fn exact_bootstrap_is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn exact_bootstrap_column_order(table: &CanonicalResultTableV2, expected: &[&str]) -> bool {
    table
        .columns
        .iter()
        .map(|column| column.id.as_str())
        .eq(expected.iter().copied())
        && table
            .rows
            .iter()
            .all(|row| row.cells.len() == expected.len())
}

fn exact_bootstrap_score_lm_projection(
    document: &CanonicalResultDocumentV2,
) -> Result<CbsemCfaScoreLmBundleV1, ProjectArchiveV6Error> {
    let table = exact_bootstrap_table(document, CBSEM_SCORE_LM_TABLE_ID_V1)?;
    let mut rows = Vec::with_capacity(table.rows.len());
    for (index, row) in table.rows.iter().enumerate() {
        if row.id != format!("score_lm_{index:04}") || row.cells.len() != 16 {
            return Err(invalid_cbsem_rmsea(
                "v9 base-point score/LM row identity drifted",
            ));
        }
        let text_at = |index| exact_bootstrap_text(&row.cells[index]);
        let outcome = match text_at(6) {
            Some("available") => CbsemCfaScoreLmOutcomeV1::Available {
                score: exact_bootstrap_number(&row.cells[7])
                    .ok_or_else(|| invalid_cbsem_rmsea("v9 score/LM score is invalid"))?,
                efficient_score: exact_bootstrap_number(&row.cells[8])
                    .ok_or_else(|| invalid_cbsem_rmsea("v9 score/LM efficient score is invalid"))?,
                candidate_information: exact_bootstrap_number(&row.cells[9]).ok_or_else(|| {
                    invalid_cbsem_rmsea("v9 score/LM candidate information is invalid")
                })?,
                efficient_information: exact_bootstrap_number(&row.cells[10]).ok_or_else(|| {
                    invalid_cbsem_rmsea("v9 score/LM efficient information is invalid")
                })?,
                modification_index: exact_bootstrap_number(&row.cells[11]).ok_or_else(|| {
                    invalid_cbsem_rmsea("v9 score/LM modification index is invalid")
                })?,
                expected_parameter_change: exact_bootstrap_number(&row.cells[12])
                    .ok_or_else(|| invalid_cbsem_rmsea("v9 score/LM EPC is invalid"))?,
                p_value: exact_bootstrap_number(&row.cells[14])
                    .ok_or_else(|| invalid_cbsem_rmsea("v9 score/LM p-value is invalid"))?,
            },
            Some("unavailable") => {
                let reason = match text_at(15) {
                    Some("nuisance_information_unavailable") => {
                        CbsemCfaScoreLmUnavailableReasonV1::NuisanceInformationUnavailable
                    }
                    Some("efficient_information_non_positive") => {
                        CbsemCfaScoreLmUnavailableReasonV1::EfficientInformationNonPositive
                    }
                    Some("non_finite_computation") => {
                        CbsemCfaScoreLmUnavailableReasonV1::NonFiniteComputation
                    }
                    _ => {
                        return Err(invalid_cbsem_rmsea(
                            "v9 score/LM unavailable reason is invalid",
                        ));
                    }
                };
                CbsemCfaScoreLmOutcomeV1::Unavailable { reason }
            }
            _ => return Err(invalid_cbsem_rmsea("v9 score/LM status is invalid")),
        };
        rows.push(CbsemCfaScoreLmRowV1 {
            parameter_id: text_at(2)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 score/LM parameter ID is invalid"))?
                .into(),
            kind: text_at(3)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 score/LM kind is invalid"))?
                .into(),
            lhs: text_at(4)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 score/LM lhs is invalid"))?
                .into(),
            rhs: text_at(5)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 score/LM rhs is invalid"))?
                .into(),
            outcome,
        });
    }
    Ok(CbsemCfaScoreLmBundleV1 {
        method_version: CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1.into(),
        scope: CBSEM_CFA_SCORE_LM_SCOPE_V1.into(),
        rows,
    })
}

fn exact_bootstrap_base_point_projection(
    document: &CanonicalResultDocumentV2,
) -> Result<CbsemExactCaseBootstrapBasePointDigestProjectionV1, ProjectArchiveV6Error> {
    let summary = exact_bootstrap_table(document, "estimation_summary")?;
    let parameters = exact_bootstrap_table(document, "parameters")?;
    let fit = exact_bootstrap_table(document, "fit_indices")?;
    let summary_columns = [
        "model_type",
        "estimator",
        "execution_adapter_version",
        "estimator_method_version",
        "moment_input_method_version",
        "compiled_moment_schema_version",
        "mean_structure",
        "input",
        "converged",
        "iterations",
        "objective",
        "gradient_norm",
        "sample_size",
        "declared_sample_size",
        "omitted_observations",
        "covariance_denominator",
        "canonical_covariance_sha256",
        "canonical_observed_means_sha256",
    ];
    let parameter_columns = [
        "name",
        "parameter_id",
        "kind",
        "lhs",
        "rhs",
        "estimate",
        "standard_error",
        "z",
        "p_two_sided",
        "fixed",
    ];
    if !exact_bootstrap_column_order(summary, &summary_columns)
        || !exact_bootstrap_column_order(parameters, &parameter_columns)
        || !exact_bootstrap_column_order(fit, CBSEM_CURRENT_FIT_COLUMNS)
    {
        return Err(invalid_cbsem_rmsea(
            "v9 base-point summary, parameter, or fit column order drifted",
        ));
    }
    if summary.rows.len() != 1
        || summary.rows[0].id != "run"
        || fit.rows.len() != 1
        || fit.rows[0].id != "model"
    {
        return Err(invalid_cbsem_rmsea(
            "v9 base-point summary or fit row identity drifted",
        ));
    }
    let summary_cell = |id: &str| {
        exact_bootstrap_cell(summary, 0, id)
            .ok_or_else(|| invalid_cbsem_rmsea(format!("v9 point summary omits {id}")))
    };
    let fit_cell = |id: &str| {
        exact_bootstrap_cell(fit, 0, id)
            .ok_or_else(|| invalid_cbsem_rmsea(format!("v9 fit table omits {id}")))
    };
    let observed_means = match summary_cell("canonical_observed_means_sha256")? {
        CanonicalResultCellV2::Missing {
            reason: CanonicalMissingReasonV2::NotEstimated,
            display: None,
        } => None,
        CanonicalResultCellV2::Text { value } if exact_bootstrap_is_sha256(value) => {
            Some(value.clone())
        }
        _ => {
            return Err(invalid_cbsem_rmsea(
                "v9 point observed-means digest has invalid null or text form",
            ));
        }
    };
    let mut point_parameters = Vec::with_capacity(parameters.rows.len());
    for (index, row) in parameters.rows.iter().enumerate() {
        if row.id != format!("parameter_{index:04}") || row.cells.len() != parameters.columns.len()
        {
            return Err(invalid_cbsem_rmsea(
                "v9 base-point parameter row identity or width drifted",
            ));
        }
        let cell = |id: &str| {
            exact_bootstrap_cell(parameters, index, id)
                .ok_or_else(|| invalid_cbsem_rmsea(format!("v9 base-point parameter omits {id}")))
        };
        point_parameters.push(CbsemExactCaseBootstrapBasePointParameterV1 {
            parameter_id: exact_bootstrap_text(cell("parameter_id")?)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 point parameter ID is invalid"))?
                .into(),
            name: exact_bootstrap_text(cell("name")?)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 point parameter name is invalid"))?
                .into(),
            kind: exact_bootstrap_text(cell("kind")?)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 point parameter kind is invalid"))?
                .into(),
            lhs: exact_bootstrap_text(cell("lhs")?)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 point parameter lhs is invalid"))?
                .into(),
            rhs: exact_bootstrap_text(cell("rhs")?)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 point parameter rhs is invalid"))?
                .into(),
            estimate: exact_bootstrap_number(cell("estimate")?)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 point parameter estimate is invalid"))?,
            standard_error: exact_bootstrap_optional_number(cell("standard_error")?)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 point parameter SE is invalid"))?,
            z_statistic: exact_bootstrap_optional_number(cell("z")?)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 point parameter z is invalid"))?,
            p_value_two_sided: exact_bootstrap_optional_number(cell("p_two_sided")?)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 point parameter p-value is invalid"))?,
            fixed: exact_bootstrap_boolean(cell("fixed")?)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 point parameter fixed flag is invalid"))?,
        });
    }
    let covariance_denominator = match exact_bootstrap_text(summary_cell("covariance_denominator")?)
    {
        Some("maximum_likelihood_n") => qpls_core::SemCovarianceDenominatorV4::MaximumLikelihoodN,
        Some("sample_n_minus_one") => qpls_core::SemCovarianceDenominatorV4::SampleNMinusOne,
        _ => {
            return Err(invalid_cbsem_rmsea(
                "v9 point covariance denominator is invalid",
            ));
        }
    };
    let declared_sample_size = match summary_cell("declared_sample_size")? {
        CanonicalResultCellV2::Missing {
            reason: CanonicalMissingReasonV2::NotEstimated,
            display: None,
        } => None,
        cell => Some(
            exact_bootstrap_usize(cell)
                .ok_or_else(|| invalid_cbsem_rmsea("v9 declared sample size is invalid"))?,
        ),
    };
    Ok(CbsemExactCaseBootstrapBasePointDigestProjectionV1 {
        digest_method: CBSEM_EXACT_CASE_BOOTSTRAP_BASE_POINT_DIGEST_METHOD_V1.into(),
        compiled_moment_schema_version: exact_bootstrap_u32(summary_cell(
            "compiled_moment_schema_version",
        )?)
        .ok_or_else(|| invalid_cbsem_rmsea("v9 compiled-moment schema is invalid"))?,
        moment_input_method_version: exact_bootstrap_text(summary_cell(
            "moment_input_method_version",
        )?)
        .ok_or_else(|| invalid_cbsem_rmsea("v9 moment-input method is invalid"))?
        .into(),
        estimator_method_version: exact_bootstrap_text(summary_cell("estimator_method_version")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 estimator method is invalid"))?
            .into(),
        model_type: exact_bootstrap_text(summary_cell("model_type")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 model type is invalid"))?
            .into(),
        estimator: exact_bootstrap_text(summary_cell("estimator")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 estimator is invalid"))?
            .into(),
        mean_structure: exact_bootstrap_boolean(summary_cell("mean_structure")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 mean-structure flag is invalid"))?,
        input: exact_bootstrap_text(summary_cell("input")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 input is invalid"))?
            .into(),
        converged: exact_bootstrap_boolean(summary_cell("converged")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 convergence flag is invalid"))?,
        iterations: exact_bootstrap_u32(summary_cell("iterations")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 point iterations are invalid"))?,
        objective: exact_bootstrap_number(summary_cell("objective")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 point objective is invalid"))?,
        gradient_norm: exact_bootstrap_number(summary_cell("gradient_norm")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 point gradient norm is invalid"))?,
        sample_size: exact_bootstrap_usize(summary_cell("sample_size")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 point sample size is invalid"))?,
        declared_sample_size,
        omitted_observations: exact_bootstrap_usize(summary_cell("omitted_observations")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 omitted-observation count is invalid"))?,
        covariance_denominator,
        canonical_covariance_sha256: exact_bootstrap_text(summary_cell(
            "canonical_covariance_sha256",
        )?)
        .filter(|value| exact_bootstrap_is_sha256(value))
        .ok_or_else(|| invalid_cbsem_rmsea("v9 canonical covariance digest is invalid"))?
        .into(),
        canonical_observed_means_sha256: observed_means,
        parameters: point_parameters,
        fit_method_version: exact_bootstrap_text(fit_cell("fit_method_version")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 fit method is invalid"))?
            .into(),
        chi_square: exact_bootstrap_number(fit_cell("chi_square")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 chi-square is invalid"))?,
        degrees_of_freedom: exact_bootstrap_i64(fit_cell("degrees_of_freedom")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 degrees of freedom are invalid"))?,
        fit_p_value: exact_bootstrap_optional_number(fit_cell("p_value")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 fit p-value is invalid"))?,
        cfi: exact_bootstrap_optional_number(fit_cell("cfi")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 CFI is invalid"))?,
        tli: exact_bootstrap_optional_number(fit_cell("tli")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 TLI is invalid"))?,
        rmsea: exact_bootstrap_optional_number(fit_cell("rmsea")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 RMSEA is invalid"))?,
        rmsea_interval_method_version: exact_bootstrap_text(fit_cell(
            "rmsea_interval_method_version",
        )?)
        .ok_or_else(|| invalid_cbsem_rmsea("v9 RMSEA interval method is invalid"))?
        .into(),
        rmsea_interval_confidence_level: exact_bootstrap_number(fit_cell(
            "rmsea_interval_confidence_level",
        )?)
        .ok_or_else(|| invalid_cbsem_rmsea("v9 RMSEA interval confidence is invalid"))?,
        rmsea_ci_lower: exact_bootstrap_optional_number(fit_cell("rmsea_ci_lower")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 RMSEA lower bound is invalid"))?,
        rmsea_ci_upper: exact_bootstrap_optional_number(fit_cell("rmsea_ci_upper")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 RMSEA upper bound is invalid"))?,
        srmr: exact_bootstrap_number(fit_cell("srmr")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 SRMR is invalid"))?,
        aic: exact_bootstrap_number(fit_cell("aic")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 AIC is invalid"))?,
        bic: exact_bootstrap_number(fit_cell("bic")?)
            .ok_or_else(|| invalid_cbsem_rmsea("v9 BIC is invalid"))?,
        score_lm: exact_bootstrap_score_lm_projection(document)?,
    })
}

fn exact_bootstrap_type7(sorted: &[f64], probability: f64) -> f64 {
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = position - lower as f64;
        sorted[lower] + fraction * (sorted[upper] - sorted[lower])
    }
}

fn exact_bootstrap_test_tail(tail: CbsemBootstrapTestTail) -> &'static str {
    match tail {
        CbsemBootstrapTestTail::TwoSided => "two_sided",
        CbsemBootstrapTestTail::OneSidedGreater => "one_sided_greater",
        CbsemBootstrapTestTail::OneSidedLess => "one_sided_less",
    }
}

fn exact_bootstrap_hypothesis_unavailable_reason(
    reason: CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1,
) -> &'static str {
    match reason {
        CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::InsufficientUsableReplicates => {
            "insufficient_usable_replicates"
        }
        CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::NonregularVarianceBoundary => {
            "nonregular_variance_boundary"
        }
        CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::ZeroNullOutsideOpenDomain => {
            "zero_null_outside_open_domain"
        }
        CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::UnsupportedParameterFamily => {
            "unsupported_parameter_family"
        }
    }
}

fn validate_exact_bootstrap_hypothesis_table_v1(
    table: &CanonicalResultTableV2,
    expected: &CbsemExactCaseBootstrapHypothesisTestsV1,
) -> Result<(), ProjectArchiveV6Error> {
    if expected.method_version != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_METHOD_VERSION_V1
        || expected.null_hypothesis != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_NULL_HYPOTHESIS_V1
        || expected.statistic != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_STATISTIC_V1
        || expected.tie_policy != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_TIE_POLICY_V1
        || expected.probability_method != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_PROBABILITY_METHOD_V1
        || expected.decision_rule != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_DECISION_RULE_V1
        || expected.null_value.to_bits() != 0.0_f64.to_bits()
        || expected.significance_level.to_bits()
            != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_SIGNIFICANCE_LEVEL_V1.to_bits()
        || expected.parameters.is_empty()
        || table.rows.len() != expected.parameters.len()
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v10 hypothesis method, null, alpha, or row count drifted",
        ));
    }
    let (inference_status, global_reason, global_message, globally_available) =
        match &expected.inference {
            CbsemExactCaseBootstrapHypothesisTestInferenceV1::Available => {
                ("available", None, None, true)
            }
            CbsemExactCaseBootstrapHypothesisTestInferenceV1::Unavailable {
                reason_code,
                message,
            } => (
                "unavailable",
                Some(reason_code.as_str()),
                Some(message.as_str()),
                false,
            ),
        };
    for (ordinal, (row, parameter)) in table.rows.iter().zip(&expected.parameters).enumerate() {
        let cells = &row.cells;
        let common_matches = row.id == format!("bootstrap_hypothesis_{ordinal:04}")
            && exact_bootstrap_text(&cells[0]) == Some(expected.method_version.as_str())
            && exact_bootstrap_text(&cells[1]) == Some(expected.null_hypothesis.as_str())
            && exact_bootstrap_text(&cells[2]) == Some(expected.statistic.as_str())
            && exact_bootstrap_text(&cells[3]) == Some(expected.tie_policy.as_str())
            && exact_bootstrap_text(&cells[4]) == Some(expected.probability_method.as_str())
            && exact_bootstrap_text(&cells[5]) == Some(expected.decision_rule.as_str())
            && exact_bootstrap_text(&cells[6])
                == Some(exact_bootstrap_test_tail(expected.selected_test_tail))
            && exact_bootstrap_number(&cells[7]).map(f64::to_bits)
                == Some(expected.null_value.to_bits())
            && exact_bootstrap_number(&cells[8]).map(f64::to_bits)
                == Some(expected.significance_level.to_bits())
            && exact_bootstrap_u32(&cells[9]) == Some(expected.usable_replicates)
            && exact_bootstrap_text(&cells[10]) == Some(inference_status)
            && exact_bootstrap_text(&cells[13]) == Some(parameter.parameter_id.as_str());
        let global_matches = match (global_reason, global_message) {
            (None, None) => {
                exact_score_lm_missing(&cells[11], CanonicalMissingReasonV2::NotApplicable)
                    && exact_score_lm_missing(&cells[12], CanonicalMissingReasonV2::NotApplicable)
            }
            (Some(reason), Some(message)) => {
                exact_bootstrap_text(&cells[11]) == Some(reason)
                    && exact_bootstrap_text(&cells[12]) == Some(message)
            }
            _ => false,
        };
        if !common_matches || !global_matches {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v10 hypothesis identity, selection, global status, or order drifted",
            ));
        }
        match parameter.outcome {
            CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available {
                point_estimate,
                two_sided_exceedances,
                greater_or_equal_exceedances,
                less_or_equal_exceedances,
                p_value_two_sided,
                p_value_greater,
                p_value_less,
                selected_exceedances,
                selected_p_value,
                reject_null,
            } if globally_available => {
                let expected_numbers = [
                    point_estimate,
                    f64::from(two_sided_exceedances),
                    f64::from(greater_or_equal_exceedances),
                    f64::from(less_or_equal_exceedances),
                    p_value_two_sided,
                    p_value_greater,
                    p_value_less,
                    f64::from(selected_exceedances),
                    selected_p_value,
                ];
                if exact_bootstrap_text(&cells[14]) != Some("available")
                    || cells[15..=23]
                        .iter()
                        .zip(expected_numbers)
                        .any(|(cell, value)| {
                            exact_bootstrap_number(cell).map(f64::to_bits) != Some(value.to_bits())
                        })
                    || exact_bootstrap_boolean(&cells[24]) != Some(reject_null)
                    || !exact_score_lm_missing(&cells[25], CanonicalMissingReasonV2::NotApplicable)
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v10 available hypothesis count, probability, decision, or null contract drifted",
                    ));
                }
            }
            CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Unavailable { reason } => {
                if exact_bootstrap_text(&cells[14]) != Some("unavailable")
                    || !cells[15..=24].iter().all(|cell| {
                        exact_score_lm_missing(cell, CanonicalMissingReasonV2::NotApplicable)
                    })
                    || exact_bootstrap_text(&cells[25])
                        != Some(exact_bootstrap_hypothesis_unavailable_reason(reason))
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v10 unavailable hypothesis outcome or null contract drifted",
                    ));
                }
            }
            CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available { .. } => {
                return Err(invalid_cbsem_rmsea(
                    "globally unavailable CB-SEM v10 hypothesis carries an available outcome",
                ));
            }
        }
    }
    Ok(())
}

fn exact_bootstrap_not_applicable(cell: &CanonicalResultCellV2) -> bool {
    matches!(
        cell,
        CanonicalResultCellV2::Missing {
            reason: CanonicalMissingReasonV2::NotApplicable,
            display: None,
        }
    )
}

fn exact_bootstrap_refit_standard_error_reason(
    value: &str,
) -> Option<CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1> {
    match value {
        "singular_information" => Some(
            CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::SingularInformation,
        ),
        "information_not_positive_definite" => Some(
            CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InformationNotPositiveDefinite,
        ),
        "invalid_information_variance_or_standard_error" => Some(
            CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InvalidInformationVarianceOrStandardError,
        ),
        "derivative_unavailable" => Some(
            CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::DerivativeUnavailable,
        ),
        "numerical_information_failure" => Some(
            CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::NumericalInformationFailure,
        ),
        _ => None,
    }
}

fn validate_recipe_v4_cbsem_studentized_bootstrap_document_v1(
    document: &CanonicalResultDocumentV2,
    original: &CbsemExactCaseBootstrapRefitV1,
    base: &CbsemExactCaseBootstrapResultV1,
) -> Result<(), ProjectArchiveV6Error> {
    let summary =
        exact_bootstrap_table(document, CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID)?;
    let point = exact_bootstrap_table(
        document,
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID,
    )?;
    let intervals = exact_bootstrap_table(
        document,
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID,
    )?;
    let refits = exact_bootstrap_table(
        document,
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID,
    )?;
    if summary.rows.len() != 1 || summary.rows[0].id != "bootstrap_studentized" {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v11 studentized summary row identity drifted",
        ));
    }
    let summary_cell = |id: &str| {
        exact_bootstrap_cell(summary, 0, id).ok_or_else(|| {
            invalid_cbsem_rmsea(format!("CB-SEM v11 studentized summary omits {id}"))
        })
    };
    let summary_text = |id: &str| {
        exact_bootstrap_text(summary_cell(id)?).ok_or_else(|| {
            invalid_cbsem_rmsea(format!("CB-SEM v11 studentized summary {id} is not text"))
        })
    };
    let summary_number = |id: &str| {
        exact_bootstrap_number(summary_cell(id)?).ok_or_else(|| {
            invalid_cbsem_rmsea(format!(
                "CB-SEM v11 studentized summary {id} is not canonical numeric data"
            ))
        })
    };
    let parameter_ids_text = summary_text("parameter_ids_json")?;
    let parameter_ids = serde_json::from_str::<Vec<String>>(parameter_ids_text)
        .ok()
        .filter(|ids| serde_json::to_string(ids).ok().as_deref() == Some(parameter_ids_text))
        .ok_or_else(|| {
            invalid_cbsem_rmsea("CB-SEM v11 studentized parameter IDs are noncanonical")
        })?;
    if parameter_ids != base.parameter_ids
        || summary_text("method_version")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_METHOD_VERSION_V1
        || summary_text("standard_error_method_version")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1
        || summary_text("expected_information_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1
        || summary_text("pivot_method")? != CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_PIVOT_METHOD_V1
        || summary_text("quantile_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_QUANTILE_METHOD_V1
        || summary_text("interval_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_INTERVAL_METHOD_V1
        || summary_text("archive_validation_scope")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_ARCHIVE_VALIDATION_SCOPE_V1
        || summary_number("confidence_level")?.to_bits() != base.confidence_level.to_bits()
        || summary_number("minimum_usable_fraction")?.to_bits()
            != base.minimum_usable_fraction.to_bits()
        || exact_bootstrap_u32(summary_cell("minimum_usable_replicates")?)
            != Some(base.minimum_usable_replicates)
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v11 studentized method, threshold, or base ownership drifted",
        ));
    }

    if point.rows.len() != base.parameter_ids.len() {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v11 point standard-error cardinality drifted",
        ));
    }
    enum PointReceiptKind {
        Available(Vec<CbsemExactCaseBootstrapParameterStandardErrorV1>),
        Unavailable(CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1),
    }
    let mut point_kind = None::<PointReceiptKind>;
    for (index, (row, parameter_id)) in point.rows.iter().zip(&base.parameter_ids).enumerate() {
        if row.id != format!("bootstrap_studentized_point_standard_error_{index:04}")
            || exact_bootstrap_text(&row.cells[0])
                != Some(CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1)
            || exact_bootstrap_text(&row.cells[1]) != Some(parameter_id)
        {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v11 point standard-error identity or order drifted",
            ));
        }
        match exact_bootstrap_text(&row.cells[2]) {
            Some("available") => {
                let standard_error = exact_bootstrap_number(&row.cells[4])
                    .filter(|value| *value > 0.0)
                    .ok_or_else(|| {
                        invalid_cbsem_rmsea("CB-SEM v11 point standard error is invalid")
                    })?;
                if exact_bootstrap_text(&row.cells[3])
                    != Some(CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1)
                    || !exact_bootstrap_not_applicable(&row.cells[5])
                {
                    return Err(invalid_cbsem_rmsea(
                        "available CB-SEM v11 point standard-error null contract drifted",
                    ));
                }
                match &mut point_kind {
                    None => {
                        point_kind = Some(PointReceiptKind::Available(vec![
                            CbsemExactCaseBootstrapParameterStandardErrorV1 {
                                parameter_id: parameter_id.clone(),
                                standard_error,
                            },
                        ]));
                    }
                    Some(PointReceiptKind::Available(parameters)) => {
                        parameters.push(CbsemExactCaseBootstrapParameterStandardErrorV1 {
                            parameter_id: parameter_id.clone(),
                            standard_error,
                        });
                    }
                    Some(PointReceiptKind::Unavailable(_)) => {
                        return Err(invalid_cbsem_rmsea(
                            "CB-SEM v11 point standard-error receipt mixes statuses",
                        ));
                    }
                }
            }
            Some("unavailable") => {
                let reason = exact_bootstrap_text(&row.cells[5])
                    .and_then(exact_bootstrap_refit_standard_error_reason)
                    .ok_or_else(|| {
                        invalid_cbsem_rmsea(
                            "CB-SEM v11 point standard-error unavailable reason is invalid",
                        )
                    })?;
                if !exact_bootstrap_not_applicable(&row.cells[3])
                    || !exact_bootstrap_not_applicable(&row.cells[4])
                {
                    return Err(invalid_cbsem_rmsea(
                        "unavailable CB-SEM v11 point standard-error null contract drifted",
                    ));
                }
                match point_kind.as_ref() {
                    None => point_kind = Some(PointReceiptKind::Unavailable(reason)),
                    Some(PointReceiptKind::Unavailable(prior)) if *prior == reason => {}
                    _ => {
                        return Err(invalid_cbsem_rmsea(
                            "CB-SEM v11 point standard-error receipt mixes outcomes",
                        ));
                    }
                }
            }
            _ => {
                return Err(invalid_cbsem_rmsea(
                    "CB-SEM v11 point standard-error status drifted",
                ));
            }
        }
    }
    let point_standard_errors = CbsemExactCaseBootstrapRefitStandardErrorsV1 {
        method_version: CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1.into(),
        outcome: match point_kind.ok_or_else(|| {
            invalid_cbsem_rmsea("CB-SEM v11 point standard-error receipt is empty")
        })? {
            PointReceiptKind::Available(parameters) => {
                CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Available {
                    information_method:
                        CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1.into(),
                    parameters,
                }
            }
            PointReceiptKind::Unavailable(reason) => {
                CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable { reason }
            }
        },
    };

    if refits.rows.len() != base.successful_refits.len() {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v11 refit standard-error cardinality differs from the base ledger",
        ));
    }
    let mut refit_standard_errors = Vec::with_capacity(refits.rows.len());
    for (row, witness) in refits.rows.iter().zip(&base.successful_refits) {
        if row.id
            != format!(
                "bootstrap_studentized_refit_standard_error_{:05}",
                witness.replicate_index
            )
            || exact_bootstrap_u32(&row.cells[0]) != Some(witness.replicate_index)
        {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v11 refit standard-error order differs from the base ledger",
            ));
        }
        let outcome = match exact_bootstrap_text(&row.cells[1]) {
            Some("available") => {
                let encoded = exact_bootstrap_text(&row.cells[3]).ok_or_else(|| {
                    invalid_cbsem_rmsea("CB-SEM v11 refit standard-error vector is missing")
                })?;
                let values = serde_json::from_str::<Vec<f64>>(encoded)
                    .ok()
                    .filter(|values| serde_json::to_string(values).ok().as_deref() == Some(encoded))
                    .filter(|values| {
                        values.len() == base.parameter_ids.len()
                            && values.iter().all(|value| {
                                value.is_finite()
                                    && *value > 0.0
                                    && value.to_bits() != (-0.0_f64).to_bits()
                            })
                    })
                    .ok_or_else(|| {
                        invalid_cbsem_rmsea(
                            "CB-SEM v11 refit standard-error vector is noncanonical",
                        )
                    })?;
                if exact_bootstrap_text(&row.cells[2])
                    != Some(CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1)
                    || !exact_bootstrap_not_applicable(&row.cells[4])
                {
                    return Err(invalid_cbsem_rmsea(
                        "available CB-SEM v11 refit standard-error null contract drifted",
                    ));
                }
                CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Available {
                    information_method:
                        CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1.into(),
                    parameters: base
                        .parameter_ids
                        .iter()
                        .cloned()
                        .zip(values)
                        .map(|(parameter_id, standard_error)| {
                            CbsemExactCaseBootstrapParameterStandardErrorV1 {
                                parameter_id,
                                standard_error,
                            }
                        })
                        .collect(),
                }
            }
            Some("unavailable") => {
                let reason = exact_bootstrap_text(&row.cells[4])
                    .and_then(exact_bootstrap_refit_standard_error_reason)
                    .ok_or_else(|| {
                        invalid_cbsem_rmsea(
                            "CB-SEM v11 refit standard-error unavailable reason is invalid",
                        )
                    })?;
                if !exact_bootstrap_not_applicable(&row.cells[2])
                    || !exact_bootstrap_not_applicable(&row.cells[3])
                {
                    return Err(invalid_cbsem_rmsea(
                        "unavailable CB-SEM v11 refit standard-error null contract drifted",
                    ));
                }
                CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable { reason }
            }
            _ => {
                return Err(invalid_cbsem_rmsea(
                    "CB-SEM v11 refit standard-error status drifted",
                ));
            }
        };
        refit_standard_errors.push(CbsemExactCaseBootstrapRefitStandardErrorsV1 {
            method_version: CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1
                .into(),
            outcome,
        });
    }

    let expected = recompute_cbsem_exact_case_bootstrap_studentized_sidecar_v1(
        original,
        &point_standard_errors,
        base,
        refit_standard_errors,
    )
    .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
    validate_exact_bootstrap_studentized_tables_v1(summary, point, intervals, refits, &expected)
}

fn validate_exact_bootstrap_studentized_tables_v1(
    summary: &CanonicalResultTableV2,
    point: &CanonicalResultTableV2,
    intervals: &CanonicalResultTableV2,
    refits: &CanonicalResultTableV2,
    expected: &CbsemExactCaseBootstrapStudentizedSidecarV1,
) -> Result<(), ProjectArchiveV6Error> {
    let summary_cell = |id: &str| {
        exact_bootstrap_cell(summary, 0, id).ok_or_else(|| {
            invalid_cbsem_rmsea(format!("CB-SEM v11 studentized summary omits {id}"))
        })
    };
    let expected_parameter_ids = serde_json::to_string(&expected.parameter_ids)
        .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
    if exact_bootstrap_text(summary_cell("method_version")?)
        != Some(expected.method_version.as_str())
        || exact_bootstrap_text(summary_cell("standard_error_method_version")?)
            != Some(expected.standard_error_method_version.as_str())
        || exact_bootstrap_text(summary_cell("expected_information_method")?)
            != Some(expected.expected_information_method.as_str())
        || exact_bootstrap_text(summary_cell("pivot_method")?)
            != Some(expected.pivot_method.as_str())
        || exact_bootstrap_text(summary_cell("quantile_method")?)
            != Some(expected.quantile_method.as_str())
        || exact_bootstrap_text(summary_cell("interval_method")?)
            != Some(expected.interval_method.as_str())
        || exact_bootstrap_text(summary_cell("archive_validation_scope")?)
            != Some(expected.archive_validation_scope.as_str())
        || exact_bootstrap_number(summary_cell("confidence_level")?).map(f64::to_bits)
            != Some(expected.confidence_level.to_bits())
        || exact_bootstrap_number(summary_cell("minimum_usable_fraction")?).map(f64::to_bits)
            != Some(expected.minimum_usable_fraction.to_bits())
        || exact_bootstrap_u32(summary_cell("minimum_usable_replicates")?)
            != Some(expected.minimum_usable_replicates)
        || exact_bootstrap_u32(summary_cell("studentized_usable_replicates")?)
            != Some(expected.studentized_usable_replicates)
        || exact_bootstrap_text(summary_cell("parameter_ids_json")?)
            != Some(expected_parameter_ids.as_str())
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v11 stored studentized summary differs from pure recomputation",
        ));
    }
    match &expected.inference {
        CbsemExactCaseBootstrapStudentizedInferenceV1::Available => {
            if exact_bootstrap_text(summary_cell("inference_status")?) != Some("available")
                || !exact_bootstrap_not_applicable(summary_cell("unavailable_reason_code")?)
                || !exact_bootstrap_not_applicable(summary_cell("unavailable_message")?)
            {
                return Err(invalid_cbsem_rmsea(
                    "available CB-SEM v11 studentized summary differs from pure recomputation",
                ));
            }
        }
        CbsemExactCaseBootstrapStudentizedInferenceV1::Unavailable { reason, message } => {
            let reason = match reason {
                qpls_estimation::CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::PointStandardErrorsUnavailable => {
                    "point_standard_errors_unavailable"
                }
                qpls_estimation::CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::InsufficientStudentizedUsableReplicates => {
                    "insufficient_studentized_usable_replicates"
                }
            };
            if exact_bootstrap_text(summary_cell("inference_status")?) != Some("unavailable")
                || exact_bootstrap_text(summary_cell("unavailable_reason_code")?) != Some(reason)
                || exact_bootstrap_text(summary_cell("unavailable_message")?)
                    != Some(message.as_str())
            {
                return Err(invalid_cbsem_rmsea(
                    "unavailable CB-SEM v11 studentized summary differs from pure recomputation",
                ));
            }
        }
    }

    if intervals.rows.len() != expected.intervals.len() {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v11 stored interval cardinality differs from pure recomputation",
        ));
    }
    for (index, (row, interval)) in intervals.rows.iter().zip(&expected.intervals).enumerate() {
        if row.id != format!("bootstrap_studentized_interval_{index:04}")
            || exact_bootstrap_text(&row.cells[0]) != Some(interval.parameter_id.as_str())
        {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v11 stored interval order differs from pure recomputation",
            ));
        }
        match interval.outcome {
            CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1::Available {
                point_estimate,
                point_standard_error,
                lower_pivot_quantile,
                upper_pivot_quantile,
                interval_lower,
                interval_upper,
                usable_replicates,
            } => {
                let expected_numbers = [
                    point_estimate,
                    point_standard_error,
                    lower_pivot_quantile,
                    upper_pivot_quantile,
                    interval_lower,
                    interval_upper,
                ];
                if exact_bootstrap_text(&row.cells[1]) != Some("available")
                    || row.cells[2..=7]
                        .iter()
                        .zip(expected_numbers)
                        .any(|(cell, expected)| {
                            exact_bootstrap_number(cell).map(f64::to_bits)
                                != Some(expected.to_bits())
                        })
                    || exact_bootstrap_u32(&row.cells[8]) != Some(usable_replicates)
                    || !exact_bootstrap_not_applicable(&row.cells[9])
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v11 stored pivot or interval arithmetic differs bit-exactly from pure recomputation",
                    ));
                }
            }
            CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1::Unavailable {
                reason,
            } => {
                let expected_reason = match reason {
                    qpls_estimation::CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::PointStandardErrorsUnavailable => {
                        "point_standard_errors_unavailable"
                    }
                    qpls_estimation::CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::InsufficientStudentizedUsableReplicates => {
                        "insufficient_studentized_usable_replicates"
                    }
                };
                if exact_bootstrap_text(&row.cells[1]) != Some("unavailable")
                    || row.cells[2..=8]
                        .iter()
                        .any(|cell| !exact_bootstrap_not_applicable(cell))
                    || exact_bootstrap_text(&row.cells[9]) != Some(expected_reason)
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v11 stored unavailable interval differs from pure recomputation",
                    ));
                }
            }
        }
    }

    if refits.rows.len() != expected.refit_standard_errors.len() {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v11 stored refit standard-error cardinality differs from pure recomputation",
        ));
    }
    for (row, expected_receipt) in refits.rows.iter().zip(&expected.refit_standard_errors) {
        if exact_bootstrap_u32(&row.cells[0]) != Some(expected_receipt.replicate_index) {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v11 stored refit standard-error index differs from pure recomputation",
            ));
        }
        match &expected_receipt.outcome {
            CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1::Available {
                information_method,
                standard_errors,
            } => {
                let encoded = serde_json::to_string(standard_errors)
                    .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
                if exact_bootstrap_text(&row.cells[1]) != Some("available")
                    || exact_bootstrap_text(&row.cells[2]) != Some(information_method.as_str())
                    || exact_bootstrap_text(&row.cells[3]) != Some(encoded.as_str())
                    || !exact_bootstrap_not_applicable(&row.cells[4])
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v11 stored refit standard errors differ from pure recomputation",
                    ));
                }
            }
            CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1::Unavailable {
                reason,
            } => {
                let expected_reason = match reason {
                    CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::SingularInformation => "singular_information",
                    CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InformationNotPositiveDefinite => "information_not_positive_definite",
                    CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InvalidInformationVarianceOrStandardError => "invalid_information_variance_or_standard_error",
                    CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::DerivativeUnavailable => "derivative_unavailable",
                    CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::NumericalInformationFailure => "numerical_information_failure",
                };
                if exact_bootstrap_text(&row.cells[1]) != Some("unavailable")
                    || !exact_bootstrap_not_applicable(&row.cells[2])
                    || !exact_bootstrap_not_applicable(&row.cells[3])
                    || exact_bootstrap_text(&row.cells[4]) != Some(expected_reason)
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v11 stored unavailable refit standard error differs from pure recomputation",
                    ));
                }
            }
        }
    }

    // The point table is the persisted input to pure recomputation. Its exact
    // row/order/null contract was reconstructed above; retaining the argument
    // here makes the four-table atomic ownership explicit.
    if point.rows.len() != expected.parameter_ids.len() {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v11 stored point standard-error cardinality drifted",
        ));
    }
    Ok(())
}

fn exact_bootstrap_bca_unavailable_reason_text_v1(
    reason: CbsemExactCaseBootstrapBcaUnavailableReasonV1,
) -> &'static str {
    match reason {
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::BaseInferenceUnavailable => {
            "base_inference_unavailable"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::IncompleteDeleteOneLedger => {
            "incomplete_delete_one_ledger"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::BiasCorrectionProbabilityAtBoundary => {
            "bias_correction_probability_at_boundary"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::DegenerateJackknifeAcceleration => {
            "degenerate_jackknife_acceleration"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::NonfiniteJackknifeArithmetic => {
            "nonfinite_jackknife_arithmetic"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::SingularAccelerationAdjustment => {
            "singular_acceleration_adjustment"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::InvalidAdjustedProbability => {
            "invalid_adjusted_probability"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::AdjustedProbabilityOrderInvalid => {
            "adjusted_probability_order_invalid"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::NonfiniteOrReversedInterval => {
            "nonfinite_or_reversed_interval"
        }
    }
}

fn validate_recipe_v4_cbsem_bca_bootstrap_document_v1(
    document: &CanonicalResultDocumentV2,
    original: &CbsemExactCaseBootstrapRefitV1,
    base: &CbsemExactCaseBootstrapResultV1,
) -> Result<(), ProjectArchiveV6Error> {
    let summary = exact_bootstrap_table(document, CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID)?;
    let intervals = exact_bootstrap_table(document, CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID)?;
    let refits = exact_bootstrap_table(document, CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID)?;
    let failures = exact_bootstrap_table(document, CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID)?;
    if summary.rows.len() != 1 || summary.rows[0].id != "bootstrap_bca" {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v12 BCa summary row identity drifted",
        ));
    }
    let summary_cell = |id: &str| {
        exact_bootstrap_cell(summary, 0, id)
            .ok_or_else(|| invalid_cbsem_rmsea(format!("CB-SEM v12 BCa summary omits {id}")))
    };
    let summary_text = |id: &str| {
        exact_bootstrap_text(summary_cell(id)?)
            .ok_or_else(|| invalid_cbsem_rmsea(format!("CB-SEM v12 BCa summary {id} is not text")))
    };
    let summary_number = |id: &str| {
        exact_bootstrap_number(summary_cell(id)?).ok_or_else(|| {
            invalid_cbsem_rmsea(format!(
                "CB-SEM v12 BCa summary {id} is not canonical numeric data"
            ))
        })
    };
    let parameter_ids_text = summary_text("parameter_ids_json")?;
    let parameter_ids = serde_json::from_str::<Vec<String>>(parameter_ids_text)
        .ok()
        .filter(|ids| serde_json::to_string(ids).ok().as_deref() == Some(parameter_ids_text))
        .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v12 BCa parameter IDs are noncanonical"))?;
    let delete_one_case_count = exact_bootstrap_usize(summary_cell("delete_one_case_count")?)
        .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v12 delete-one case count is invalid"))?;
    let successful_count = exact_bootstrap_usize(summary_cell("successful_delete_one_refits")?)
        .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v12 successful delete-one count is invalid"))?;
    let failed_count = exact_bootstrap_usize(summary_cell("failed_delete_one_refits")?)
        .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v12 failed delete-one count is invalid"))?;
    if parameter_ids != base.parameter_ids
        || delete_one_case_count != base.complete_case_sample_size
        || successful_count != refits.rows.len()
        || failed_count != failures.rows.len()
        || successful_count + failed_count != delete_one_case_count
        || summary_text("method_version")? != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_METHOD_VERSION_V1
        || summary_text("base_bootstrap_method_version")? != base.method_version
        || summary_text("outer_recipe_analytical_identity_sha256")?
            != base.outer_recipe_analytical_identity_sha256
        || summary_text("base_point_result_sha256")? != base.base_point_result_sha256
        || summary_text("compiler_analytical_identity_sha256")?
            != base.compiler_analytical_identity_sha256
        || summary_text("plan_sha256")? != base.plan_sha256
        || summary_text("model_scientific_sha256")? != base.model_scientific_sha256
        || summary_text("delete_one_refit_method_version")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1
        || summary_text("delete_one_sampling_positions_digest_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1
        || summary_text("delete_one_sample_indices_digest_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1
        || summary_text("bias_correction_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_BIAS_CORRECTION_METHOD_V1
        || summary_text("acceleration_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ACCELERATION_METHOD_V2
        || summary_text("adjusted_probability_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ADJUSTMENT_METHOD_V2
        || summary_text("quantile_method")? != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_QUANTILE_METHOD_V1
        || summary_text("retry_policy")? != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_RETRY_POLICY_V1
        || summary_text("archive_validation_scope")? != CBSEM_EXACT_BOOTSTRAP_BCA_ARCHIVE_SCOPE
        || summary_number("confidence_level")?.to_bits() != base.confidence_level.to_bits()
        || exact_bootstrap_u32(summary_cell("bootstrap_usable_replicates")?)
            != Some(base.usable_replicates)
        || exact_bootstrap_u32(summary_cell("minimum_bootstrap_usable_replicates")?)
            != Some(base.minimum_usable_replicates)
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v12 BCa constants, base authority, or accounting drifted",
        ));
    }

    let mut successful = Vec::with_capacity(refits.rows.len());
    for (row_index, row) in refits.rows.iter().enumerate() {
        let position = exact_bootstrap_usize(&row.cells[0]).ok_or_else(|| {
            invalid_cbsem_rmsea("CB-SEM v12 successful delete-one position is invalid")
        })?;
        let source_row = exact_bootstrap_usize(&row.cells[1]).ok_or_else(|| {
            invalid_cbsem_rmsea("CB-SEM v12 successful delete-one source row is invalid")
        })?;
        let positions_sha256 = exact_bootstrap_text(&row.cells[2])
            .filter(|value| exact_bootstrap_is_sha256(value))
            .ok_or_else(|| {
                invalid_cbsem_rmsea("CB-SEM v12 successful delete-one position digest is invalid")
            })?;
        let indices_sha256 = exact_bootstrap_text(&row.cells[3])
            .filter(|value| exact_bootstrap_is_sha256(value))
            .ok_or_else(|| {
                invalid_cbsem_rmsea("CB-SEM v12 successful delete-one source digest is invalid")
            })?;
        let estimates_text = exact_bootstrap_text(&row.cells[4]).ok_or_else(|| {
            invalid_cbsem_rmsea("CB-SEM v12 successful delete-one estimates are missing")
        })?;
        let estimates = serde_json::from_str::<Vec<f64>>(estimates_text)
            .ok()
            .filter(|values| serde_json::to_string(values).ok().as_deref() == Some(estimates_text))
            .filter(|values| {
                values.len() == base.parameter_ids.len()
                    && values.iter().all(|value| value.is_finite())
            })
            .ok_or_else(|| {
                invalid_cbsem_rmsea("CB-SEM v12 successful delete-one estimates are noncanonical")
            })?;
        let iterations = exact_bootstrap_u32(&row.cells[5])
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                invalid_cbsem_rmsea("CB-SEM v12 successful delete-one iterations are invalid")
            })?;
        let objective = exact_bootstrap_number(&row.cells[6])
            .filter(|value| *value >= 0.0)
            .ok_or_else(|| {
                invalid_cbsem_rmsea("CB-SEM v12 successful delete-one objective is invalid")
            })?;
        let gradient_norm = exact_bootstrap_number(&row.cells[7])
            .filter(|value| *value >= 0.0)
            .ok_or_else(|| {
                invalid_cbsem_rmsea("CB-SEM v12 successful delete-one gradient is invalid")
            })?;
        if row.id != format!("bootstrap_bca_delete_one_refit_{position:05}")
            || successful
                .last()
                .is_some_and(|prior: &CbsemExactCaseBootstrapDeleteOneWitnessV1| {
                    prior.omitted_complete_case_position >= position
                })
            || row_index >= delete_one_case_count
        {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v12 successful delete-one identity or order drifted",
            ));
        }
        successful.push(CbsemExactCaseBootstrapDeleteOneWitnessV1 {
            omitted_complete_case_position: position,
            omitted_source_row_index: source_row,
            retained_sampling_positions_sha256: positions_sha256.into(),
            retained_sample_indices_sha256: indices_sha256.into(),
            parameter_estimates: estimates,
            iterations,
            objective,
            gradient_norm,
        });
    }

    let mut failed = Vec::with_capacity(failures.rows.len());
    for row in &failures.rows {
        let position = exact_bootstrap_usize(&row.cells[0]).ok_or_else(|| {
            invalid_cbsem_rmsea("CB-SEM v12 failed delete-one position is invalid")
        })?;
        let source_row = exact_bootstrap_usize(&row.cells[1]).ok_or_else(|| {
            invalid_cbsem_rmsea("CB-SEM v12 failed delete-one source row is invalid")
        })?;
        let positions_sha256 = exact_bootstrap_text(&row.cells[2])
            .filter(|value| exact_bootstrap_is_sha256(value))
            .ok_or_else(|| {
                invalid_cbsem_rmsea("CB-SEM v12 failed delete-one position digest is invalid")
            })?;
        let indices_sha256 = exact_bootstrap_text(&row.cells[3])
            .filter(|value| exact_bootstrap_is_sha256(value))
            .ok_or_else(|| {
                invalid_cbsem_rmsea("CB-SEM v12 failed delete-one source digest is invalid")
            })?;
        let kind = match exact_bootstrap_text(&row.cells[4]) {
            Some("moment_matrix_not_positive_definite") => {
                CbsemExactCaseBootstrapFailureKindV1::MomentMatrixNotPositiveDefinite
            }
            Some("non_convergence") => CbsemExactCaseBootstrapFailureKindV1::NonConvergence,
            Some("inadmissible_solution") => {
                CbsemExactCaseBootstrapFailureKindV1::InadmissibleSolution
            }
            Some("numerical_failure") => CbsemExactCaseBootstrapFailureKindV1::NumericalFailure,
            _ => return Err(invalid_cbsem_rmsea("CB-SEM v12 failure kind is invalid")),
        };
        let message = exact_bootstrap_text(&row.cells[5])
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v12 failure message is invalid"))?;
        if row.id != format!("bootstrap_bca_delete_one_failure_{position:05}")
            || failed
                .last()
                .is_some_and(|prior: &CbsemExactCaseBootstrapDeleteOneFailureV1| {
                    prior.omitted_complete_case_position >= position
                })
        {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v12 failed delete-one identity or order drifted",
            ));
        }
        failed.push(CbsemExactCaseBootstrapDeleteOneFailureV1 {
            omitted_complete_case_position: position,
            omitted_source_row_index: source_row,
            retained_sampling_positions_sha256: positions_sha256.into(),
            retained_sample_indices_sha256: indices_sha256.into(),
            kind,
            message: message.into(),
        });
    }

    let mut sampling_frame = vec![None; delete_one_case_count];
    for (position, source_row) in successful
        .iter()
        .map(|row| {
            (
                row.omitted_complete_case_position,
                row.omitted_source_row_index,
            )
        })
        .chain(failed.iter().map(|row| {
            (
                row.omitted_complete_case_position,
                row.omitted_source_row_index,
            )
        }))
    {
        if position >= sampling_frame.len()
            || source_row >= original.source_row_count
            || sampling_frame[position].replace(source_row).is_some()
        {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v12 delete-one omission partition is invalid",
            ));
        }
    }
    let sampling_frame = sampling_frame
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            invalid_cbsem_rmsea("CB-SEM v12 delete-one omission partition is incomplete")
        })?;
    let identity_positions = (0..delete_one_case_count).collect::<Vec<_>>();
    let mut replay_original = original.clone();
    replay_original.sampling_positions_digest_method =
        CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1.into();
    replay_original.sampling_positions_sha256 =
        cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
            delete_one_case_count,
            &identity_positions,
        );
    replay_original.sample_indices_sha256 = cbsem_exact_case_bootstrap_index_digest_v1(
        &original.source_dataset_fingerprint,
        original.source_row_count,
        &sampling_frame,
    );
    let expected = recompute_cbsem_exact_case_bootstrap_bca_sidecar_v1(
        &replay_original,
        base,
        successful,
        failed,
    )
    .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
    validate_exact_bootstrap_bca_tables_v1(summary, intervals, &expected)
}

fn validate_exact_bootstrap_bca_tables_v1(
    summary: &CanonicalResultTableV2,
    intervals: &CanonicalResultTableV2,
    expected: &CbsemExactCaseBootstrapBcaSidecarV1,
) -> Result<(), ProjectArchiveV6Error> {
    let summary_cell = |id: &str| {
        exact_bootstrap_cell(summary, 0, id)
            .ok_or_else(|| invalid_cbsem_rmsea(format!("CB-SEM v12 BCa summary omits {id}")))
    };
    let expected_parameter_ids = serde_json::to_string(&expected.parameter_ids)
        .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
    if exact_bootstrap_text(summary_cell("method_version")?)
        != Some(expected.method_version.as_str())
        || exact_bootstrap_text(summary_cell("base_bootstrap_method_version")?)
            != Some(expected.base_bootstrap_method_version.as_str())
        || exact_bootstrap_text(summary_cell("outer_recipe_analytical_identity_sha256")?)
            != Some(expected.outer_recipe_analytical_identity_sha256.as_str())
        || exact_bootstrap_text(summary_cell("base_point_result_sha256")?)
            != Some(expected.base_point_result_sha256.as_str())
        || exact_bootstrap_text(summary_cell("compiler_analytical_identity_sha256")?)
            != Some(expected.compiler_analytical_identity_sha256.as_str())
        || exact_bootstrap_text(summary_cell("plan_sha256")?) != Some(expected.plan_sha256.as_str())
        || exact_bootstrap_text(summary_cell("model_scientific_sha256")?)
            != Some(expected.model_scientific_sha256.as_str())
        || exact_bootstrap_text(summary_cell("delete_one_refit_method_version")?)
            != Some(expected.delete_one_refit_method_version.as_str())
        || exact_bootstrap_text(summary_cell("bias_correction_method")?)
            != Some(expected.bias_correction_method.as_str())
        || exact_bootstrap_text(summary_cell("acceleration_method")?)
            != Some(expected.acceleration_method.as_str())
        || exact_bootstrap_text(summary_cell("adjusted_probability_method")?)
            != Some(expected.adjusted_probability_method.as_str())
        || exact_bootstrap_text(summary_cell("quantile_method")?)
            != Some(expected.quantile_method.as_str())
        || exact_bootstrap_text(summary_cell("retry_policy")?)
            != Some(expected.retry_policy.as_str())
        || exact_bootstrap_number(summary_cell("confidence_level")?).map(f64::to_bits)
            != Some(expected.confidence_level.to_bits())
        || exact_bootstrap_u32(summary_cell("bootstrap_usable_replicates")?)
            != Some(expected.bootstrap_usable_replicates)
        || exact_bootstrap_u32(summary_cell("minimum_bootstrap_usable_replicates")?)
            != Some(expected.minimum_bootstrap_usable_replicates)
        || exact_bootstrap_usize(summary_cell("delete_one_case_count")?)
            != Some(expected.delete_one_case_count)
        || exact_bootstrap_usize(summary_cell("successful_delete_one_refits")?)
            != Some(expected.successful_delete_one_refits.len())
        || exact_bootstrap_usize(summary_cell("failed_delete_one_refits")?)
            != Some(expected.failed_delete_one_refits.len())
        || exact_bootstrap_text(summary_cell("parameter_ids_json")?)
            != Some(expected_parameter_ids.as_str())
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v12 stored BCa summary differs from pure recomputation",
        ));
    }
    match &expected.inference {
        CbsemExactCaseBootstrapBcaInferenceV1::Available => {
            if exact_bootstrap_text(summary_cell("inference_status")?) != Some("available")
                || !exact_bootstrap_not_applicable(summary_cell("unavailable_reason_code")?)
                || !exact_bootstrap_not_applicable(summary_cell("unavailable_message")?)
            {
                return Err(invalid_cbsem_rmsea(
                    "available CB-SEM v12 BCa summary differs from pure recomputation",
                ));
            }
        }
        CbsemExactCaseBootstrapBcaInferenceV1::Unavailable { reason, message } => {
            if exact_bootstrap_text(summary_cell("inference_status")?) != Some("unavailable")
                || exact_bootstrap_text(summary_cell("unavailable_reason_code")?)
                    != Some(exact_bootstrap_bca_unavailable_reason_text_v1(*reason))
                || exact_bootstrap_text(summary_cell("unavailable_message")?)
                    != Some(message.as_str())
            {
                return Err(invalid_cbsem_rmsea(
                    "unavailable CB-SEM v12 BCa summary differs from pure recomputation",
                ));
            }
        }
    }
    if intervals.rows.len() != expected.intervals.len() {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v12 stored BCa interval cardinality differs from pure recomputation",
        ));
    }
    for (index, (row, interval)) in intervals.rows.iter().zip(&expected.intervals).enumerate() {
        if row.id != format!("bootstrap_bca_interval_{index:04}")
            || exact_bootstrap_text(&row.cells[0]) != Some(interval.parameter_id.as_str())
        {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v12 stored BCa interval identity or order drifted",
            ));
        }
        match interval.outcome {
            CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Available {
                point_estimate,
                bias_correction,
                acceleration,
                adjusted_lower_probability,
                adjusted_upper_probability,
                interval_lower,
                interval_upper,
                usable_replicates,
            } => {
                let expected_numbers = [
                    point_estimate,
                    bias_correction,
                    acceleration,
                    adjusted_lower_probability,
                    adjusted_upper_probability,
                    interval_lower,
                    interval_upper,
                ];
                if exact_bootstrap_text(&row.cells[1]) != Some("available")
                    || row.cells[2..=8]
                        .iter()
                        .zip(expected_numbers)
                        .any(|(cell, expected)| {
                            exact_bootstrap_number(cell).map(f64::to_bits)
                                != Some(expected.to_bits())
                        })
                    || exact_bootstrap_u32(&row.cells[9]) != Some(usable_replicates)
                    || !exact_bootstrap_not_applicable(&row.cells[10])
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v12 stored BCa arithmetic differs bit-exactly from pure recomputation",
                    ));
                }
            }
            CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Unavailable { reason } => {
                if exact_bootstrap_text(&row.cells[1]) != Some("unavailable")
                    || row.cells[2..=9]
                        .iter()
                        .any(|cell| !exact_bootstrap_not_applicable(cell))
                    || exact_bootstrap_text(&row.cells[10])
                        != Some(exact_bootstrap_bca_unavailable_reason_text_v1(reason))
                {
                    return Err(invalid_cbsem_rmsea(
                        "CB-SEM v12 stored unavailable BCa interval differs from pure recomputation",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_cbsem_studentized_archive_workload_v1(
    requested_replicates: u32,
    workers: usize,
    complete_case_sample_size: usize,
    modeled_variable_count: usize,
    free_parameter_row_count: usize,
    optimizer_dimension_count: usize,
) -> Result<(), ProjectArchiveV6Error> {
    if !(500..=10_000).contains(&requested_replicates)
        || !(1..=12).contains(&workers)
        || !(1..=180).contains(&complete_case_sample_size)
        || !(1..=9).contains(&modeled_variable_count)
        || !(1..=18).contains(&free_parameter_row_count)
        || !(1..=18).contains(&optimizer_dimension_count)
    {
        return Err(invalid_cbsem_rmsea(format!(
            "CB-SEM v11/v12 exceeds the fail-closed Labs workload envelope B=500..10000, W=1..12, N=1..180, V=1..9, P=1..18, D=1..18 (actual B={requested_replicates}, W={workers}, N={complete_case_sample_size}, V={modeled_variable_count}, P={free_parameter_row_count}, D={optimizer_dimension_count})"
        )));
    }
    Ok(())
}

fn validate_recipe_v4_cbsem_exact_bootstrap_document_v1(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    dataset: &DatasetDescriptor,
) -> Result<(), ProjectArchiveV6Error> {
    let table_ids = [
        CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID,
    ];
    let present_tables = document
        .tables
        .iter()
        .filter(|table| table_ids.contains(&table.id.as_str()))
        .count();
    let sections = document
        .sections
        .iter()
        .filter(|section| section.id == CBSEM_EXACT_BOOTSTRAP_SECTION_ID)
        .collect::<Vec<_>>();
    let is_v9 =
        document.provenance.engine_version == CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_HISTORICAL;
    let is_v10 =
        document.provenance.engine_version == CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_CURRENT;
    let is_v11 = document.provenance.engine_version
        == CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_STUDENTIZED_CURRENT;
    let is_v12 =
        document.provenance.engine_version == CBSEM_EXACT_ADAPTER_LISTWISE_BOOTSTRAP_BCA_CURRENT;
    let hypothesis_tables = document
        .tables
        .iter()
        .filter(|table| table.id == CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID)
        .collect::<Vec<_>>();
    let hypothesis_sections = document
        .sections
        .iter()
        .filter(|section| section.id == CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID)
        .collect::<Vec<_>>();
    let studentized_table_ids = [
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID,
    ];
    let studentized_tables = document
        .tables
        .iter()
        .filter(|table| studentized_table_ids.contains(&table.id.as_str()))
        .collect::<Vec<_>>();
    let studentized_sections = document
        .sections
        .iter()
        .filter(|section| section.id == CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID)
        .collect::<Vec<_>>();
    let bca_table_ids = [
        CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID,
    ];
    let bca_tables = document
        .tables
        .iter()
        .filter(|table| bca_table_ids.contains(&table.id.as_str()))
        .collect::<Vec<_>>();
    let bca_sections = document
        .sections
        .iter()
        .filter(|section| section.id == CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID)
        .collect::<Vec<_>>();
    if !is_v9 && !is_v10 && !is_v11 && !is_v12 {
        if present_tables == 0
            && sections.is_empty()
            && hypothesis_tables.is_empty()
            && hypothesis_sections.is_empty()
            && studentized_tables.is_empty()
            && studentized_sections.is_empty()
            && bca_tables.is_empty()
            && bca_sections.is_empty()
        {
            return Ok(());
        }
        return Err(invalid_cbsem_rmsea(
            "non-bootstrap CB-SEM adapters cannot carry exact case-bootstrap artifacts",
        ));
    }
    let [section] = sections.as_slice() else {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v9/v10/v11/v12 requires exactly one bootstrap-inference section",
        ));
    };
    if present_tables != table_ids.len()
        || section.table_ids.iter().map(String::as_str).ne(table_ids)
        || !section.chart_ids.is_empty()
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v9/v10/v11/v12 bootstrap table membership or order drifted",
        ));
    }
    if is_v9
        && (!hypothesis_tables.is_empty()
            || !hypothesis_sections.is_empty()
            || !studentized_tables.is_empty()
            || !studentized_sections.is_empty()
            || !bca_tables.is_empty()
            || !bca_sections.is_empty())
    {
        return Err(invalid_cbsem_rmsea(
            "historical CB-SEM v9 rejects injected bootstrap hypothesis artifacts",
        ));
    }
    if is_v10
        && (!studentized_tables.is_empty()
            || !studentized_sections.is_empty()
            || !bca_tables.is_empty()
            || !bca_sections.is_empty())
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v10 rejects injected studentized or BCa bootstrap artifacts",
        ));
    }
    let hypothesis_table = if is_v10 || is_v11 || is_v12 {
        let [table] = hypothesis_tables.as_slice() else {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v10/v11/v12 requires exactly one bootstrap hypothesis table",
            ));
        };
        let [section] = hypothesis_sections.as_slice() else {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v10/v11/v12 requires exactly one bootstrap hypothesis section",
            ));
        };
        if section
            .table_ids
            .iter()
            .map(String::as_str)
            .ne([CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID])
            || !section.chart_ids.is_empty()
            || !exact_bootstrap_column_order(table, CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS)
        {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v10/v11/v12 bootstrap hypothesis table membership or columns drifted",
            ));
        }
        Some(*table)
    } else {
        None
    };
    if is_v11 {
        let [section] = studentized_sections.as_slice() else {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v11 requires exactly one studentized-inference section",
            ));
        };
        if studentized_tables.len() != studentized_table_ids.len()
            || section
                .table_ids
                .iter()
                .map(String::as_str)
                .ne(studentized_table_ids)
            || !section.chart_ids.is_empty()
        {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v11 studentized table membership or order drifted",
            ));
        }
        for (id, columns) in [
            (
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERROR_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERROR_COLUMNS,
            ),
        ] {
            let table = exact_bootstrap_table(document, id)?;
            if !exact_bootstrap_column_order(table, columns) {
                return Err(invalid_cbsem_rmsea(format!(
                    "CB-SEM v11 {id} column order or row width drifted"
                )));
            }
        }
    } else if !studentized_tables.is_empty() || !studentized_sections.is_empty() {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v2-v10/v12 cannot carry studentized bootstrap artifacts",
        ));
    }
    if is_v12 {
        let [section] = bca_sections.as_slice() else {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v12 requires exactly one BCa-inference section",
            ));
        };
        if bca_tables.len() != bca_table_ids.len()
            || section
                .table_ids
                .iter()
                .map(String::as_str)
                .ne(bca_table_ids)
            || !section.chart_ids.is_empty()
        {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v12 BCa table membership or order drifted",
            ));
        }
        for (id, columns) in [
            (
                CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_BCA_REFIT_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_BCA_FAILURE_COLUMNS,
            ),
        ] {
            let table = exact_bootstrap_table(document, id)?;
            if !exact_bootstrap_column_order(table, columns) {
                return Err(invalid_cbsem_rmsea(format!(
                    "CB-SEM v12 {id} column order or row width drifted"
                )));
            }
        }
    } else if !bca_tables.is_empty() || !bca_sections.is_empty() {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v2-v11 cannot carry BCa bootstrap artifacts",
        ));
    }
    let summary = exact_bootstrap_table(document, CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID)?;
    let intervals = exact_bootstrap_table(document, CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID)?;
    let refits = exact_bootstrap_table(document, CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID)?;
    let failures = exact_bootstrap_table(document, CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID)?;
    for (table, expected) in [
        (summary, CBSEM_EXACT_BOOTSTRAP_SUMMARY_COLUMNS),
        (intervals, CBSEM_EXACT_BOOTSTRAP_INTERVAL_COLUMNS),
        (refits, CBSEM_EXACT_BOOTSTRAP_REFIT_COLUMNS),
        (failures, CBSEM_EXACT_BOOTSTRAP_FAILURE_COLUMNS),
    ] {
        if !exact_bootstrap_column_order(table, expected) {
            return Err(invalid_cbsem_rmsea(format!(
                "CB-SEM v9 {} column order or row width drifted",
                table.id
            )));
        }
    }
    if summary.rows.len() != 1 || summary.rows[0].id != "bootstrap" {
        return Err(invalid_cbsem_rmsea("CB-SEM v9 summary row drifted"));
    }
    let summary_cell = |id: &str| {
        exact_bootstrap_cell(summary, 0, id)
            .ok_or_else(|| invalid_cbsem_rmsea(format!("CB-SEM v9 summary omits {id}")))
    };
    let summary_text = |id: &str| {
        exact_bootstrap_text(summary_cell(id)?)
            .ok_or_else(|| invalid_cbsem_rmsea(format!("CB-SEM v9 summary {id} is not text")))
    };
    let summary_number = |id: &str| {
        exact_bootstrap_number(summary_cell(id)?).ok_or_else(|| {
            invalid_cbsem_rmsea(format!(
                "CB-SEM v9 summary {id} is not canonical numeric data"
            ))
        })
    };
    let requested = exact_bootstrap_u32(summary_cell("requested_replicates")?)
        .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v9 requested count is invalid"))?;
    let attempted = exact_bootstrap_u32(summary_cell("attempted_refits")?)
        .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v9 attempted count is invalid"))?;
    let usable = exact_bootstrap_u32(summary_cell("usable_replicates")?)
        .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v9 usable count is invalid"))?;
    let failed = exact_bootstrap_u32(summary_cell("failed_replicates")?)
        .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v9 failure count is invalid"))?;
    let complete_n = exact_bootstrap_usize(summary_cell("complete_case_sample_size")?)
        .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v9 complete-case size is invalid"))?;
    let seed_text = summary_text("seed_decimal")?;
    let seed = seed_text
        .parse::<u64>()
        .ok()
        .filter(|seed| seed.to_string() == seed_text)
        .ok_or_else(|| invalid_cbsem_rmsea("CB-SEM v9 seed decimal is noncanonical"))?;
    let parameter_ids_text = summary_text("parameter_ids_json")?;
    let parameter_ids = serde_json::from_str::<Vec<String>>(parameter_ids_text)
        .ok()
        .filter(|ids| serde_json::to_string(ids).ok().as_deref() == Some(parameter_ids_text))
        .ok_or_else(|| {
            invalid_cbsem_rmsea("CB-SEM v9 parameter IDs are not compact canonical JSON")
        })?;
    if parameter_ids.is_empty()
        || parameter_ids.iter().any(|id| id.trim().is_empty())
        || parameter_ids.iter().collect::<BTreeSet<_>>().len() != parameter_ids.len()
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v9 parameter IDs are empty or duplicated",
        ));
    }
    let Some(MethodConfig::Cbsem {
        model_type: CbsemModelType::Cfa,
        estimator: CbsemEstimator::Ml,
        input: CbsemInput::Raw,
        mean_structure: false,
        bootstrap_samples,
        bootstrap_v2: Some(bootstrap_config),
        group_column: None,
        invariance_steps,
    }) = recipe.method_config.as_ref()
    else {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v9 is not bound to an exact raw single-group CFA bootstrap recipe",
        ));
    };
    if !invariance_steps.is_empty()
        || bootstrap_config.algorithm != CbsemBootstrapAlgorithm::CaseResamplingFullMl
        || (is_v11 && bootstrap_config.interval != CbsemBootstrapInterval::AnalyticStudentizedType7)
        || (is_v12 && bootstrap_config.interval != CbsemBootstrapInterval::BcaType7)
        || (!is_v11
            && !is_v12
            && bootstrap_config.interval != CbsemBootstrapInterval::PercentileType7)
        || (is_v9 && bootstrap_config.test_tail != CbsemBootstrapTestTail::TwoSided)
        || *bootstrap_samples != requested
        || recipe.settings.bootstrap_samples != requested
        || !(CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_REQUESTED_REPLICATES_V1
            ..=CBSEM_EXACT_CASE_BOOTSTRAP_MAXIMUM_REPLICATES_V1)
            .contains(&requested)
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v9 recipe and requested bootstrap plan differ",
        ));
    }
    let target = RecipeV4CompilerTarget::CbsemPlanV2;
    let outer = compile_analysis_recipe_v4(recipe, Some(model), target, target.capability_cell())
        .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
    let mut point_recipe = recipe.clone();
    point_recipe.settings.bootstrap_samples = 0;
    let Some(MethodConfig::Cbsem {
        bootstrap_samples,
        bootstrap_v2,
        ..
    }) = point_recipe.method_config.as_mut()
    else {
        unreachable!("v9 recipe shape was checked above")
    };
    *bootstrap_samples = 0;
    *bootstrap_v2 = None;
    let point =
        compile_analysis_recipe_v4(&point_recipe, Some(model), target, target.capability_cell())
            .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
    let CompiledRecipePlanV4::CbsemPlanV2 { plan: point_plan } = point.plan() else {
        return Err(invalid_cbsem_rmsea(
            "exact-bootstrap point recompilation did not produce a CB-SEM plan",
        ));
    };
    let point_projection = exact_bootstrap_base_point_projection(document)?;
    let base_point_sha256 = cbsem_exact_case_bootstrap_base_point_sha256_v1(&point_projection)
        .map_err(|error| invalid_cbsem_rmsea(error))?;
    let source_dataset_id = summary_text("source_dataset_id")?;
    let source_fingerprint = summary_text("source_dataset_fingerprint")?;
    let outer_recipe_sha256 = summary_text("outer_recipe_analytical_identity_sha256")?;
    let model_sha256 = summary_text("model_scientific_sha256")?;
    let compiler_sha256 = summary_text("compiler_analytical_identity_sha256")?;
    let plan_sha256 = summary_text("plan_sha256")?;
    for value in [
        source_fingerprint,
        outer_recipe_sha256,
        summary_text("base_point_result_sha256")?,
        model_sha256,
        compiler_sha256,
        plan_sha256,
        summary_text("complete_case_universe_sha256")?,
    ] {
        if !exact_bootstrap_is_sha256(value) {
            return Err(invalid_cbsem_rmsea(
                "CB-SEM v9 contains a non-lowercase SHA-256 field",
            ));
        }
    }
    if document.provenance.method_version != CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        || document.provenance.seed != Some(seed)
        || seed != recipe.settings.seed
        || usize::try_from(document.provenance.workers).ok() != Some(recipe.settings.workers)
        || source_dataset_id != document.provenance.dataset_id
        || source_dataset_id != dataset.id.to_string()
        || source_fingerprint != document.provenance.dataset_fingerprint
        || recorded_dataset_sha256(&dataset.fingerprint.0) != Some(source_fingerprint)
        || outer_recipe_sha256 != document.provenance.recipe_digest
        || outer_recipe_sha256 != outer.receipt().recipe_analytical_sha256()
        || model_sha256 != document.provenance.model_digest
        || model_sha256
            != model
                .scientific_sha256()
                .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?
        || compiler_sha256 != point.receipt().analytical_identity_sha256()
        || plan_sha256 != point.receipt().plan_sha256()
        || summary_text("base_point_result_sha256")? != base_point_sha256
        || summary_text("method_version")? != CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1
        || summary_text("estimator_method_version")?
            != CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        || summary_text("complete_case_universe_digest_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1
        || summary_text("covariance_denominator")? != "maximum_likelihood_n"
        || summary_text("sample_indices_digest_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1
        || summary_text("sampling_positions_digest_method")?
            != CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1
        || summary_text("interval_method")? != CBSEM_EXACT_CASE_BOOTSTRAP_INTERVAL_METHOD_V1
        || summary_text("stream_token")? != CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1
        || summary_text("retry_policy")? != CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1
        || summary_text("archive_validation_scope")? != CBSEM_EXACT_BOOTSTRAP_ARCHIVE_SCOPE
        || summary_number("confidence_level")?.to_bits()
            != CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1.to_bits()
        || summary_number("minimum_usable_fraction")?.to_bits()
            != CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V1.to_bits()
        || exact_bootstrap_u32(summary_cell("minimum_usable_replicates")?)
            != Some(required_usable_refits(requested))
        || exact_bootstrap_u32(summary_cell("max_attempts_per_replicate")?)
            != Some(u32::from(
                CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
            ))
        || attempted != requested
        || usable != refits.rows.len() as u32
        || failed != failures.rows.len() as u32
        || usable + failed != requested
        || complete_n != point_projection.sample_size
        || complete_n < 10
        || complete_n > dataset.schema.case_count
        || point_projection.omitted_observations != dataset.schema.case_count - complete_n
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v9 method, recipe/model/dataset, point, plan, count, or provenance binding drifted",
        ));
    }
    let mut free_point_parameters = point_projection
        .parameters
        .iter()
        .filter(|parameter| !parameter.fixed)
        .collect::<Vec<_>>();
    free_point_parameters.sort_by(|left, right| left.parameter_id.cmp(&right.parameter_id));
    if free_point_parameters
        .iter()
        .map(|parameter| parameter.parameter_id.as_str())
        .ne(parameter_ids.iter().map(String::as_str))
    {
        return Err(invalid_cbsem_rmsea(
            "CB-SEM v9 bootstrap parameter order differs from the base point",
        ));
    }
    if is_v11 || is_v12 {
        let mut optimizer_dimensions = BTreeSet::new();
        for parameter in point_plan.parameters() {
            if let CompiledCbsemParameterStatusV2::Free { equality_label, .. } =
                parameter.specification()
            {
                optimizer_dimensions.insert(match equality_label {
                    Some(label) => format!("equality:{}", label.trim()),
                    None => format!("parameter:{}", parameter.id()),
                });
            }
        }
        let modeled_variable_count = point_plan.observed_variables().len();
        let free_parameter_row_count = free_point_parameters.len();
        let optimizer_dimension_count = optimizer_dimensions.len();
        validate_cbsem_studentized_archive_workload_v1(
            requested,
            recipe.settings.workers,
            complete_n,
            modeled_variable_count,
            free_parameter_row_count,
            optimizer_dimension_count,
        )?;
    }
    let mut successful = Vec::<CbsemExactCaseBootstrapWitnessV1>::with_capacity(refits.rows.len());
    let mut all_indices = BTreeSet::new();
    for row in &refits.rows {
        let index = exact_bootstrap_u32(
            exact_bootstrap_cell(refits, successful.len(), "replicate_index")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 refit index is missing"))?,
        )
        .ok_or_else(|| invalid_cbsem_rmsea("v9 refit index is invalid"))?;
        if row.id != format!("bootstrap_refit_{index:05}")
            || index >= requested
            || !all_indices.insert(index)
            || successful
                .last()
                .is_some_and(|prior| prior.replicate_index >= index)
        {
            return Err(invalid_cbsem_rmsea("v9 successful-refit ordering drifted"));
        }
        let row_index = successful.len();
        let schedule_digest = exact_bootstrap_text(
            exact_bootstrap_cell(refits, row_index, "sampling_positions_sha256")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 refit schedule digest is missing"))?,
        )
        .filter(|value| exact_bootstrap_is_sha256(value))
        .ok_or_else(|| invalid_cbsem_rmsea("v9 refit schedule digest is invalid"))?;
        let source_digest = exact_bootstrap_text(
            exact_bootstrap_cell(refits, row_index, "sample_indices_sha256")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 refit source digest is missing"))?,
        )
        .filter(|value| exact_bootstrap_is_sha256(value))
        .ok_or_else(|| invalid_cbsem_rmsea("v9 refit source-row digest is invalid"))?;
        let _ = source_digest;
        let operation = format!("{}:primary", CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1);
        let positions = bootstrap_indices(complete_n, seed, &operation, index);
        if schedule_digest
            != cbsem_exact_case_bootstrap_schedule_positions_digest_v1(
                CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
                seed,
                index,
                complete_n,
                &positions,
            )
        {
            return Err(invalid_cbsem_rmsea(
                "v9 successful-refit schedule digest does not match deterministic regeneration",
            ));
        }
        let estimates_text = exact_bootstrap_text(
            exact_bootstrap_cell(refits, row_index, "parameter_estimates_json")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 refit estimate vector is missing"))?,
        )
        .ok_or_else(|| invalid_cbsem_rmsea("v9 refit estimate vector is not text"))?;
        let estimates = serde_json::from_str::<Vec<f64>>(estimates_text)
            .ok()
            .filter(|values| serde_json::to_string(values).ok().as_deref() == Some(estimates_text))
            .filter(|values| {
                values.len() == parameter_ids.len()
                    && values
                        .iter()
                        .all(|value| value.is_finite() && value.to_bits() != (-0.0_f64).to_bits())
            })
            .ok_or_else(|| invalid_cbsem_rmsea("v9 refit estimate vector is noncanonical"))?;
        let iterations = exact_bootstrap_u32(
            exact_bootstrap_cell(refits, row_index, "iterations")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 refit iterations are missing"))?,
        )
        .ok_or_else(|| invalid_cbsem_rmsea("v9 refit iterations are invalid"))?;
        let objective = exact_bootstrap_number(
            exact_bootstrap_cell(refits, row_index, "objective")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 refit objective is missing"))?,
        )
        .ok_or_else(|| invalid_cbsem_rmsea("v9 refit objective is invalid"))?;
        let gradient = exact_bootstrap_number(
            exact_bootstrap_cell(refits, row_index, "gradient_norm")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 refit gradient is missing"))?,
        )
        .ok_or_else(|| invalid_cbsem_rmsea("v9 refit gradient is invalid"))?;
        if iterations == 0
            || iterations > recipe.settings.max_iterations
            || objective < 0.0
            || gradient < 0.0
        {
            return Err(invalid_cbsem_rmsea(
                "v9 refit optimizer witness is outside the exact contract",
            ));
        }
        successful.push(CbsemExactCaseBootstrapWitnessV1 {
            replicate_index: index,
            sampling_positions_sha256: schedule_digest.to_owned(),
            sample_indices_sha256: source_digest.to_owned(),
            parameter_estimates: estimates,
            iterations,
            objective,
            gradient_norm: gradient,
        });
    }
    let mut prior_failure = None;
    let mut failed_refits =
        Vec::<CbsemExactCaseBootstrapFailureV1>::with_capacity(failures.rows.len());
    for (row_index, row) in failures.rows.iter().enumerate() {
        let index = exact_bootstrap_u32(
            exact_bootstrap_cell(failures, row_index, "replicate_index")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 failure index is missing"))?,
        )
        .ok_or_else(|| invalid_cbsem_rmsea("v9 failure index is invalid"))?;
        let schedule_digest = exact_bootstrap_text(
            exact_bootstrap_cell(failures, row_index, "sampling_positions_sha256")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 failure schedule digest is missing"))?,
        )
        .filter(|value| exact_bootstrap_is_sha256(value))
        .ok_or_else(|| invalid_cbsem_rmsea("v9 failure schedule digest is invalid"))?;
        let source_digest = exact_bootstrap_text(
            exact_bootstrap_cell(failures, row_index, "sample_indices_sha256")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 failure source digest is missing"))?,
        )
        .filter(|value| exact_bootstrap_is_sha256(value))
        .ok_or_else(|| invalid_cbsem_rmsea("v9 failure source digest is invalid"))?;
        let kind = match exact_bootstrap_text(
            exact_bootstrap_cell(failures, row_index, "kind")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 failure kind is missing"))?,
        ) {
            Some("moment_matrix_not_positive_definite") => {
                CbsemExactCaseBootstrapFailureKindV1::MomentMatrixNotPositiveDefinite
            }
            Some("non_convergence") => CbsemExactCaseBootstrapFailureKindV1::NonConvergence,
            Some("inadmissible_solution") => {
                CbsemExactCaseBootstrapFailureKindV1::InadmissibleSolution
            }
            Some("numerical_failure") => CbsemExactCaseBootstrapFailureKindV1::NumericalFailure,
            _ => return Err(invalid_cbsem_rmsea("v9 failure kind is invalid")),
        };
        let message = exact_bootstrap_text(
            exact_bootstrap_cell(failures, row_index, "message")
                .ok_or_else(|| invalid_cbsem_rmsea("v9 failure message is missing"))?,
        )
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid_cbsem_rmsea("v9 failure message is invalid"))?;
        let operation = format!("{}:primary", CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1);
        let positions = bootstrap_indices(complete_n, seed, &operation, index);
        if row.id != format!("bootstrap_failure_{index:05}")
            || index >= requested
            || !all_indices.insert(index)
            || prior_failure.is_some_and(|prior| prior >= index)
            || schedule_digest
                != cbsem_exact_case_bootstrap_schedule_positions_digest_v1(
                    CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
                    seed,
                    index,
                    complete_n,
                    &positions,
                )
        {
            return Err(invalid_cbsem_rmsea(
                "v9 failure ledger identity, classification, or schedule drifted",
            ));
        }
        failed_refits.push(CbsemExactCaseBootstrapFailureV1 {
            replicate_index: index,
            sampling_positions_sha256: schedule_digest.into(),
            sample_indices_sha256: source_digest.into(),
            kind,
            message: message.into(),
        });
        prior_failure = Some(index);
    }
    if all_indices.into_iter().ne(0..requested) {
        return Err(invalid_cbsem_rmsea(
            "v9 success/failure ledgers do not exactly partition 0..B-1",
        ));
    }
    let required = required_usable_refits(requested);
    let available = usable >= required;
    let status = summary_text("inference_status")?;
    if available {
        if status != "available"
            || !matches!(
                summary_cell("unavailable_reason_code")?,
                CanonicalResultCellV2::Missing {
                    reason: CanonicalMissingReasonV2::NotApplicable,
                    display: None,
                }
            )
            || !matches!(
                summary_cell("unavailable_message")?,
                CanonicalResultCellV2::Missing {
                    reason: CanonicalMissingReasonV2::NotApplicable,
                    display: None,
                }
            )
            || intervals.rows.len() != parameter_ids.len()
        {
            return Err(invalid_cbsem_rmsea(
                "v9 available inference status, nulls, or interval count drifted",
            ));
        }
    } else {
        let expected_message = format!(
            "Exact CB-SEM case-bootstrap inference is unavailable because {usable} usable refits are below the required {required}; no intervals were emitted."
        );
        if status != "unavailable"
            || summary_text("unavailable_reason_code")? != "insufficient_usable_refits"
            || summary_text("unavailable_message")? != expected_message
            || !intervals.rows.is_empty()
        {
            return Err(invalid_cbsem_rmsea(
                "v9 unavailable inference status, reason, or interval null contract drifted",
            ));
        }
    }
    let mut reconstructed_intervals = Vec::with_capacity(intervals.rows.len());
    for (parameter_index, (parameter_id, point_parameter)) in parameter_ids
        .iter()
        .zip(free_point_parameters.iter().copied())
        .enumerate()
    {
        if !available {
            break;
        }
        let row = &intervals.rows[parameter_index];
        if row.id != format!("bootstrap_interval_{parameter_index:04}")
            || exact_bootstrap_text(&row.cells[0]) != Some(parameter_id)
        {
            return Err(invalid_cbsem_rmsea(
                "v9 interval parameter identity drifted",
            ));
        }
        let mut values = successful
            .iter()
            .map(|witness| witness.parameter_estimates[parameter_index])
            .collect::<Vec<_>>();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let standard_error = (values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (values.len() - 1) as f64)
            .sqrt();
        values.sort_by(f64::total_cmp);
        let lower_probability = (1.0 - CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1) / 2.0;
        let upper_probability = 1.0 - lower_probability;
        let lower = exact_bootstrap_type7(&values, lower_probability);
        let upper = exact_bootstrap_type7(&values, upper_probability);
        let expected = [
            point_parameter.estimate,
            mean,
            mean - point_parameter.estimate,
            standard_error,
            lower,
            upper,
        ];
        let recorded = row.cells[1..=6]
            .iter()
            .map(exact_bootstrap_number)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| invalid_cbsem_rmsea("v9 interval contains invalid numeric cells"))?;
        if recorded
            .iter()
            .zip(expected)
            .any(|(recorded, expected)| recorded.to_bits() != expected.to_bits())
            || exact_bootstrap_u32(&row.cells[7]) != Some(usable)
        {
            return Err(invalid_cbsem_rmsea(
                "v9 interval original, mean, bias, sample SD, Type-7 bounds, or usable count drifted",
            ));
        }
        reconstructed_intervals.push(CbsemExactCaseBootstrapParameterIntervalV1 {
            parameter_id: parameter_id.clone(),
            original: expected[0],
            bootstrap_mean: expected[1],
            bias: expected[2],
            standard_error: expected[3],
            percentile_lower: expected[4],
            percentile_upper: expected[5],
            usable_replicates: usable,
        });
    }
    let original = CbsemExactCaseBootstrapRefitV1 {
        method_version: CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1.into(),
        estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3.into(),
        source_dataset_id: source_dataset_id.into(),
        source_dataset_fingerprint: source_fingerprint.into(),
        compiler_analytical_identity_sha256: compiler_sha256.into(),
        plan_sha256: plan_sha256.into(),
        model_scientific_sha256: model_sha256.into(),
        source_row_count: dataset.schema.case_count,
        complete_case_sample_size: complete_n,
        complete_case_universe_digest_method:
            CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1.into(),
        complete_case_universe_sha256: summary_text("complete_case_universe_sha256")?.into(),
        resampled_observations: complete_n,
        covariance_denominator: point_projection.covariance_denominator,
        sample_indices_digest_method: CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1.into(),
        sampling_positions_digest_method:
            CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1.into(),
        sampling_positions_sha256: String::new(),
        sample_indices_sha256: String::new(),
        free_parameters: free_point_parameters
            .iter()
            .map(|parameter| CbsemExactCaseBootstrapParameterEstimateV1 {
                parameter_id: parameter.parameter_id.clone(),
                estimate: parameter.estimate,
            })
            .collect(),
        iterations: point_projection.iterations,
        objective: point_projection.objective,
        gradient_norm: point_projection.gradient_norm,
    };
    let expected_hypothesis = if let Some(hypothesis_table) = hypothesis_table {
        let parameter_eligibility =
            compile_cbsem_exact_case_bootstrap_zero_null_eligibility_v1(point_plan);
        // The pure hypothesis summarizer consumes only the ordered point
        // estimates and successful-refit ledger. Other refit metadata is
        // already validated above against the canonical v9 base tables.
        let expected = summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1(
            &original,
            &successful,
            required,
            CbsemExactCaseBootstrapHypothesisTestPlanV1 {
                selected_test_tail: bootstrap_config.test_tail,
                parameter_eligibility: &parameter_eligibility,
            },
        )
        .map_err(|error| invalid_cbsem_rmsea(error.to_string()))?;
        validate_exact_bootstrap_hypothesis_table_v1(hypothesis_table, &expected)?;
        Some(expected)
    } else {
        None
    };
    if is_v11 || is_v12 {
        let base = CbsemExactCaseBootstrapResultV1 {
            method_version: CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1.into(),
            estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3.into(),
            source_dataset_id: source_dataset_id.into(),
            source_dataset_fingerprint: source_fingerprint.into(),
            outer_recipe_analytical_identity_sha256: outer_recipe_sha256.into(),
            base_point_result_sha256: base_point_sha256,
            compiler_analytical_identity_sha256: compiler_sha256.into(),
            plan_sha256: plan_sha256.into(),
            model_scientific_sha256: model_sha256.into(),
            complete_case_sample_size: complete_n,
            complete_case_universe_digest_method:
                CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1.into(),
            complete_case_universe_sha256: summary_text("complete_case_universe_sha256")?.into(),
            covariance_denominator: point_projection.covariance_denominator,
            sample_indices_digest_method: CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1.into(),
            sampling_positions_digest_method:
                CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1.into(),
            interval_method: CBSEM_EXACT_CASE_BOOTSTRAP_INTERVAL_METHOD_V1.into(),
            confidence_level: summary_number("confidence_level")?,
            requested_replicates: requested,
            attempted_refits: attempted,
            usable_replicates: usable,
            failed_replicates: failed,
            minimum_usable_fraction: summary_number("minimum_usable_fraction")?,
            minimum_usable_replicates: required,
            seed,
            stream_token: CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1.into(),
            retry_policy: CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1.into(),
            max_attempts_per_replicate: CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
            parameter_ids,
            inference: if available {
                CbsemExactCaseBootstrapInferenceV1::Available
            } else {
                CbsemExactCaseBootstrapInferenceV1::Unavailable {
                    reason_code: summary_text("unavailable_reason_code")?.into(),
                    message: summary_text("unavailable_message")?.into(),
                }
            },
            intervals: reconstructed_intervals,
            hypothesis_tests: expected_hypothesis,
            successful_refits: successful,
            failed_refits,
        };
        if is_v11 {
            validate_recipe_v4_cbsem_studentized_bootstrap_document_v1(document, &original, &base)?;
        } else {
            validate_recipe_v4_cbsem_bca_bootstrap_document_v1(document, &original, &base)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ProjectArchiveUpgradeRequestV6 {
    pub source_archive_sha256: String,
    pub source_archive_path: String,
    pub destination_archive_path: String,
    pub upgraded_at: DateTime<Utc>,
    /// The UI/archive adapter supplies drawings because legacy `ModelSpec`
    /// never carried them as scientific relations.
    pub legacy_display_covariances: BTreeMap<String, Vec<LegacyDisplayCovarianceV4>>,
}

#[derive(Debug, Clone)]
pub struct ProjectArchiveUpgradePlanV6 {
    pub document: ProjectArchiveDocumentV6,
    pub source_must_remain_unchanged: bool,
    pub destination_must_be_new: bool,
}

impl ProjectArchiveUpgradePlanV6 {
    pub fn ensure_valid(&self) -> Result<(), ProjectArchiveV6Error> {
        if !self.source_must_remain_unchanged || !self.destination_must_be_new {
            return Err(ProjectArchiveV6Error::UnsafeUpgradePolicy);
        }
        if self.document.upgrade_lineage().is_none() {
            return Err(ProjectArchiveV6Error::UpgradeOriginRequired);
        }
        self.document.ensure_valid()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FutureProjectArchiveReadOnlyV6 {
    pub schema_version: u32,
    pub document_sha256: String,
    pub dataset_count: usize,
    pub model_count: usize,
    pub recipe_count: usize,
    pub result_count: usize,
    pub canonical_result_document_count: usize,
    pub read_only: bool,
}

#[derive(Debug, Clone)]
pub enum ProjectArchiveInspectionV6 {
    HistoricalUpgradeRequired { schema_version: u32 },
    Current(ProjectArchiveDocumentV6),
    FutureReadOnly(FutureProjectArchiveReadOnlyV6),
}

/// Serializes a validated schema-v6 document into stable UTF-8 JSON bytes.
///
/// Object keys are recursively sorted before encoding. Vector order is kept
/// because model, recipe, result, and presentation order can be meaningful.
pub fn serialize_project_document_v6(
    document: &ProjectArchiveDocumentV6,
) -> Result<Vec<u8>, ProjectArchiveV6Error> {
    document.ensure_valid()?;
    let value = canonicalize_json_value(serde_json::to_value(document)?);
    Ok(serde_json::to_vec(&value)?)
}

/// Strictly decodes schema-v6 document bytes. Duplicate keys, unknown fields,
/// invalid cross-references, and digest mismatches are rejected.
pub fn deserialize_project_document_v6(
    bytes: &[u8],
) -> Result<ProjectArchiveDocumentV6, ProjectArchiveV6Error> {
    reject_duplicate_json_object_keys(bytes, "project schema v6 document")
        .map_err(ProjectArchiveV6Error::LegacyArchiveLayer)?;
    let document: ProjectArchiveDocumentV6 = serde_json::from_slice(bytes)?;
    document.ensure_valid()?;
    Ok(document)
}

/// Reads and strictly validates a standalone schema-v6 document file.
pub fn read_project_document_v6(
    path: &Path,
) -> Result<ProjectArchiveDocumentV6, ProjectArchiveV6Error> {
    deserialize_project_document_v6(&fs::read(path)?)
}

/// Inspects a document using the schema version embedded in its own bytes.
/// Future documents are summarized without interpreting their scientific
/// payload and are always marked read-only.
pub fn inspect_project_document_bytes_v6(
    bytes: &[u8],
) -> Result<ProjectArchiveInspectionV6, ProjectArchiveV6Error> {
    reject_duplicate_json_object_keys(bytes, "project document")
        .map_err(ProjectArchiveV6Error::LegacyArchiveLayer)?;
    let value: Value = serde_json::from_slice(bytes)?;
    let object = value
        .as_object()
        .ok_or(ProjectArchiveV6Error::FutureDocumentNotObject)?;
    let schema_version = object
        .get("schema_version")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or(ProjectArchiveV6Error::InvalidEmbeddedSchemaVersion)?;
    inspect_project_document_v6(schema_version, bytes)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectArchiveWriteReceiptV6 {
    pub schema_version: u32,
    pub project_id: Uuid,
    pub destination_archive_path: String,
    pub document_sha256: String,
    pub byte_length: u64,
    pub post_write_validated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectArchiveUpgradeReceiptV6 {
    pub write: ProjectArchiveWriteReceiptV6,
    pub source_archive_path: String,
    pub source_archive_sha256: String,
    pub source_verified_unchanged: bool,
    pub historical_results_immutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectArchiveCanonicalAppendReceiptV6 {
    pub schema_version: u32,
    pub project_id: Uuid,
    pub archive_path: String,
    pub source_document_sha256: String,
    pub updated_document_sha256: String,
    pub canonical_document_id: String,
    pub run_id: String,
    pub canonical_result_document_count: usize,
    pub source_verified_at_commit: bool,
    pub post_write_validated: bool,
    pub rollback_copy_removed: bool,
}

/// Atomically publishes a validated schema-v6 JSON document to a destination
/// that must not already exist. This is a foundation API, not the live `.qpls`
/// ZIP writer.
pub fn write_project_document_v6_new(
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
) -> Result<ProjectArchiveWriteReceiptV6, ProjectArchiveV6Error> {
    write_project_document_v6_new_with_checks(destination, document, |_| Ok(()), |_| Ok(()))
}

/// Executes an upgrade-copy plan while preserving the source archive exactly.
/// The source digest and both paths must match the immutable lineage embedded
/// in the plan. Any post-write validation failure removes the new destination.
pub fn execute_project_upgrade_copy_v6(
    source: &Path,
    destination: &Path,
    plan: &ProjectArchiveUpgradePlanV6,
) -> Result<ProjectArchiveUpgradeReceiptV6, ProjectArchiveV6Error> {
    plan.ensure_valid()?;
    let lineage = plan
        .document
        .upgrade_lineage()
        .ok_or(ProjectArchiveV6Error::UpgradeOriginRequired)?;
    ensure_lineage_path_binding("source_archive_path", source, &lineage.source_archive_path)?;
    ensure_lineage_path_binding(
        "destination_archive_path",
        destination,
        &lineage.destination_archive_path,
    )?;
    ensure_destination_absent(destination)?;

    let expected_source_sha256 = &lineage.source_archive_sha256;
    let source_sha256 = sha256_file_v6(source)?;
    if source_sha256 != *expected_source_sha256 {
        return Err(ProjectArchiveV6Error::SourceDigestMismatch {
            expected: expected_source_sha256.clone(),
            observed: source_sha256,
        });
    }

    let verify_source_before = |_: &Path| {
        let observed = sha256_file_v6(source)?;
        if observed != *expected_source_sha256 {
            return Err(ProjectArchiveV6Error::SourceChangedDuringUpgrade);
        }
        Ok(())
    };
    let verify_source_after = |_: &Path| {
        let observed = sha256_file_v6(source)?;
        if observed != *expected_source_sha256 {
            return Err(ProjectArchiveV6Error::SourceChangedDuringUpgrade);
        }
        Ok(())
    };
    let write = write_project_document_v6_new_with_checks(
        destination,
        &plan.document,
        verify_source_before,
        verify_source_after,
    )?;

    Ok(ProjectArchiveUpgradeReceiptV6 {
        write,
        source_archive_path: lineage.source_archive_path.clone(),
        source_archive_sha256: expected_source_sha256.clone(),
        source_verified_unchanged: true,
        historical_results_immutable: lineage.historical_results_immutable,
    })
}

/// Appends one immutable canonical result to an existing schema-v6 standalone
/// document. The caller must bind the exact source digest it inspected. The
/// update is written and validated under a private same-directory name, then
/// atomically replaces the source while retaining a rollback copy until exact
/// post-write validation succeeds.
pub fn append_canonical_result_document_v2_file_v6(
    archive_path: &Path,
    expected_source_sha256: &str,
    canonical_document: CanonicalResultDocumentV2,
) -> Result<ProjectArchiveCanonicalAppendReceiptV6, ProjectArchiveV6Error> {
    append_canonical_result_document_v2_file_v6_with_cancel(
        archive_path,
        expected_source_sha256,
        canonical_document,
        || false,
    )
}

/// Atomically inserts the exact current Recipe-v4 and its canonical result.
/// This is the persistence boundary used by a newly completed exact-CB-SEM
/// run whose transient recipe is not yet resident in the schema-6 document.
pub fn append_recipe_v4_and_canonical_result_document_v2_file_v6(
    archive_path: &Path,
    expected_source_sha256: &str,
    recipe: AnalysisRecipeV4,
    canonical_document: CanonicalResultDocumentV2,
) -> Result<ProjectArchiveCanonicalAppendReceiptV6, ProjectArchiveV6Error> {
    append_canonical_result_document_v2_file_v6_inner(
        archive_path,
        expected_source_sha256,
        Some(recipe),
        canonical_document,
        || false,
    )
}

pub fn append_canonical_result_document_v2_file_v6_with_cancel<Cancelled>(
    archive_path: &Path,
    expected_source_sha256: &str,
    canonical_document: CanonicalResultDocumentV2,
    cancelled: Cancelled,
) -> Result<ProjectArchiveCanonicalAppendReceiptV6, ProjectArchiveV6Error>
where
    Cancelled: Fn() -> bool,
{
    append_canonical_result_document_v2_file_v6_inner(
        archive_path,
        expected_source_sha256,
        None,
        canonical_document,
        cancelled,
    )
}

fn append_canonical_result_document_v2_file_v6_inner<Cancelled>(
    archive_path: &Path,
    expected_source_sha256: &str,
    recipe: Option<AnalysisRecipeV4>,
    canonical_document: CanonicalResultDocumentV2,
    cancelled: Cancelled,
) -> Result<ProjectArchiveCanonicalAppendReceiptV6, ProjectArchiveV6Error>
where
    Cancelled: Fn() -> bool,
{
    validate_sha256("expected_source_sha256", expected_source_sha256)?;
    let metadata = fs::symlink_metadata(archive_path)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(ProjectArchiveV6Error::AppendArchiveMustBeRegularFile(
            archive_path.to_path_buf(),
        ));
    }

    let lock_path = append_private_path_v6(archive_path, "lock")?;
    let mut lock = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                ProjectArchiveV6Error::AppendAlreadyInProgress(archive_path.to_path_buf())
            } else {
                ProjectArchiveV6Error::Io(error)
            }
        })?;
    writeln!(lock, "{}", std::process::id())?;
    lock.sync_all()?;
    drop(lock);
    let _lock_guard = TemporaryProjectDocumentV6Guard::new(lock_path);

    let source_bytes = fs::read(archive_path)?;
    let observed_source_sha256 = sha256_bytes(&source_bytes);
    if observed_source_sha256 != expected_source_sha256 {
        return Err(ProjectArchiveV6Error::SourceDigestMismatch {
            expected: expected_source_sha256.to_owned(),
            observed: observed_source_sha256,
        });
    }
    if source_bytes.starts_with(b"PK\x03\x04") {
        return append_canonical_result_document_v2_zip_v6_inner(
            archive_path,
            expected_source_sha256,
            source_bytes,
            recipe,
            canonical_document,
            cancelled,
        );
    }
    let mut source = deserialize_project_document_v6(&source_bytes)?;
    if cancelled() {
        return Err(ProjectArchiveV6Error::AppendCancelled);
    }

    if let Some(recipe) = recipe {
        source.recipes.push(recipe);
        source.ensure_valid()?;
    }

    let canonical_document_id = canonical_document.document_id.clone();
    let run_id = canonical_document.provenance.run_id.clone();
    let updated = attach_canonical_result_document_v2_v6(&source, canonical_document)?;
    let updated_bytes = serialize_project_document_v6(&updated)?;
    let updated_document_sha256 = sha256_bytes(&updated_bytes);

    let temporary = append_private_path_v6(archive_path, &format!("{}.tmp", Uuid::new_v4()))?;
    let mut temporary_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let mut temporary_guard = TemporaryProjectDocumentV6Guard::new(temporary.clone());
    temporary_file.write_all(&updated_bytes)?;
    temporary_file.sync_all()?;
    drop(temporary_file);
    let temporary_bytes = fs::read(&temporary)?;
    if temporary_bytes != updated_bytes
        || serialize_project_document_v6(&deserialize_project_document_v6(&temporary_bytes)?)?
            != updated_bytes
    {
        return Err(ProjectArchiveV6Error::TemporaryDocumentChanged);
    }
    if cancelled() {
        return Err(ProjectArchiveV6Error::AppendCancelled);
    }

    // Recheck after all potentially expensive serialization and validation.
    let commit_source = fs::read(archive_path)?;
    if commit_source != source_bytes || sha256_bytes(&commit_source) != expected_source_sha256 {
        return Err(ProjectArchiveV6Error::SourceChangedDuringAppend);
    }

    let rollback = append_private_path_v6(archive_path, &format!("{}.rollback", Uuid::new_v4()))?;
    atomic_replace_with_rollback_v6(&temporary, archive_path, &rollback)?;
    temporary_guard.disarm();
    let mut rollback_guard = TemporaryProjectDocumentV6Guard::new(rollback.clone());

    let validation = (|| -> Result<(), ProjectArchiveV6Error> {
        if sha256_file_v6(&rollback)? != expected_source_sha256 {
            return Err(ProjectArchiveV6Error::SourceChangedDuringAppend);
        }
        let persisted_bytes = fs::read(archive_path)?;
        if persisted_bytes != updated_bytes
            || sha256_bytes(&persisted_bytes) != updated_document_sha256
        {
            return Err(ProjectArchiveV6Error::PostWriteValidation);
        }
        let _persisted = deserialize_project_document_v6(&persisted_bytes)?;
        Ok(())
    })();
    if let Err(error) = validation {
        restore_rollback_v6(&rollback, archive_path).map_err(|rollback_error| {
            ProjectArchiveV6Error::AppendRollbackFailed {
                original: error.to_string(),
                rollback: rollback_error.to_string(),
            }
        })?;
        rollback_guard.disarm();
        return Err(error);
    }

    fs::remove_file(&rollback)?;
    rollback_guard.disarm();
    sync_parent_directory_v6(archive_path)?;
    Ok(ProjectArchiveCanonicalAppendReceiptV6 {
        schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        project_id: updated.project_id,
        archive_path: archive_path.to_string_lossy().into_owned(),
        source_document_sha256: expected_source_sha256.to_owned(),
        updated_document_sha256,
        canonical_document_id,
        run_id,
        canonical_result_document_count: updated.canonical_result_documents.len(),
        source_verified_at_commit: true,
        post_write_validated: true,
        rollback_copy_removed: true,
    })
}

fn zip_layer_error_v6(error: impl Into<ProjectError>) -> ProjectArchiveV6Error {
    ProjectArchiveV6Error::LegacyArchiveLayer(error.into())
}

fn append_canonical_result_document_v2_zip_v6_inner<Cancelled>(
    archive_path: &Path,
    expected_source_sha256: &str,
    source_bytes: Vec<u8>,
    recipe: Option<AnalysisRecipeV4>,
    canonical_document: CanonicalResultDocumentV2,
    cancelled: Cancelled,
) -> Result<ProjectArchiveCanonicalAppendReceiptV6, ProjectArchiveV6Error>
where
    Cancelled: Fn() -> bool,
{
    let loaded = load_project_archive_v6(archive_path).map_err(zip_layer_error_v6)?;
    let mut source = loaded.document;
    let source_manifest = loaded.manifest;
    if let Some(recipe) = recipe {
        source.recipes.push(recipe);
        source.ensure_valid()?;
    }
    let canonical_document_id = canonical_document.document_id.clone();
    let run_id = canonical_document.provenance.run_id.clone();
    let updated = attach_canonical_result_document_v2_v6(&source, canonical_document)?;
    let updated_project_bytes = serialize_project_document_v6(&updated)?;
    if cancelled() {
        return Err(ProjectArchiveV6Error::AppendCancelled);
    }

    let temporary = append_private_path_v6(archive_path, &format!("{}.tmp", Uuid::new_v4()))?;
    let temporary_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let mut temporary_guard = TemporaryProjectDocumentV6Guard::new(temporary.clone());
    let mut source_archive =
        ZipArchive::new(File::open(archive_path)?).map_err(zip_layer_error_v6)?;
    let mut output = ZipWriter::new(temporary_file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut checksums = BTreeMap::new();

    output
        .start_file(PROJECT_ENTRY_NAME, options)
        .map_err(zip_layer_error_v6)?;
    output.write_all(&updated_project_bytes)?;
    checksums.insert(
        PROJECT_ENTRY_NAME.to_owned(),
        sha256_bytes(&updated_project_bytes),
    );
    for descriptor in &updated.datasets {
        let entry_name = format!("data/{}.arrow", descriptor.id);
        let mut entry = source_archive
            .by_name(&entry_name)
            .map_err(zip_layer_error_v6)?;
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes)?;
        output
            .start_file(&entry_name, options)
            .map_err(zip_layer_error_v6)?;
        output.write_all(&bytes)?;
        checksums.insert(entry_name, sha256_bytes(&bytes));
    }
    drop(source_archive);
    let manifest = ProjectManifest {
        schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        project_id: updated.project_id,
        name: updated.name.clone(),
        created_at: updated.created_at,
        modified_at: updated.modified_at,
        engine_version: source_manifest.engine_version,
        checksum_algorithm: "sha256".into(),
        checksums,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    output
        .start_file(MANIFEST_ENTRY_NAME, options)
        .map_err(zip_layer_error_v6)?;
    output.write_all(&manifest_bytes)?;
    let finished = output.finish().map_err(zip_layer_error_v6)?;
    finished.sync_all()?;
    drop(finished);

    let temporary_loaded = load_project_archive_v6(&temporary).map_err(zip_layer_error_v6)?;
    if serialize_project_document_v6(&temporary_loaded.document)? != updated_project_bytes {
        return Err(ProjectArchiveV6Error::TemporaryDocumentChanged);
    }
    if cancelled() {
        return Err(ProjectArchiveV6Error::AppendCancelled);
    }
    if fs::read(archive_path)? != source_bytes
        || sha256_file_v6(archive_path)? != expected_source_sha256
    {
        return Err(ProjectArchiveV6Error::SourceChangedDuringAppend);
    }

    let rollback = append_private_path_v6(archive_path, &format!("{}.rollback", Uuid::new_v4()))?;
    atomic_replace_with_rollback_v6(&temporary, archive_path, &rollback)?;
    temporary_guard.disarm();
    let mut rollback_guard = TemporaryProjectDocumentV6Guard::new(rollback.clone());
    let validation = (|| -> Result<String, ProjectArchiveV6Error> {
        if sha256_file_v6(&rollback)? != expected_source_sha256 {
            return Err(ProjectArchiveV6Error::SourceChangedDuringAppend);
        }
        let reopened = load_project_archive_v6(archive_path).map_err(zip_layer_error_v6)?;
        if serialize_project_document_v6(&reopened.document)? != updated_project_bytes {
            return Err(ProjectArchiveV6Error::PostWriteValidation);
        }
        sha256_file_v6(archive_path)
    })();
    let updated_archive_sha256 = match validation {
        Ok(value) => value,
        Err(error) => {
            restore_rollback_v6(&rollback, archive_path).map_err(|rollback_error| {
                ProjectArchiveV6Error::AppendRollbackFailed {
                    original: error.to_string(),
                    rollback: rollback_error.to_string(),
                }
            })?;
            rollback_guard.disarm();
            return Err(error);
        }
    };
    fs::remove_file(&rollback)?;
    rollback_guard.disarm();
    sync_parent_directory_v6(archive_path)?;
    Ok(ProjectArchiveCanonicalAppendReceiptV6 {
        schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        project_id: updated.project_id,
        archive_path: archive_path.to_string_lossy().into_owned(),
        source_document_sha256: expected_source_sha256.to_owned(),
        updated_document_sha256: updated_archive_sha256,
        canonical_document_id,
        run_id,
        canonical_result_document_count: updated.canonical_result_documents.len(),
        source_verified_at_commit: true,
        post_write_validated: true,
        rollback_copy_removed: true,
    })
}

pub fn inspect_project_document_v6(
    observed_schema_version: u32,
    bytes: &[u8],
) -> Result<ProjectArchiveInspectionV6, ProjectArchiveV6Error> {
    match classify_project_archive_schema_v6(observed_schema_version)? {
        ProjectArchiveSchemaAccessV6::HistoricalUpgradeCopyRequired => {
            Ok(ProjectArchiveInspectionV6::HistoricalUpgradeRequired {
                schema_version: observed_schema_version,
            })
        }
        ProjectArchiveSchemaAccessV6::CurrentEditable => {
            reject_duplicate_json_object_keys(bytes, "project schema v6 document")
                .map_err(ProjectArchiveV6Error::LegacyArchiveLayer)?;
            let document: ProjectArchiveDocumentV6 = serde_json::from_slice(bytes)?;
            document.ensure_valid()?;
            Ok(ProjectArchiveInspectionV6::Current(document))
        }
        ProjectArchiveSchemaAccessV6::FutureReadOnly => {
            reject_duplicate_json_object_keys(bytes, "future project document")
                .map_err(ProjectArchiveV6Error::LegacyArchiveLayer)?;
            let value: Value = serde_json::from_slice(bytes)?;
            let object = value
                .as_object()
                .ok_or(ProjectArchiveV6Error::FutureDocumentNotObject)?;
            Ok(ProjectArchiveInspectionV6::FutureReadOnly(
                FutureProjectArchiveReadOnlyV6 {
                    schema_version: observed_schema_version,
                    document_sha256: sha256_bytes(bytes),
                    dataset_count: array_len(object.get("datasets")),
                    model_count: array_len(object.get("models")),
                    recipe_count: array_len(object.get("recipes"))
                        + array_len(object.get("historical_recipes")),
                    result_count: object
                        .get("historical_results")
                        .or_else(|| object.get("results"))
                        .map_or(0, |value| value.as_array().map_or(0, Vec::len)),
                    canonical_result_document_count: array_len(
                        object.get("canonical_result_documents"),
                    ),
                    read_only: true,
                },
            ))
        }
    }
}

pub fn plan_project_upgrade_to_v6(
    source: &Project,
    request: &ProjectArchiveUpgradeRequestV6,
) -> Result<ProjectArchiveUpgradePlanV6, ProjectArchiveV6Error> {
    if source.read_only || source.source_archive_version > 5 {
        return Err(ProjectArchiveV6Error::FutureSourceReadOnly(
            source.source_archive_version,
        ));
    }
    classify_project_archive_schema_v6(source.source_archive_version)?;
    // Upgrade planning still has the resident v5 Arrow datasets, so require
    // exact replay before copying the reserved lineage value into schema v6.
    validate_project_data_lineage_resident_v1(&source.datasets, &source.layouts)?;
    validate_sha256("source_archive_sha256", &request.source_archive_sha256)?;
    validate_distinct_paths(
        &request.source_archive_path,
        &request.destination_archive_path,
    )?;

    let mut legacy_models = BTreeMap::<String, ModelSpec>::new();
    for model in &source.models {
        insert_legacy_model(&mut legacy_models, model)?;
    }
    for recipe in &source.recipes {
        insert_legacy_model(&mut legacy_models, &recipe.model)?;
    }
    for key in request.legacy_display_covariances.keys() {
        if !legacy_models.contains_key(key) {
            return Err(ProjectArchiveV6Error::UnknownDisplayCovarianceModel(
                key.clone(),
            ));
        }
    }

    let mut model_intents = BTreeMap::<String, BTreeSet<LegacySemIntentV6>>::new();
    for recipe in &source.recipes {
        model_intents
            .entry(recipe.model.id.to_string())
            .or_default()
            .insert(legacy_sem_intent(recipe.settings.method));
    }

    let mut models = Vec::with_capacity(legacy_models.len());
    for (model_id, legacy_model) in &legacy_models {
        let drawings = request
            .legacy_display_covariances
            .get(model_id)
            .cloned()
            .unwrap_or_default();
        validate_display_covariances(legacy_model, &drawings)?;
        let automatic_interpretation = unambiguous_interpretation(model_intents.get(model_id));
        let converted = automatic_interpretation.map(|interpretation| {
            convert_legacy_basic_model_v4(legacy_model, interpretation, &drawings)
                .map(|model| (model, interpretation))
        });
        match converted {
            Some(Ok((model, _interpretation))) => {
                let scientific_sha256 = model.scientific_sha256()?;
                models.push(ProjectModelRecordV6 {
                    model_id: model_id.clone(),
                    payload: ProjectModelPayloadV6::SemModelV4 {
                        model,
                        scientific_sha256,
                    },
                });
            }
            Some(Err(error)) => models.push(ProjectModelRecordV6 {
                model_id: model_id.clone(),
                payload: ProjectModelPayloadV6::LegacyEstimandUnspecified {
                    legacy_model: legacy_model.clone(),
                    legacy_model_sha256: sha256_serialized(legacy_model),
                    display_covariances: drawings,
                    automatic_conversion_blocker: Some(error.to_string()),
                },
            }),
            None => models.push(ProjectModelRecordV6 {
                model_id: model_id.clone(),
                payload: ProjectModelPayloadV6::LegacyEstimandUnspecified {
                    legacy_model: legacy_model.clone(),
                    legacy_model_sha256: sha256_serialized(legacy_model),
                    display_covariances: drawings,
                    automatic_conversion_blocker: None,
                },
            }),
        }
    }

    let historical_recipes = source
        .recipes
        .iter()
        .map(ImmutableHistoricalRecipeV6::from_recipe)
        .collect::<Vec<_>>();
    let historical_recipes_by_id = historical_recipes
        .iter()
        .map(|recipe| (recipe.recipe_id(), recipe))
        .collect::<BTreeMap<_, _>>();
    let historical_results = source
        .results
        .iter()
        .map(|result| {
            ImmutableHistoricalResultV6::from_result(
                result,
                &historical_recipes_by_id,
                source.source_archive_version >= 3,
            )
        })
        .collect();
    drop(historical_recipes_by_id);

    let document = ProjectArchiveDocumentV6 {
        schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        project_id: source.manifest.project_id,
        name: source.manifest.name.clone(),
        created_at: source.manifest.created_at,
        modified_at: request.upgraded_at,
        datasets: source
            .datasets
            .iter()
            .map(DatasetDescriptor::from)
            .collect(),
        models,
        recipes: Vec::new(),
        historical_recipes,
        layouts: source.layouts.clone(),
        historical_results,
        canonical_result_documents: Vec::new(),
        origin: ProjectOriginV6::UpgradedCopy {
            lineage: ProjectUpgradeLineageV6 {
                source_project_id: source.manifest.project_id,
                source_archive_schema_version: source.source_archive_version,
                source_archive_sha256: request.source_archive_sha256.clone(),
                source_archive_path: request.source_archive_path.clone(),
                destination_archive_path: request.destination_archive_path.clone(),
                upgraded_at: request.upgraded_at,
                source_preservation: SourcePreservationPolicyV6::Required,
                write_policy: UpgradeWritePolicyV6::NewArchiveOnly,
                historical_results_immutable: true,
            },
        },
        sem_generation: None,
    };
    let plan = ProjectArchiveUpgradePlanV6 {
        document,
        source_must_remain_unchanged: true,
        destination_must_be_new: true,
    };
    plan.ensure_valid()?;
    Ok(plan)
}

/// Adds one immutable canonical result attachment to a cloned schema-6
/// document. The historical result envelopes and source lineage are preserved
/// exactly. This is intentionally not called by the live schema-v5 writer.
pub fn attach_canonical_result_document_v2_v6(
    source: &ProjectArchiveDocumentV6,
    canonical_document: CanonicalResultDocumentV2,
) -> Result<ProjectArchiveDocumentV6, ProjectArchiveV6Error> {
    source.ensure_valid()?;
    let attachment = CanonicalResultDocumentAttachmentV2::from_document(canonical_document)?;
    attachment.ensure_valid(&source.project_id.to_string())?;
    if source
        .canonical_result_documents
        .iter()
        .any(|current| current.document_id() == attachment.document_id())
    {
        return Err(ProjectArchiveV6Error::DuplicateCanonicalResultDocumentId(
            attachment.document_id().to_owned(),
        ));
    }
    if source
        .canonical_result_documents
        .iter()
        .any(|current| current.run_id() == attachment.run_id())
    {
        return Err(ProjectArchiveV6Error::DuplicateCanonicalResultRunId(
            attachment.run_id().to_owned(),
        ));
    }

    let mut attached = source.clone();
    attached.canonical_result_documents.push(attachment);
    attached
        .canonical_result_documents
        .sort_by(|left, right| left.document_id().cmp(right.document_id()));
    attached.ensure_valid()?;
    Ok(attached)
}

/// Returns true only for authoring semantics introduced by the General SEM v1
/// program. Legacy single-interaction and single disjoint reflective-reflective
/// HOC models remain available to existing schema-6 projects exactly as before.
pub fn sem_model_requires_general_sem_v1(model: &SemModelV4) -> bool {
    let mut interaction_count = 0usize;
    let mut higher_order_count = 0usize;
    let mut has_interaction_v2 = false;
    let mut has_extended_higher_order = false;

    for term in &model.derived_terms {
        match term {
            SemDerivedTermV4::Interaction { .. } => interaction_count += 1,
            SemDerivedTermV4::InteractionV2 { .. } => {
                interaction_count += 1;
                has_interaction_v2 = true;
            }
            SemDerivedTermV4::HigherOrder {
                approach,
                measurement_type,
                ..
            } => {
                higher_order_count += 1;
                has_extended_higher_order |= !matches!(
                    (approach, measurement_type),
                    (
                        qpls_core::HigherOrderConstructionApproachV4::DisjointTwoStage,
                        qpls_core::HigherOrderMeasurementTypeV4::ReflectiveReflective
                    )
                );
            }
            SemDerivedTermV4::Polynomial { .. } => {}
        }
    }

    has_interaction_v2
        || interaction_count > 1
        || higher_order_count > 1
        || (interaction_count > 0 && higher_order_count > 0)
        || has_extended_higher_order
}

fn ensure_general_sem_v1_model_authority(
    project: &ProjectArchiveDocumentV6,
    model: &SemModelV4,
) -> Result<(), ProjectArchiveV6Error> {
    if sem_model_requires_general_sem_v1(model) && !project.supports_general_sem_v1() {
        return Err(ProjectArchiveV6Error::GeneralSemFeatureRequiresGeneration {
            subject: format!("SEM model {}", model.id),
        });
    }
    Ok(())
}

/// Inserts a new authoring-integrity-checked model as a non-executable draft in
/// a cloned schema-6 document. Existing records and every non-model field are
/// left untouched; callers must choose a new model identifier for a revision
/// rather than reuse an existing scientific authority.
pub fn insert_sem_model_v4_draft_v6(
    source: &ProjectArchiveDocumentV6,
    draft: SemModelV4,
) -> Result<ProjectArchiveDocumentV6, ProjectArchiveV6Error> {
    source.ensure_valid()?;
    draft.ensure_authoring_integrity()?;
    ensure_general_sem_v1_model_authority(source, &draft)?;
    if source
        .models
        .iter()
        .any(|record| record.model_id == draft.id)
    {
        return Err(ProjectArchiveV6Error::DuplicateOrEmptyModelId(
            draft.id.clone(),
        ));
    }
    ensure_model_mutation_unreferenced_v6(source, &draft.id)?;
    let model_document_sha256 = draft.model_document_sha256()?;

    let mut inserted = source.clone();
    inserted.models.push(ProjectModelRecordV6 {
        model_id: draft.id.clone(),
        payload: ProjectModelPayloadV6::SemModelV4Draft {
            model: draft,
            model_document_sha256,
        },
    });
    inserted.ensure_valid()?;
    Ok(inserted)
}

/// Replaces the exact, unreferenced authoring draft selected by its current
/// document digest. The model identifier is stable within this operation; a
/// referenced model must instead be revised under a new identifier.
pub fn replace_sem_model_v4_draft_v6(
    source: &ProjectArchiveDocumentV6,
    model_id: &str,
    expected_model_document_sha256: &str,
    replacement: SemModelV4,
) -> Result<ProjectArchiveDocumentV6, ProjectArchiveV6Error> {
    source.ensure_valid()?;
    let record_index = model_record_index_v6(source, model_id)?;
    ensure_model_mutation_unreferenced_v6(source, model_id)?;
    let ProjectModelPayloadV6::SemModelV4Draft {
        model_document_sha256: current_model_document_sha256,
        ..
    } = &source.models[record_index].payload
    else {
        return Err(ProjectArchiveV6Error::ModelMutationRequiresDraft(
            model_id.to_owned(),
        ));
    };
    ensure_current_model_document_digest_v6(
        model_id,
        expected_model_document_sha256,
        current_model_document_sha256,
    )?;
    replacement.ensure_authoring_integrity()?;
    ensure_general_sem_v1_model_authority(source, &replacement)?;
    if replacement.id != model_id {
        return Err(ProjectArchiveV6Error::ModelMutationIdentityMismatch {
            expected: model_id.to_owned(),
            observed: replacement.id,
        });
    }
    let model_document_sha256 = replacement.model_document_sha256()?;

    let mut replaced = source.clone();
    replaced.models[record_index].payload = ProjectModelPayloadV6::SemModelV4Draft {
        model: replacement,
        model_document_sha256,
    };
    replaced.ensure_valid()?;
    Ok(replaced)
}

/// Promotes the exact authoring draft selected by its current document digest
/// only when full SemModelV4 readiness validation succeeds. Recipes and result
/// attachments are never created or rewritten by promotion.
pub fn promote_sem_model_v4_draft_v6(
    source: &ProjectArchiveDocumentV6,
    model_id: &str,
    expected_model_document_sha256: &str,
) -> Result<ProjectArchiveDocumentV6, ProjectArchiveV6Error> {
    source.ensure_valid()?;
    let record_index = model_record_index_v6(source, model_id)?;
    ensure_model_mutation_unreferenced_v6(source, model_id)?;
    let ProjectModelPayloadV6::SemModelV4Draft {
        model,
        model_document_sha256: current_model_document_sha256,
    } = &source.models[record_index].payload
    else {
        return Err(ProjectArchiveV6Error::ModelMutationRequiresDraft(
            model_id.to_owned(),
        ));
    };
    ensure_current_model_document_digest_v6(
        model_id,
        expected_model_document_sha256,
        current_model_document_sha256,
    )?;
    model.ensure_valid()?;
    ensure_general_sem_v1_model_authority(source, model)?;
    let scientific_sha256 = model.scientific_sha256()?;

    let mut promoted = source.clone();
    promoted.models[record_index].payload = ProjectModelPayloadV6::SemModelV4 {
        model: model.clone(),
        scientific_sha256,
    };
    promoted.ensure_valid()?;
    Ok(promoted)
}

fn model_record_index_v6(
    source: &ProjectArchiveDocumentV6,
    model_id: &str,
) -> Result<usize, ProjectArchiveV6Error> {
    source
        .models
        .iter()
        .position(|record| record.model_id == model_id)
        .ok_or_else(|| ProjectArchiveV6Error::UnknownModel(model_id.to_owned()))
}

fn ensure_model_mutation_unreferenced_v6(
    source: &ProjectArchiveDocumentV6,
    model_id: &str,
) -> Result<(), ProjectArchiveV6Error> {
    let recipe_reference = source
        .recipes
        .iter()
        .any(|recipe| recipe.model_binding.model_id() == model_id);
    let result_reference = source
        .canonical_result_documents
        .iter()
        .any(|attachment| attachment.canonical_document().provenance.model_id == model_id);
    if recipe_reference || result_reference {
        return Err(ProjectArchiveV6Error::ModelMutationReferenced(
            model_id.to_owned(),
        ));
    }
    Ok(())
}

fn ensure_current_model_document_digest_v6(
    model_id: &str,
    expected: &str,
    current: &str,
) -> Result<(), ProjectArchiveV6Error> {
    if expected != current {
        return Err(ProjectArchiveV6Error::ModelDocumentDigestMismatch {
            model_id: model_id.to_owned(),
            expected: expected.to_owned(),
            current: current.to_owned(),
        });
    }
    Ok(())
}

/// Applies the one-time user confirmation to a copy of an archive document.
/// Historical results and migration lineage are cloned byte-for-byte.
pub fn confirm_project_legacy_estimand_v6(
    source: &ProjectArchiveDocumentV6,
    model_id: &str,
    interpretation: LegacyBasicModelInterpretationV4,
) -> Result<ProjectArchiveDocumentV6, ProjectArchiveV6Error> {
    source.ensure_valid()?;
    if interpretation == LegacyBasicModelInterpretationV4::Unspecified {
        return Err(ProjectArchiveV6Error::EstimandConfirmationRequired);
    }
    let Some(record_index) = source
        .models
        .iter()
        .position(|record| record.model_id == model_id)
    else {
        return Err(ProjectArchiveV6Error::UnknownModel(model_id.into()));
    };
    let ProjectModelPayloadV6::LegacyEstimandUnspecified {
        legacy_model,
        display_covariances,
        ..
    } = &source.models[record_index].payload
    else {
        return Err(ProjectArchiveV6Error::ModelAlreadyConfirmed(
            model_id.into(),
        ));
    };
    let model = convert_legacy_basic_model_v4(legacy_model, interpretation, display_covariances)?;
    let scientific_sha256 = model.scientific_sha256()?;
    let mut confirmed = source.clone();
    confirmed.models[record_index].payload = ProjectModelPayloadV6::SemModelV4 {
        model: model.clone(),
        scientific_sha256,
    };
    for recipe in &mut confirmed.recipes {
        if matches!(
            &recipe.model_binding,
            AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified {
                legacy_model_id,
                ..
            } if legacy_model_id == model_id
        ) {
            let (updated, _) = confirm_legacy_recipe_estimand_v4(
                recipe,
                legacy_model,
                display_covariances,
                interpretation,
            )?;
            *recipe = updated;
        }
    }
    confirmed.ensure_valid()?;
    Ok(confirmed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LegacySemIntentV6 {
    PlsComposite,
    CbsemCommonFactor,
    MethodNeutral,
}

fn legacy_sem_intent(method: AnalysisMethod) -> LegacySemIntentV6 {
    match method {
        AnalysisMethod::PlsPm
        | AnalysisMethod::Bootstrap
        | AnalysisMethod::Plsc
        | AnalysisMethod::Wpls
        | AnalysisMethod::Cca
        | AnalysisMethod::CtaPls
        | AnalysisMethod::Endogeneity
        | AnalysisMethod::NonlinearEffects
        | AnalysisMethod::ModeratedMediation
        | AnalysisMethod::Predict
        | AnalysisMethod::Mga
        | AnalysisMethod::Ipma
        | AnalysisMethod::Gsca
        | AnalysisMethod::Nca => LegacySemIntentV6::PlsComposite,
        AnalysisMethod::Cbsem => LegacySemIntentV6::CbsemCommonFactor,
        AnalysisMethod::PlsSampleSizePower
        | AnalysisMethod::Pca
        | AnalysisMethod::Regression
        | AnalysisMethod::Legacy => LegacySemIntentV6::MethodNeutral,
    }
}

fn unambiguous_interpretation(
    intents: Option<&BTreeSet<LegacySemIntentV6>>,
) -> Option<LegacyBasicModelInterpretationV4> {
    let intents = intents?;
    if intents.len() != 1 {
        return None;
    }
    match intents.first()? {
        LegacySemIntentV6::PlsComposite => Some(LegacyBasicModelInterpretationV4::PlsComposite),
        LegacySemIntentV6::CbsemCommonFactor => {
            Some(LegacyBasicModelInterpretationV4::CbsemCommonFactor)
        }
        LegacySemIntentV6::MethodNeutral => None,
    }
}

fn insert_legacy_model(
    models: &mut BTreeMap<String, ModelSpec>,
    candidate: &ModelSpec,
) -> Result<(), ProjectArchiveV6Error> {
    let id = candidate.id.to_string();
    if let Some(existing) = models.get(&id) {
        if existing != candidate {
            return Err(ProjectArchiveV6Error::ConflictingLegacyModel(id));
        }
    } else {
        models.insert(id, candidate.clone());
    }
    Ok(())
}

fn validate_display_covariances(
    model: &ModelSpec,
    drawings: &[LegacyDisplayCovarianceV4],
) -> Result<(), ProjectArchiveV6Error> {
    let construct_ids = model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut drawing_ids = BTreeSet::new();
    let mut endpoint_pairs = BTreeSet::new();
    for drawing in drawings {
        if drawing.id.trim().is_empty() || !drawing_ids.insert(drawing.id.as_str()) {
            return Err(ProjectArchiveV6Error::DuplicateDisplayCovarianceId(
                drawing.id.clone(),
            ));
        }
        if drawing.left_construct == drawing.right_construct
            || !construct_ids.contains(drawing.left_construct.as_str())
            || !construct_ids.contains(drawing.right_construct.as_str())
        {
            return Err(ProjectArchiveV6Error::InvalidDisplayCovariance(
                drawing.id.clone(),
            ));
        }
        let pair = if drawing.left_construct <= drawing.right_construct {
            (&drawing.left_construct, &drawing.right_construct)
        } else {
            (&drawing.right_construct, &drawing.left_construct)
        };
        if !endpoint_pairs.insert((pair.0.as_str(), pair.1.as_str())) {
            return Err(ProjectArchiveV6Error::DuplicateDisplayCovariancePair(
                drawing.id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_upgrade_lineage(
    lineage: &ProjectUpgradeLineageV6,
) -> Result<(), ProjectArchiveV6Error> {
    if !(1..=5).contains(&lineage.source_archive_schema_version) {
        return Err(ProjectArchiveV6Error::InvalidUpgradeSourceSchema(
            lineage.source_archive_schema_version,
        ));
    }
    validate_sha256("source_archive_sha256", &lineage.source_archive_sha256)?;
    validate_distinct_paths(
        &lineage.source_archive_path,
        &lineage.destination_archive_path,
    )?;
    if lineage.source_preservation != SourcePreservationPolicyV6::Required
        || lineage.write_policy != UpgradeWritePolicyV6::NewArchiveOnly
        || !lineage.historical_results_immutable
    {
        return Err(ProjectArchiveV6Error::UnsafeUpgradePolicy);
    }
    Ok(())
}

fn validate_distinct_paths(source: &str, destination: &str) -> Result<(), ProjectArchiveV6Error> {
    let normalize = |path: &str| {
        path.trim()
            .trim_end_matches(['\\', '/'])
            .replace('/', "\\")
            .to_ascii_lowercase()
    };
    let source = normalize(source);
    let destination = normalize(destination);
    if source.is_empty() || destination.is_empty() {
        return Err(ProjectArchiveV6Error::EmptyUpgradePath);
    }
    if source == destination {
        return Err(ProjectArchiveV6Error::DestinationMustBeNew);
    }
    Ok(())
}

fn validate_sha256(field: &'static str, value: &str) -> Result<(), ProjectArchiveV6Error> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(ProjectArchiveV6Error::InvalidSha256 { field })
    }
}

fn canonicalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(values) => {
            Value::Array(values.into_iter().map(canonicalize_json_value).collect())
        }
        Value::Object(object) => {
            let mut entries = object.into_iter().collect::<Vec<_>>();
            entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            let mut canonical = serde_json::Map::new();
            for (key, value) in entries {
                canonical.insert(key, canonicalize_json_value(value));
            }
            Value::Object(canonical)
        }
        scalar => scalar,
    }
}

fn write_project_document_v6_new_with_checks<BeforePublish, AfterPublish>(
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
    before_publish: BeforePublish,
    after_publish: AfterPublish,
) -> Result<ProjectArchiveWriteReceiptV6, ProjectArchiveV6Error>
where
    BeforePublish: FnOnce(&Path) -> Result<(), ProjectArchiveV6Error>,
    AfterPublish: FnOnce(&Path) -> Result<(), ProjectArchiveV6Error>,
{
    document.ensure_valid()?;
    if let Some(lineage) = document.upgrade_lineage() {
        ensure_lineage_path_binding(
            "destination_archive_path",
            destination,
            &lineage.destination_archive_path,
        )?;
    }
    ensure_destination_absent(destination)?;

    let bytes = serialize_project_document_v6(document)?;
    let document_sha256 = sha256_bytes(&bytes);
    let temporary = temporary_upgrade_path_v6(destination)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let mut temporary_guard = TemporaryProjectDocumentV6Guard::new(temporary.clone());
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);

    before_publish(&temporary)?;
    let temporary_bytes = fs::read(&temporary)?;
    if temporary_bytes != bytes {
        return Err(ProjectArchiveV6Error::TemporaryDocumentChanged);
    }
    let temporary_document = deserialize_project_document_v6(&temporary_bytes)?;
    if serialize_project_document_v6(&temporary_document)? != bytes {
        return Err(ProjectArchiveV6Error::TemporaryDocumentChanged);
    }

    atomic_publish_new_v6(&temporary, destination)?;
    temporary_guard.disarm();
    let mut destination_guard = NewProjectDocumentV6Guard::new(destination.to_path_buf());
    sync_parent_directory_v6(destination)?;
    after_publish(destination)?;

    let persisted_bytes = fs::read(destination)?;
    if persisted_bytes != bytes {
        return Err(ProjectArchiveV6Error::PostWriteValidation);
    }
    let persisted_document = deserialize_project_document_v6(&persisted_bytes)?;
    if serialize_project_document_v6(&persisted_document)? != bytes {
        return Err(ProjectArchiveV6Error::PostWriteValidation);
    }

    destination_guard.disarm();
    Ok(ProjectArchiveWriteReceiptV6 {
        schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        project_id: document.project_id,
        destination_archive_path: destination.to_string_lossy().into_owned(),
        document_sha256,
        byte_length: u64::try_from(bytes.len()).expect("document length must fit in u64"),
        post_write_validated: true,
    })
}

fn ensure_lineage_path_binding(
    field: &'static str,
    observed: &Path,
    expected: &str,
) -> Result<(), ProjectArchiveV6Error> {
    let observed = observed
        .to_str()
        .ok_or(ProjectArchiveV6Error::NonUnicodeUpgradePath { field })?;
    if observed != expected {
        return Err(ProjectArchiveV6Error::UpgradePathBinding {
            field,
            expected: expected.to_owned(),
            observed: observed.to_owned(),
        });
    }
    Ok(())
}

fn ensure_destination_absent(destination: &Path) -> Result<(), ProjectArchiveV6Error> {
    match fs::symlink_metadata(destination) {
        Ok(_) => Err(ProjectArchiveV6Error::DestinationExists(
            destination.to_path_buf(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn temporary_upgrade_path_v6(destination: &Path) -> Result<PathBuf, ProjectArchiveV6Error> {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(ProjectArchiveV6Error::DestinationFileName)?;
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(format!(".{file_name}.upgrade-v6-{}.tmp", Uuid::new_v4())))
}

fn append_private_path_v6(
    archive_path: &Path,
    suffix: &str,
) -> Result<PathBuf, ProjectArchiveV6Error> {
    let file_name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(ProjectArchiveV6Error::DestinationFileName)?;
    let parent = archive_path.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(format!(".{file_name}.append-v6-{suffix}")))
}

#[cfg(windows)]
fn wide_path_v6(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn atomic_replace_with_rollback_v6(
    replacement: &Path,
    destination: &Path,
    rollback: &Path,
) -> Result<(), ProjectArchiveV6Error> {
    ensure_destination_absent(rollback)?;
    #[cfg(windows)]
    {
        let destination = wide_path_v6(destination);
        let replacement = wide_path_v6(replacement);
        let rollback = wide_path_v6(rollback);
        // SAFETY: all three buffers are NUL-terminated and live for the call.
        let replaced = unsafe {
            ReplaceFileW(
                destination.as_ptr(),
                replacement.as_ptr(),
                rollback.as_ptr(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if replaced == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(not(windows))]
    {
        fs::hard_link(destination, rollback)?;
        if let Err(error) = fs::rename(replacement, destination) {
            let _ = fs::remove_file(rollback);
            return Err(error.into());
        }
    }
    Ok(())
}

fn restore_rollback_v6(rollback: &Path, destination: &Path) -> Result<(), ProjectArchiveV6Error> {
    #[cfg(windows)]
    {
        let destination = wide_path_v6(destination);
        let rollback = wide_path_v6(rollback);
        // SAFETY: both path buffers are NUL-terminated and live for the call.
        let restored = unsafe {
            ReplaceFileW(
                destination.as_ptr(),
                rollback.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if restored == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(not(windows))]
    fs::rename(rollback, destination)?;
    sync_parent_directory_v6(destination)
}

fn atomic_publish_new_v6(
    temporary: &Path,
    destination: &Path,
) -> Result<(), ProjectArchiveV6Error> {
    // Rename can replace an existing file on supported Windows and POSIX
    // implementations. A same-directory hard link is an atomic no-clobber
    // publication primitive on both platforms. Removing the private temp name
    // afterward leaves the fsynced inode at the destination.
    fs::hard_link(temporary, destination).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists
            || fs::symlink_metadata(destination).is_ok()
        {
            ProjectArchiveV6Error::DestinationExists(destination.to_path_buf())
        } else {
            ProjectArchiveV6Error::Io(error)
        }
    })?;
    if let Err(error) = fs::remove_file(temporary) {
        let _ = fs::remove_file(destination);
        return Err(error.into());
    }
    Ok(())
}

fn sync_parent_directory_v6(path: &Path) -> Result<(), ProjectArchiveV6Error> {
    #[cfg(not(windows))]
    {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        File::open(parent)?.sync_all()?;
    }
    #[cfg(windows)]
    {
        let _ = path;
    }
    Ok(())
}

fn sha256_file_v6(path: &Path) -> Result<String, ProjectArchiveV6Error> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

struct TemporaryProjectDocumentV6Guard {
    path: Option<PathBuf>,
}

impl TemporaryProjectDocumentV6Guard {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for TemporaryProjectDocumentV6Guard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

struct NewProjectDocumentV6Guard {
    path: Option<PathBuf>,
}

impl NewProjectDocumentV6Guard {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for NewProjectDocumentV6Guard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

fn sha256_json(value: &Value) -> String {
    let canonical = canonicalize_json_value(value.clone());
    sha256_bytes(&serde_json::to_vec(&canonical).expect("JSON values must serialize"))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectArchiveV6Error {
    #[error("project archive schema version 0 is unsupported")]
    UnsupportedSchemaZero,
    #[error("project archive v6 requires schema_version 6 (found {0})")]
    Schema(u32),
    #[error("future project archive schema {0} is read-only and cannot be upgraded by this build")]
    FutureSourceReadOnly(u32),
    #[error("upgrade source archive schema {0} must be historical schema 1 through 5")]
    InvalidUpgradeSourceSchema(u32),
    #[error("{field} must be a lowercase 64-character SHA-256 digest")]
    InvalidSha256 { field: &'static str },
    #[error("source and destination archive paths must both be provided")]
    EmptyUpgradePath,
    #[error("an upgraded project must be written to a new path; the source archive is immutable")]
    DestinationMustBeNew,
    #[error("archive v6 upgrade policy must preserve the source and write only a new archive")]
    UnsafeUpgradePolicy,
    #[error("an upgrade-copy plan requires project origin upgraded_copy")]
    UpgradeOriginRequired,
    #[error("upgraded_copy origin source_project_id must match project_id")]
    UpgradeProjectIdentity,
    #[error(
        "general_sem_v1 generation authority is valid only for a newly created schema-v6 project"
    )]
    GeneralSemGenerationRequiresNewProject,
    #[error(
        "{subject} uses General SEM v1 features; create a new schema-6 project with general_sem_v1 authority"
    )]
    GeneralSemFeatureRequiresGeneration { subject: String },
    #[error("schema-v6 General SEM canonical authority is invalid: {0}")]
    CanonicalGeneralSemAuthority(String),
    #[error("legacy model {0} has conflicting scientific content")]
    ConflictingLegacyModel(String),
    #[error("display covariance input references unknown model {0}")]
    UnknownDisplayCovarianceModel(String),
    #[error("display covariance {0} has an empty or duplicate identifier")]
    DuplicateDisplayCovarianceId(String),
    #[error("display covariance {0} must connect two distinct known legacy constructs")]
    InvalidDisplayCovariance(String),
    #[error("display covariance {0} duplicates an existing endpoint pair")]
    DuplicateDisplayCovariancePair(String),
    #[error("project model identifier {0} is empty or duplicated")]
    DuplicateOrEmptyModelId(String),
    #[error("project model {0} has a mismatched identity or digest")]
    ModelDigestOrIdentity(String),
    #[error("model mutation target {0} must be an authoring SemModelV4 draft")]
    ModelMutationRequiresDraft(String),
    #[error(
        "model {0} is referenced by current RecipeV4 or canonical-result authority; create a new model identifier/revision"
    )]
    ModelMutationReferenced(String),
    #[error(
        "replacement model identity differs from its target (expected {expected}, observed {observed})"
    )]
    ModelMutationIdentityMismatch { expected: String, observed: String },
    #[error("model {model_id} document digest is stale (expected {expected}, current {current})")]
    ModelDocumentDigestMismatch {
        model_id: String,
        expected: String,
        current: String,
    },
    #[error("analysis recipe {recipe_id} references unavailable model {model_id}")]
    RecipeModelReference { recipe_id: Uuid, model_id: String },
    #[error("analysis recipe {0} model digest differs from its project model")]
    RecipeModelDigest(Uuid),
    #[error("analysis recipe identifier {0} is duplicated")]
    DuplicateRecipeId(Uuid),
    #[error("historical analysis recipe {0} identity differs from its immutable envelope")]
    HistoricalRecipeIdentity(Uuid),
    #[error("historical analysis recipe {0} schema is not an exact schema-1-through-3 document")]
    HistoricalRecipeSchema(Uuid),
    #[error("historical analysis recipe {0} digest differs from its immutable envelope")]
    HistoricalRecipeDigest(Uuid),
    #[error("historical result {0} identity differs from its immutable envelope")]
    HistoricalResultIdentity(Uuid),
    #[error("historical result {0} schema differs from its immutable envelope")]
    HistoricalResultSchema(Uuid),
    #[error("historical result {0} digest differs from its immutable envelope")]
    HistoricalResultDigest(Uuid),
    #[error("historical result {0} has a missing, invented, or mismatched source recipe binding")]
    HistoricalResultRecipeBinding(Uuid),
    #[error("historical result identifier {0} is duplicated")]
    DuplicateHistoricalResultId(Uuid),
    #[error("canonical result document identifier {0} is duplicated")]
    DuplicateCanonicalResultDocumentId(String),
    #[error("canonical result run identifier {0} is duplicated")]
    DuplicateCanonicalResultRunId(String),
    #[error("Recipe-v4 PLS canonical result references unavailable recipe {0}")]
    CanonicalPlsRecipeUnavailable(String),
    #[error("Recipe-v4 PLS canonical result references unavailable explicit SemModelV4 {0}")]
    CanonicalPlsModelUnavailable(String),
    #[error("Recipe-v4 PLS canonical result references unavailable dataset {0}")]
    CanonicalPlsDatasetUnavailable(String),
    #[error("Recipe-v4 PLS nonlinear canonical contract is invalid: {0}")]
    CanonicalPlsNonlinear(String),
    #[error("Recipe-v4 PLS canonical result has unsupported method identity {0}")]
    CanonicalPlsMethodUnsupported(String),
    #[error("plain pls_pm_v1 canonical result has versioned score-execution adapter {0}")]
    CanonicalPlsAdapterMismatch(String),
    #[error("Recipe-v4 CB-SEM canonical result references unavailable recipe {0}")]
    CanonicalCbsemRecipeUnavailable(String),
    #[error("Recipe-v4 CB-SEM canonical result references unavailable explicit SemModelV4 {0}")]
    CanonicalCbsemModelUnavailable(String),
    #[error("Recipe-v4 CB-SEM canonical result references unavailable dataset descriptor {0}")]
    CanonicalCbsemDatasetUnavailable(String),
    #[error("Recipe-v4 CB-SEM canonical RMSEA fit contract is invalid: {0}")]
    CanonicalCbsemRmseaFit(String),
    #[error("future project document must be a JSON object")]
    FutureDocumentNotObject,
    #[error("project document has no valid embedded schema_version")]
    InvalidEmbeddedSchemaVersion,
    #[error("{field} is not valid Unicode and cannot be bound to upgrade lineage")]
    NonUnicodeUpgradePath { field: &'static str },
    #[error("{field} differs from upgrade lineage (expected {expected}, observed {observed})")]
    UpgradePathBinding {
        field: &'static str,
        expected: String,
        observed: String,
    },
    #[error("upgrade destination already exists: {0}")]
    DestinationExists(PathBuf),
    #[error("upgrade destination must have a valid file name")]
    DestinationFileName,
    #[error(
        "source archive digest differs from upgrade lineage (expected {expected}, observed {observed})"
    )]
    SourceDigestMismatch { expected: String, observed: String },
    #[error("source archive changed while its upgraded copy was being written")]
    SourceChangedDuringUpgrade,
    #[error("temporary schema-v6 document changed before atomic publication")]
    TemporaryDocumentChanged,
    #[error("persisted schema-v6 document failed exact post-write validation")]
    PostWriteValidation,
    #[error("schema-v6 result append requires a regular, non-symlink archive file: {0}")]
    AppendArchiveMustBeRegularFile(PathBuf),
    #[error("another schema-v6 append is already active for {0}")]
    AppendAlreadyInProgress(PathBuf),
    #[error("schema-v6 append was cancelled before commit")]
    AppendCancelled,
    #[error("schema-v6 source changed before or during append commit")]
    SourceChangedDuringAppend,
    #[error(
        "schema-v6 append failed and rollback could not be restored (original: {original}; rollback: {rollback})"
    )]
    AppendRollbackFailed { original: String, rollback: String },
    #[error("model {0} does not exist")]
    UnknownModel(String),
    #[error("model {0} already has explicit SemModelV4 estimand semantics")]
    ModelAlreadyConfirmed(String),
    #[error("factor-versus-composite confirmation is required")]
    EstimandConfirmationRequired,
    #[error(transparent)]
    Recipe(#[from] AnalysisRecipeV4Error),
    #[error(transparent)]
    LegacyConversion(#[from] LegacyBasicModelConversionErrorV4),
    #[error(transparent)]
    InvalidSemModel(#[from] SemModelV4ValidationError),
    #[error(transparent)]
    CanonicalResultDocument(#[from] CanonicalResultDocumentV2Error),
    #[error(transparent)]
    CanonicalPlsScoreExecution(#[from] PlsScoreExecutionDocumentV2Error),
    #[error(transparent)]
    CanonicalCbsemMissingData(#[from] MissingDataExecutionDocumentV1Error),
    #[error(transparent)]
    DataLineage(#[from] ProjectDataLineageV1Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("legacy archive integrity layer rejected the document: {0}")]
    LegacyArchiveLayer(ProjectError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CanonicalChartDisplayOptionsV2, CanonicalChartKindV2, CanonicalChartPointV2,
        CanonicalChartSeriesV2, CanonicalChartXValueV2, CanonicalColumnRoleV2,
        CanonicalColumnTypeV2, CanonicalResultChartV2, CanonicalResultColumnV2,
        CanonicalResultPresentationV2, CanonicalResultProvenanceV2, CanonicalResultRowV2,
        CanonicalResultSectionV2,
    };
    use chrono::TimeZone;
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisPayload, AnalysisRecipe, AnalysisSettings,
        Construct, FactorIdentificationV4, GeneralSemConfigV1, InteractionHierarchyPolicyV2,
        InteractionMethodV4, MeasurementMode, MethodConfig, ObservedRoleV4, ObservedScaleV4,
        RESULT_SCHEMA_VERSION, RunProvenance, RunStatus, SamplingWeightNormalizationV4,
        SemDataBindingV4, SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4,
        SemWeightBindingV4, StructuralPath, StructuralRelationRoleV4,
        migrate_analysis_recipe_to_v4_pending, resolve_weight_declaration_v1,
    };

    #[test]
    fn studentized_schema6_workload_caps_match_the_v11_labs_envelope() {
        assert!(validate_cbsem_studentized_archive_workload_v1(500, 12, 180, 9, 18, 18).is_ok());
        assert!(validate_cbsem_studentized_archive_workload_v1(10_000, 1, 10, 1, 1, 1).is_ok());
        for outside in [
            validate_cbsem_studentized_archive_workload_v1(499, 12, 180, 9, 18, 18),
            validate_cbsem_studentized_archive_workload_v1(10_001, 12, 180, 9, 18, 18),
            validate_cbsem_studentized_archive_workload_v1(500, 0, 180, 9, 18, 18),
            validate_cbsem_studentized_archive_workload_v1(500, 13, 180, 9, 18, 18),
            validate_cbsem_studentized_archive_workload_v1(500, 12, 181, 9, 18, 18),
            validate_cbsem_studentized_archive_workload_v1(500, 12, 180, 10, 18, 18),
            validate_cbsem_studentized_archive_workload_v1(500, 12, 180, 9, 19, 18),
            validate_cbsem_studentized_archive_workload_v1(500, 12, 180, 9, 18, 19),
        ] {
            assert!(matches!(
                outside,
                Err(ProjectArchiveV6Error::CanonicalCbsemRmseaFit(message))
                    if message.contains("fail-closed Labs workload envelope")
            ));
        }
    }

    fn nonlinear_test_column(
        id: &str,
        data_type: CanonicalColumnTypeV2,
    ) -> CanonicalResultColumnV2 {
        CanonicalResultColumnV2 {
            id: id.into(),
            label: id.into(),
            data_type,
            description: format!("{id} test column"),
            role: Some(match data_type {
                CanonicalColumnTypeV2::Text => CanonicalColumnRoleV2::Label,
                CanonicalColumnTypeV2::Number => CanonicalColumnRoleV2::Estimate,
                CanonicalColumnTypeV2::Boolean => CanonicalColumnRoleV2::Diagnostic,
            }),
            unit: None,
            default_precision: None,
        }
    }

    fn nonlinear_test_text(value: impl Into<String>) -> CanonicalResultCellV2 {
        CanonicalResultCellV2::Text {
            value: value.into(),
        }
    }

    fn nonlinear_test_number(value: f64) -> CanonicalResultCellV2 {
        CanonicalResultCellV2::Number {
            value,
            display: None,
        }
    }

    fn nonlinear_test_table(
        id: &str,
        columns: Vec<CanonicalResultColumnV2>,
        rows: Vec<CanonicalResultRowV2>,
        owner: crate::CapabilityCellReferenceV2,
    ) -> CanonicalResultTableV2 {
        CanonicalResultTableV2 {
            id: id.into(),
            title: id.into(),
            description: Some(format!("{id} test table")),
            columns,
            rows,
            footnote_ids: Vec::new(),
            capability_cells: Some(vec![owner]),
        }
    }

    fn nonlinear_schema6_fixture() -> (ProjectArchiveDocumentV6, CanonicalResultDocumentV2) {
        let dataset = qpls_data::import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let legacy: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        let mut source = legacy.migrated_v3().unwrap();
        source.dataset_fingerprint = dataset.fingerprint.0.clone();
        source.settings.workers = 1;
        source.settings.seed = 20_260_816;
        source.method_config = Some(MethodConfig::PlsAlgorithm);
        let pending = migrate_analysis_recipe_to_v4_pending(&source).unwrap();
        let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source.model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *dataset_id = dataset.id.to_string();
        recipe.settings.method = AnalysisMethod::NonlinearEffects;
        recipe.method_config = Some(MethodConfig::NonlinearEffects);
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        recipe.ensure_valid().unwrap();

        let nonlinear = recipe_v4_pls_nonlinear_capability_cell_v1();
        let base = recipe_v4_pls_base_capability_cell_v1();
        let target = RecipeV4CompilerTarget::PlsPlanV2;
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            target,
            target.capability_cell_for_method(AnalysisMethod::NonlinearEffects),
        )
        .unwrap();
        let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
            unreachable!()
        };
        assert_eq!(plan.paths().len(), 1);
        let path = &plan.paths()[0];
        let source_id = path.source().to_owned();
        let target_id = path.target().to_owned();

        let mut project = Project::new("PLS nonlinear schema-v6 fixture");
        project.datasets.push(dataset.clone());
        crate::write_project_data_lineage_v1(
            &mut project.layouts,
            &crate::ProjectDataLineageV1 {
                schema_version: crate::PROJECT_DATA_LINEAGE_SCHEMA_VERSION_V1,
                records: vec![crate::ProjectDatasetVersionRecordV1 {
                    dataset_id: dataset.id.to_string(),
                    parent_dataset_id: None,
                    operation: crate::ProjectDatasetVersionOperationV1::Import,
                    created_at: None,
                    summary: "Imported nonlinear fixture".into(),
                    source_column: None,
                    target_column: None,
                    transformation: None,
                }],
            },
        )
        .unwrap();
        let mut upgrade_request = request();
        upgrade_request.legacy_display_covariances.clear();
        let mut schema6 = plan_project_upgrade_to_v6(&project, &upgrade_request)
            .unwrap()
            .document;
        schema6.recipes.push(recipe.clone());
        schema6.ensure_valid().unwrap();

        let text_columns = |ids: &[&str]| {
            ids.iter()
                .map(|id| nonlinear_test_column(id, CanonicalColumnTypeV2::Text))
                .collect::<Vec<_>>()
        };
        let mixed_columns = |columns: &[(&str, CanonicalColumnTypeV2)]| {
            columns
                .iter()
                .map(|(id, kind)| nonlinear_test_column(id, *kind))
                .collect::<Vec<_>>()
        };
        let estimation_summary = nonlinear_test_table(
            "estimation_summary",
            mixed_columns(&[
                ("converged", CanonicalColumnTypeV2::Boolean),
                ("iterations", CanonicalColumnTypeV2::Number),
                ("used_observations", CanonicalColumnTypeV2::Number),
                ("omitted_observations", CanonicalColumnTypeV2::Number),
            ]),
            vec![CanonicalResultRowV2 {
                id: "run".into(),
                cells: vec![
                    CanonicalResultCellV2::Boolean { value: true },
                    nonlinear_test_number(1.0),
                    nonlinear_test_number(8.0),
                    nonlinear_test_number(0.0),
                ],
            }],
            base.clone(),
        );
        let outer_model = nonlinear_test_table(
            "outer_model",
            mixed_columns(&[
                ("construct", CanonicalColumnTypeV2::Text),
                ("indicator", CanonicalColumnTypeV2::Text),
                ("weight", CanonicalColumnTypeV2::Number),
                ("loading", CanonicalColumnTypeV2::Number),
            ]),
            plan.blocks()
                .iter()
                .flat_map(|block| {
                    block
                        .indicators()
                        .iter()
                        .map(move |indicator| (block.construct_id(), indicator.source_column()))
                })
                .enumerate()
                .map(|(index, (construct, indicator))| CanonicalResultRowV2 {
                    id: format!("outer_{index:04}"),
                    cells: vec![
                        nonlinear_test_text(construct),
                        nonlinear_test_text(indicator),
                        nonlinear_test_number(0.5),
                        nonlinear_test_number(0.7),
                    ],
                })
                .collect(),
            base.clone(),
        );
        let linear_coefficient = 0.4;
        let structural_paths = nonlinear_test_table(
            "structural_paths",
            mixed_columns(&[
                ("source", CanonicalColumnTypeV2::Text),
                ("target", CanonicalColumnTypeV2::Text),
                ("coefficient", CanonicalColumnTypeV2::Number),
            ]),
            vec![CanonicalResultRowV2 {
                id: "path_0000".into(),
                cells: vec![
                    nonlinear_test_text(&source_id),
                    nonlinear_test_text(&target_id),
                    nonlinear_test_number(linear_coefficient),
                ],
            }],
            base.clone(),
        );
        let effects = nonlinear_test_table(
            "effects",
            mixed_columns(&[
                ("source", CanonicalColumnTypeV2::Text),
                ("target", CanonicalColumnTypeV2::Text),
                ("direct", CanonicalColumnTypeV2::Number),
                ("indirect", CanonicalColumnTypeV2::Number),
                ("total", CanonicalColumnTypeV2::Number),
            ]),
            vec![CanonicalResultRowV2 {
                id: "effect_0000".into(),
                cells: vec![
                    nonlinear_test_text(&source_id),
                    nonlinear_test_text(&target_id),
                    nonlinear_test_number(linear_coefficient),
                    nonlinear_test_number(0.0),
                    nonlinear_test_number(linear_coefficient),
                ],
            }],
            base.clone(),
        );
        let r_squared = nonlinear_test_table(
            "r_squared",
            mixed_columns(&[
                ("construct", CanonicalColumnTypeV2::Text),
                ("r_squared", CanonicalColumnTypeV2::Number),
            ]),
            vec![CanonicalResultRowV2 {
                id: "r_squared_0000".into(),
                cells: vec![nonlinear_test_text(&target_id), nonlinear_test_number(0.3)],
            }],
            base.clone(),
        );
        let attribution = qpls_estimation::PlsPointEstimateAttributionV1::for_preprocessing(
            recipe.settings.preprocessing.clone(),
        );
        let point_attribution = nonlinear_test_table(
            crate::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1,
            text_columns(&[
                "contract_version",
                "preprocessing",
                "indicator_centering",
                "indicator_scaling",
                "outer_weights",
                "outer_loadings",
                "construct_scores",
                "structural_paths",
                "effects",
            ]),
            vec![CanonicalResultRowV2 {
                id: "attribution".into(),
                cells: vec![
                    nonlinear_test_text(
                        qpls_estimation::PLS_POINT_ESTIMATE_ATTRIBUTION_CONTRACT_VERSION_V1,
                    ),
                    nonlinear_test_text(match recipe.settings.preprocessing {
                        qpls_core::Preprocessing::Standardized => "standardized",
                        qpls_core::Preprocessing::MeanCentered => "mean_centered",
                        qpls_core::Preprocessing::Unstandardized => "unstandardized",
                    }),
                    nonlinear_test_text(attribution.indicator_centering.as_str()),
                    nonlinear_test_text(attribution.indicator_scaling.as_str()),
                    nonlinear_test_text(attribution.outer_weights.as_str()),
                    nonlinear_test_text(attribution.outer_loadings.as_str()),
                    nonlinear_test_text(attribution.construct_scores.as_str()),
                    nonlinear_test_text(attribution.structural_paths.as_str()),
                    nonlinear_test_text(attribution.effects.as_str()),
                ],
            }],
            base.clone(),
        );
        let quadratic = 0.1;
        let standard_error = 0.2;
        let diagnostics = nonlinear_test_table(
            PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1,
            mixed_columns(&[
                ("source", CanonicalColumnTypeV2::Text),
                ("target", CanonicalColumnTypeV2::Text),
                ("linear_coefficient", CanonicalColumnTypeV2::Number),
                ("quadratic_coefficient", CanonicalColumnTypeV2::Number),
                ("standard_error", CanonicalColumnTypeV2::Number),
                ("t_statistic", CanonicalColumnTypeV2::Number),
                ("p_value_two_sided", CanonicalColumnTypeV2::Number),
                ("warning", CanonicalColumnTypeV2::Text),
            ]),
            vec![CanonicalResultRowV2 {
                id: "nonlinear_quadratic_diagnostic_0000".into(),
                cells: vec![
                    nonlinear_test_text(&source_id),
                    nonlinear_test_text(&target_id),
                    nonlinear_test_number(linear_coefficient),
                    nonlinear_test_number(quadratic),
                    nonlinear_test_number(standard_error),
                    nonlinear_test_number(quadratic / standard_error),
                    nonlinear_test_number(0.6),
                    CanonicalResultCellV2::Missing {
                        reason: CanonicalMissingReasonV2::NotEstimated,
                        display: None,
                    },
                ],
            }],
            nonlinear.clone(),
        );
        let linear_r_squared: f64 = 0.3;
        let augmented_r_squared: f64 = 0.35;
        let equation_fit = nonlinear_test_table(
            PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1,
            mixed_columns(&[
                ("target", CanonicalColumnTypeV2::Text),
                ("linear_r_squared", CanonicalColumnTypeV2::Number),
                ("augmented_r_squared", CanonicalColumnTypeV2::Number),
                ("delta_r_squared", CanonicalColumnTypeV2::Number),
            ]),
            vec![CanonicalResultRowV2 {
                id: "nonlinear_equation_fit_0000".into(),
                cells: vec![
                    nonlinear_test_text(&target_id),
                    nonlinear_test_number(linear_r_squared),
                    nonlinear_test_number(augmented_r_squared),
                    nonlinear_test_number((augmented_r_squared - linear_r_squared).max(0.0_f64)),
                ],
            }],
            nonlinear.clone(),
        );
        let method_scope = nonlinear_test_table(
            PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1,
            text_columns(PLS_NONLINEAR_METHOD_SCOPE_COLUMNS_V1),
            vec![CanonicalResultRowV2 {
                id: PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1.into(),
                cells: vec![
                    nonlinear_test_text(NONLINEAR_EFFECTS_METHOD_VERSION),
                    nonlinear_test_text(PLS_NONLINEAR_TERM_V1),
                    nonlinear_test_text(PLS_NONLINEAR_ENGINE_WARNING_V1),
                ],
            }],
            nonlinear.clone(),
        );
        let receipt = artifact.receipt();
        let canonical = CanonicalResultDocumentV2 {
            schema_version: 2,
            document_id: "result_pls_nonlinear_schema6".into(),
            title: "PLS nonlinear quadratic diagnostics".into(),
            provenance: CanonicalResultProvenanceV2 {
                run_id: "run_pls_nonlinear_schema6".into(),
                project_id: schema6.project_id.to_string(),
                model_id: receipt.model_id().into(),
                model_digest: receipt.model_scientific_sha256().into(),
                dataset_id: dataset.id.to_string(),
                dataset_fingerprint: recorded_dataset_sha256(receipt.dataset_fingerprint())
                    .unwrap()
                    .into(),
                recipe_id: receipt.recipe_id().to_string(),
                recipe_digest: receipt.recipe_analytical_sha256().into(),
                capability_cell: nonlinear.clone(),
                method_version: NONLINEAR_EFFECTS_METHOD_VERSION.into(),
                engine_version: RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7.into(),
                seed: Some(recipe.settings.seed),
                workers: recipe.settings.workers as u32,
                started_at: "2026-08-16T03:00:00Z".into(),
                completed_at: "2026-08-16T03:00:01Z".into(),
            },
            capability_cells: Some(vec![nonlinear.clone(), base.clone()]),
            general_sem_results: None,
            sections: vec![
                CanonicalResultSectionV2 {
                    id: "run_details".into(),
                    title: "Run details".into(),
                    description: None,
                    table_ids: vec![
                        "estimation_summary".into(),
                        crate::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1.into(),
                    ],
                    chart_ids: Vec::new(),
                    capability_cells: Some(vec![base.clone()]),
                },
                CanonicalResultSectionV2 {
                    id: "measurement_model".into(),
                    title: "Measurement model".into(),
                    description: None,
                    table_ids: vec!["outer_model".into()],
                    chart_ids: Vec::new(),
                    capability_cells: Some(vec![base.clone()]),
                },
                CanonicalResultSectionV2 {
                    id: "structural_model".into(),
                    title: "Structural model".into(),
                    description: None,
                    table_ids: vec![
                        "structural_paths".into(),
                        "effects".into(),
                        "r_squared".into(),
                    ],
                    chart_ids: Vec::new(),
                    capability_cells: Some(vec![base.clone()]),
                },
                CanonicalResultSectionV2 {
                    id: PLS_NONLINEAR_SECTION_ID_V1.into(),
                    title: "Nonlinear relationships".into(),
                    description: None,
                    table_ids: vec![
                        PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1.into(),
                        PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1.into(),
                        PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1.into(),
                    ],
                    chart_ids: Vec::new(),
                    capability_cells: Some(vec![nonlinear.clone()]),
                },
            ],
            tables: vec![
                estimation_summary,
                outer_model,
                structural_paths,
                effects,
                r_squared,
                point_attribution,
                diagnostics,
                equation_fit,
                method_scope,
            ],
            charts: Vec::new(),
            notices: Vec::new(),
            exclusions: Vec::new(),
            footnotes: Vec::new(),
            presentation: CanonicalResultPresentationV2 {
                default_section_id: Some(PLS_NONLINEAR_SECTION_ID_V1.into()),
                default_table_id: Some(PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1.into()),
                precision: 4,
                missing_value_label: "—".into(),
                chart_defaults: CanonicalChartDisplayOptionsV2::default(),
            },
        };
        canonical.ensure_valid().unwrap();
        (schema6, canonical)
    }

    #[test]
    fn pls_nonlinear_v7_appends_reopens_and_rejects_identity_and_numeric_tamper() {
        assert_eq!(
            RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7,
            qpls_runner::RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        );
        let (schema6, canonical) = nonlinear_schema6_fixture();
        assert_eq!(
            canonical.sections[0].table_ids,
            vec![
                "estimation_summary".to_string(),
                crate::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1.to_string(),
            ]
        );
        assert!(canonical.tables.iter().all(|table| {
            !matches!(
                table.id.as_str(),
                crate::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1
                    | crate::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1
            )
        }));

        let mut zero_delta_rounding = canonical.clone();
        let equation = zero_delta_rounding
            .tables
            .iter_mut()
            .find(|table| table.id == PLS_NONLINEAR_EQUATION_FIT_TABLE_ID_V1)
            .unwrap();
        let linear = 0.3_f64;
        let augmented = f64::from_bits(linear.to_bits() - 1);
        equation.rows[0].cells[1] = nonlinear_test_number(linear);
        equation.rows[0].cells[2] = nonlinear_test_number(augmented);
        equation.rows[0].cells[3] = nonlinear_test_number(0.0);
        assert!(attach_canonical_result_document_v2_v6(&schema6, zero_delta_rounding).is_ok());

        let attached = attach_canonical_result_document_v2_v6(&schema6, canonical.clone()).unwrap();
        let bytes = serialize_project_document_v6(&attached).unwrap();
        let reopened = deserialize_project_document_v6(&bytes).unwrap();
        assert_eq!(serialize_project_document_v6(&reopened).unwrap(), bytes);
        assert_eq!(
            crate::canonical_result_document_v2_json(
                reopened.canonical_result_documents[0].canonical_document()
            )
            .unwrap(),
            crate::canonical_result_document_v2_json(&canonical).unwrap()
        );

        let mut old_adapter = canonical.clone();
        old_adapter.provenance.engine_version =
            crate::RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION_V1.into();
        assert!(attach_canonical_result_document_v2_v6(&schema6, old_adapter).is_err());

        let mut base_primary = canonical.clone();
        base_primary.provenance.capability_cell = recipe_v4_pls_base_capability_cell_v1();
        base_primary.provenance.method_version = PLS_METHOD_VERSION.into();
        assert!(attach_canonical_result_document_v2_v6(&schema6, base_primary).is_err());

        let mut unknown_primary = canonical.clone();
        let unknown_cell = crate::CapabilityCellReferenceV2 {
            registry_schema_version: 2,
            capability_id: "smartpls.unknown".into(),
            cell_id: "qpls3.unknown".into(),
            capability_version: "unknown_v1".into(),
        };
        unknown_primary.provenance.capability_cell = unknown_cell.clone();
        unknown_primary
            .capability_cells
            .as_mut()
            .unwrap()
            .push(unknown_cell.clone());
        unknown_primary.ensure_valid().unwrap();
        assert!(attach_canonical_result_document_v2_v6(&schema6, unknown_primary).is_err());

        let mut unrelated_primary = canonical.clone();
        unrelated_primary.sections.pop();
        unrelated_primary
            .tables
            .truncate(unrelated_primary.tables.len() - 3);
        unrelated_primary.provenance.capability_cell = unknown_cell.clone();
        unrelated_primary.provenance.method_version = "unrelated_method_v1".into();
        unrelated_primary.provenance.engine_version = "unrelated_adapter_v1".into();
        unrelated_primary.capability_cells =
            Some(vec![recipe_v4_pls_base_capability_cell_v1(), unknown_cell]);
        unrelated_primary.presentation.default_section_id = Some("structural_model".into());
        unrelated_primary.presentation.default_table_id = Some("structural_paths".into());
        unrelated_primary.ensure_valid().unwrap();
        assert!(
            attach_canonical_result_document_v2_v6(&schema6, unrelated_primary.clone()).is_ok()
        );

        let mut method_only = unrelated_primary.clone();
        method_only.provenance.method_version = NONLINEAR_EFFECTS_METHOD_VERSION.into();
        assert!(attach_canonical_result_document_v2_v6(&schema6, method_only).is_err());

        let mut adapter_only = unrelated_primary;
        adapter_only.provenance.engine_version =
            RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7.into();
        assert!(attach_canonical_result_document_v2_v6(&schema6, adapter_only).is_err());

        let injected_chart = CanonicalResultChartV2 {
            id: "nonlinear_chart_injection".into(),
            title: "Injected nonlinear chart".into(),
            description: "Chart output is outside the exact nonlinear v7 surface.".into(),
            kind: CanonicalChartKindV2::Scatter,
            series: vec![CanonicalChartSeriesV2 {
                id: "injected_series".into(),
                label: "Injected series".into(),
                group: None,
                points: vec![CanonicalChartPointV2 {
                    x: CanonicalChartXValueV2::Text("injected".into()),
                    y: 0.0,
                    lower: None,
                    upper: None,
                    label: None,
                }],
            }],
            source_table_id: Some(PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1.into()),
            display: CanonicalChartDisplayOptionsV2::default(),
        };
        let mut unreferenced_chart = canonical.clone();
        unreferenced_chart.charts.push(injected_chart.clone());
        unreferenced_chart.ensure_valid().unwrap();
        assert!(attach_canonical_result_document_v2_v6(&schema6, unreferenced_chart).is_err());

        let mut referenced_chart = canonical.clone();
        referenced_chart.charts.push(injected_chart);
        referenced_chart.sections[3]
            .chart_ids
            .push("nonlinear_chart_injection".into());
        referenced_chart.ensure_valid().unwrap();
        assert!(attach_canonical_result_document_v2_v6(&schema6, referenced_chart).is_err());

        let mut numeric_tamper = canonical.clone();
        let diagnostic = numeric_tamper
            .tables
            .iter_mut()
            .find(|table| table.id == PLS_NONLINEAR_DIAGNOSTICS_TABLE_ID_V1)
            .unwrap();
        diagnostic.rows[0].cells[3] = nonlinear_test_number(0.2);
        assert!(attach_canonical_result_document_v2_v6(&schema6, numeric_tamper).is_err());

        let mut binding_tamper = canonical.clone();
        binding_tamper.provenance.recipe_digest = "f".repeat(64);
        assert!(attach_canonical_result_document_v2_v6(&schema6, binding_tamper).is_err());

        let mut convergence_injection = canonical.clone();
        let mut injected = convergence_injection
            .tables
            .iter()
            .find(|table| table.id == crate::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1)
            .unwrap()
            .clone();
        injected.id = crate::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1.into();
        convergence_injection.sections[0]
            .table_ids
            .push(crate::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1.into());
        let insertion = convergence_injection.tables.len() - 3;
        convergence_injection.tables.insert(insertion, injected);
        convergence_injection.ensure_valid().unwrap();
        assert!(attach_canonical_result_document_v2_v6(&schema6, convergence_injection).is_err());

        let mut score_injection = canonical;
        let mut injected = score_injection
            .tables
            .iter()
            .find(|table| table.id == PLS_NONLINEAR_METHOD_SCOPE_TABLE_ID_V1)
            .unwrap()
            .clone();
        injected.id = PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2.into();
        injected.title = "Injected score table".into();
        injected.capability_cells = Some(vec![recipe_v4_pls_base_capability_cell_v1()]);
        score_injection.sections[0]
            .table_ids
            .push(PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2.into());
        let insertion = score_injection.tables.len() - 3;
        score_injection.tables.insert(insertion, injected);
        score_injection.ensure_valid().unwrap();
        assert!(attach_canonical_result_document_v2_v6(&schema6, score_injection).is_err());
    }

    fn legacy_model() -> ModelSpec {
        ModelSpec {
            id: Uuid::from_u128(10),
            name: "Legacy".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["y1".into(), "y2".into()],
                },
            ],
            paths: vec![StructuralPath {
                source: "x".into(),
                target: "y".into(),
            }],
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        }
    }

    fn recipe(method: AnalysisMethod) -> AnalysisRecipe {
        let method_config = match method {
            AnalysisMethod::PlsPm => Some(MethodConfig::PlsAlgorithm),
            AnalysisMethod::Cbsem => Some(MethodConfig::Cbsem {
                model_type: qpls_core::CbsemModelType::Sem,
                estimator: qpls_core::CbsemEstimator::Ml,
                input: qpls_core::CbsemInput::Raw,
                mean_structure: false,
                bootstrap_samples: 0,
                bootstrap_v2: None,
                group_column: None,
                invariance_steps: Vec::new(),
            }),
            _ => None,
        };
        AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(20 + method as u128),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: "dataset".into(),
            model: legacy_model(),
            settings: AnalysisSettings {
                method,
                ..AnalysisSettings::default()
            },
            method_config,
            metadata: BTreeMap::new(),
        }
    }

    fn request() -> ProjectArchiveUpgradeRequestV6 {
        ProjectArchiveUpgradeRequestV6 {
            source_archive_sha256: "a".repeat(64),
            source_archive_path: r"D:\study.qpls".into(),
            destination_archive_path: r"D:\study-upgraded.qpls".into(),
            upgraded_at: Utc.timestamp_opt(1_800_000_000, 0).unwrap(),
            legacy_display_covariances: BTreeMap::from([(
                legacy_model().id.to_string(),
                vec![LegacyDisplayCovarianceV4 {
                    id: "visual-cov".into(),
                    left_construct: "x".into(),
                    right_construct: "y".into(),
                    label: None,
                }],
            )]),
        }
    }

    fn explicit_recipe_v4(
        source: &AnalysisRecipe,
        interpretation: LegacyBasicModelInterpretationV4,
        id: Uuid,
    ) -> AnalysisRecipeV4 {
        let pending = migrate_analysis_recipe_to_v4_pending(source).unwrap();
        let migration_request = request();
        let drawings = migration_request
            .legacy_display_covariances
            .get(&source.model.id.to_string())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let (mut current, _) =
            confirm_legacy_recipe_estimand_v4(&pending, &source.model, drawings, interpretation)
                .unwrap();
        current.id = id;
        current
    }

    fn document_with_weight_binding(weight: SemWeightBindingV4) -> ProjectArchiveDocumentV6 {
        let mut project = Project::new("Weight declaration persistence");
        project.models.push(legacy_model());
        project.recipes.push(recipe(AnalysisMethod::PlsPm));
        let mut document = plan_project_upgrade_to_v6(&project, &request())
            .unwrap()
            .document;
        let current_recipe = explicit_recipe_v4(
            &project.recipes[0],
            LegacyBasicModelInterpretationV4::PlsComposite,
            Uuid::from_u128(9_001),
        );
        document.recipes.push(current_recipe);
        let (model_id, scientific_sha256) = {
            let record = &mut document.models[0];
            let ProjectModelPayloadV6::SemModelV4 {
                model,
                scientific_sha256,
            } = &mut record.payload
            else {
                unreachable!()
            };
            for (id, source_column) in [
                ("observed:weight", "survey_weight"),
                ("observed:alternate_weight", "alternate_weight"),
            ] {
                model.variables.push(SemVariableV4::Observed {
                    id: id.into(),
                    label: source_column.into(),
                    source_column: source_column.into(),
                    scale: ObservedScaleV4::Continuous,
                    role: ObservedRoleV4::Control,
                    categories: Vec::new(),
                    value_labels: BTreeMap::new(),
                    missing_markers: Vec::new(),
                    transformation_lineage: Vec::new(),
                });
            }
            let SemDataBindingV4::Raw {
                dataset_id,
                weight: configured,
                ..
            } = &mut model.data_binding
            else {
                unreachable!()
            };
            *dataset_id = "dataset:survey".into();
            *configured = Some(weight);
            model.ensure_valid().unwrap();
            let updated_sha256 = model.scientific_sha256().unwrap();
            *scientific_sha256 = updated_sha256.clone();
            (record.model_id.clone(), updated_sha256)
        };
        let AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: recipe_model_id,
            scientific_sha256: recipe_sha256,
        } = &mut document.recipes[0].model_binding
        else {
            unreachable!()
        };
        assert_eq!(recipe_model_id, &model_id);
        *recipe_sha256 = scientific_sha256;
        document.ensure_valid().unwrap();
        document
    }

    fn explicit_model(document: &ProjectArchiveDocumentV6) -> &SemModelV4 {
        let ProjectModelPayloadV6::SemModelV4 { model, .. } = &document.models[0].payload else {
            unreachable!()
        };
        model
    }

    fn explicit_model_mut(document: &mut ProjectArchiveDocumentV6) -> &mut SemModelV4 {
        let ProjectModelPayloadV6::SemModelV4 { model, .. } = &mut document.models[0].payload
        else {
            unreachable!()
        };
        model
    }

    fn make_first_model_a_draft(document: &mut ProjectArchiveDocumentV6) {
        let model = match &document.models[0].payload {
            ProjectModelPayloadV6::SemModelV4 { model, .. } => model.clone(),
            _ => panic!("fixture requires a ready SemModelV4"),
        };
        let model_document_sha256 = model.model_document_sha256().unwrap();
        document.models[0].payload = ProjectModelPayloadV6::SemModelV4Draft {
            model,
            model_document_sha256,
        };
    }

    fn refresh_stored_model_digest_only(document: &mut ProjectArchiveDocumentV6) {
        let ProjectModelPayloadV6::SemModelV4 {
            model,
            scientific_sha256,
        } = &mut document.models[0].payload
        else {
            unreachable!()
        };
        *scientific_sha256 = model.scientific_sha256().unwrap();
    }

    fn project_with_historical_result(method: AnalysisMethod) -> Project {
        let mut project = Project::new("Migration with history");
        project.models.push(legacy_model());
        project.recipes.push(recipe(method));
        let recipe_id = project.recipes[0].id;
        let timestamp = Utc.timestamp_opt(1_700_000_001, 0).unwrap();
        project.results.push(qpls_core::AnalysisResult {
            schema_version: RESULT_SCHEMA_VERSION,
            id: Uuid::from_u128(30),
            status: RunStatus::Completed,
            provenance: RunProvenance {
                recipe_id,
                dataset_fingerprint: "dataset".into(),
                method,
                method_version: "historical".into(),
                engine_version: "historical".into(),
                seed: 1,
                settings: project.recipes[0].settings.clone(),
                started_at: timestamp,
                completed_at: timestamp,
            },
            diagnostics: Vec::new(),
            payload: AnalysisPayload::Legacy {
                value: serde_json::json!({"coefficient": 0.25}),
            },
        });
        project
    }

    fn filesystem_plan(
        source: &Path,
        destination: &Path,
        source_bytes: &[u8],
    ) -> ProjectArchiveUpgradePlanV6 {
        let mut request = request();
        request.source_archive_sha256 = sha256_bytes(source_bytes);
        request.source_archive_path = source.to_str().unwrap().to_owned();
        request.destination_archive_path = destination.to_str().unwrap().to_owned();
        plan_project_upgrade_to_v6(
            &project_with_historical_result(AnalysisMethod::PlsPm),
            &request,
        )
        .unwrap()
    }

    fn assert_no_upgrade_temporary_files(directory: &Path) {
        let entries = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(
            entries
                .iter()
                .all(|name| !name.contains(".upgrade-v6-") && !name.contains(".append-v6-")),
            "private schema-v6 artifacts remain: {entries:?}"
        );
    }

    fn canonical_result_document(
        project_id: Uuid,
        document_id: &str,
        run_id: &str,
    ) -> CanonicalResultDocumentV2 {
        let digest = "a".repeat(64);
        let document = serde_json::json!({
            "schema_version": 2,
            "document_id": document_id,
            "title": "Historical result table",
            "provenance": {
                "run_id": run_id,
                "project_id": project_id.to_string(),
                "model_id": "model-1",
                "model_digest": digest,
                "dataset_id": "dataset-1",
                "dataset_fingerprint": "b".repeat(64),
                "recipe_id": "recipe-1",
                "recipe_digest": "c".repeat(64),
                "capability_cell": {
                    "registry_schema_version": 2,
                    "capability_id": "qpls3.pls.algorithm",
                    "cell_id": "standard.reflective_recursive",
                    "capability_version": "pls_algorithm_v2"
                },
                "method_version": "pls_algorithm_v2",
                "engine_version": "qpls-estimation-test",
                "seed": 42,
                "workers": 1,
                "started_at": "2026-08-14T00:00:00Z",
                "completed_at": "2026-08-14T00:00:01Z"
            },
            "sections": [{
                "id": "structural",
                "title": "Structural model",
                "table_ids": ["paths"],
                "chart_ids": []
            }],
            "tables": [{
                "id": "paths",
                "title": "Path coefficients",
                "columns": [{
                    "id": "path",
                    "label": "Path",
                    "data_type": "text",
                    "description": "Directed structural path",
                    "role": "label"
                }],
                "rows": [{
                    "id": "x_to_y",
                    "cells": [{"kind": "text", "value": "X to Y"}]
                }],
                "footnote_ids": []
            }],
            "charts": [],
            "notices": [],
            "exclusions": [],
            "footnotes": [],
            "presentation": {
                "default_section_id": "structural",
                "default_table_id": "paths",
                "precision": 4,
                "missing_value_label": "N/A",
                "chart_defaults": {}
            }
        });
        let parsed: CanonicalResultDocumentV2 = serde_json::from_value(document).unwrap();
        parsed.ensure_valid().unwrap();
        parsed
    }

    fn assert_non_model_bytes_preserved(
        source: &ProjectArchiveDocumentV6,
        mutated: &ProjectArchiveDocumentV6,
    ) {
        let mut normalized = mutated.clone();
        normalized.models = source.models.clone();
        assert_eq!(
            serde_json::to_vec(&normalized).unwrap(),
            serde_json::to_vec(source).unwrap(),
            "a model-only mutation changed schema/origin/datasets/layouts/history/current recipes/results/attachments",
        );
    }

    fn draft_digest(document: &ProjectArchiveDocumentV6, model_id: &str) -> String {
        let record = document
            .models
            .iter()
            .find(|record| record.model_id == model_id)
            .unwrap();
        let ProjectModelPayloadV6::SemModelV4Draft {
            model_document_sha256,
            ..
        } = &record.payload
        else {
            panic!("fixture requires a SemModelV4 draft")
        };
        model_document_sha256.clone()
    }

    #[test]
    fn archive_schema_access_is_historical_current_or_future_read_only() {
        assert_eq!(crate::PROJECT_ARCHIVE_VERSION, 5);
        assert_eq!(qpls_core::ANALYSIS_RECIPE_SCHEMA_VERSION, 3);
        assert!(classify_project_archive_schema_v6(0).is_err());
        assert_eq!(
            classify_project_archive_schema_v6(5).unwrap(),
            ProjectArchiveSchemaAccessV6::HistoricalUpgradeCopyRequired
        );
        assert_eq!(
            classify_project_archive_schema_v6(6).unwrap(),
            ProjectArchiveSchemaAccessV6::CurrentEditable
        );
        assert_eq!(
            classify_project_archive_schema_v6(7).unwrap(),
            ProjectArchiveSchemaAccessV6::FutureReadOnly
        );
    }

    #[test]
    fn draft_insert_replace_and_exact_promotion_are_clone_only_model_mutations() {
        let project = project_with_historical_result(AnalysisMethod::PlsPm);
        let mut source = plan_project_upgrade_to_v6(&project, &request())
            .unwrap()
            .document;
        source.layouts.insert(
            "model_editor.presentation".into(),
            serde_json::json!({"zoom": 1.25, "selected": ["x", "y"]}),
        );
        source.recipes.push(explicit_recipe_v4(
            &project.recipes[0],
            LegacyBasicModelInterpretationV4::PlsComposite,
            Uuid::from_u128(9_100),
        ));
        let mut unrelated_result = canonical_result_document(
            source.project_id,
            "result.document:model-mutation-preservation",
            "run-model-mutation-preservation",
        );
        unrelated_result.provenance.model_id = "model:unrelated-authority".into();
        let source = attach_canonical_result_document_v2_v6(&source, unrelated_result).unwrap();
        let source_before = serde_json::to_vec(&source).unwrap();
        let original_models = source.models.clone();

        let mut draft = explicit_model(&source).clone();
        draft.id = "model:revision:2".into();
        draft.name = "Revision 2 draft".into();
        let exact_inserted_model = draft.clone();
        let inserted = insert_sem_model_v4_draft_v6(&source, draft).unwrap();

        assert_eq!(serde_json::to_vec(&source).unwrap(), source_before);
        assert_non_model_bytes_preserved(&source, &inserted);
        assert_eq!(
            &inserted.models[..original_models.len()],
            original_models.as_slice()
        );
        let inserted_record = inserted.models.last().unwrap();
        assert_eq!(inserted_record.model_id, exact_inserted_model.id);
        let ProjectModelPayloadV6::SemModelV4Draft {
            model,
            model_document_sha256,
        } = &inserted_record.payload
        else {
            panic!("new authoring model must remain a draft")
        };
        assert_eq!(model, &exact_inserted_model);
        assert_eq!(
            model_document_sha256,
            &exact_inserted_model.model_document_sha256().unwrap()
        );

        let inserted_before = serde_json::to_vec(&inserted).unwrap();
        let expected_inserted_digest = model_document_sha256.clone();
        let mut replacement = exact_inserted_model;
        replacement.name = "Revision 2 edited draft".into();
        let exact_replacement = replacement.clone();
        let replaced = replace_sem_model_v4_draft_v6(
            &inserted,
            "model:revision:2",
            &expected_inserted_digest,
            replacement,
        )
        .unwrap();

        assert_eq!(serde_json::to_vec(&inserted).unwrap(), inserted_before);
        assert_non_model_bytes_preserved(&inserted, &replaced);
        assert_eq!(
            &replaced.models[..original_models.len()],
            original_models.as_slice()
        );
        let replacement_digest = draft_digest(&replaced, "model:revision:2");
        assert_ne!(replacement_digest, expected_inserted_digest);

        let replaced_before = serde_json::to_vec(&replaced).unwrap();
        let promoted =
            promote_sem_model_v4_draft_v6(&replaced, "model:revision:2", &replacement_digest)
                .unwrap();

        assert_eq!(serde_json::to_vec(&replaced).unwrap(), replaced_before);
        assert_non_model_bytes_preserved(&replaced, &promoted);
        assert_eq!(promoted.models.len(), replaced.models.len());
        let promoted_record = promoted.models.last().unwrap();
        let ProjectModelPayloadV6::SemModelV4 {
            model,
            scientific_sha256,
        } = &promoted_record.payload
        else {
            panic!("exact draft must be promoted to a ready SemModelV4")
        };
        assert_eq!(model, &exact_replacement);
        assert_eq!(scientific_sha256, &model.scientific_sha256().unwrap());
        assert_eq!(promoted.recipes, source.recipes);
        assert_eq!(
            promoted.canonical_result_documents,
            source.canonical_result_documents
        );
    }

    #[test]
    fn draft_mutations_reject_collision_stale_identity_invalid_content_and_tampering() {
        let source = plan_project_upgrade_to_v6(
            &project_with_historical_result(AnalysisMethod::PlsPm),
            &request(),
        )
        .unwrap()
        .document;
        let mut collision = explicit_model(&source).clone();
        collision.name = "Colliding draft".into();
        assert!(matches!(
            insert_sem_model_v4_draft_v6(&source, collision),
            Err(ProjectArchiveV6Error::DuplicateOrEmptyModelId(_))
        ));

        let mut candidate = explicit_model(&source).clone();
        candidate.id = "model:editable-draft".into();
        candidate.name = "Editable draft".into();
        let inserted = insert_sem_model_v4_draft_v6(&source, candidate).unwrap();
        let current_digest = draft_digest(&inserted, "model:editable-draft");
        let inserted_before = serde_json::to_vec(&inserted).unwrap();
        let ProjectModelPayloadV6::SemModelV4Draft {
            model: stored_draft,
            ..
        } = &inserted.models.last().unwrap().payload
        else {
            unreachable!()
        };
        let mut replacement = stored_draft.clone();
        replacement.name = "Edited draft".into();

        assert!(matches!(
            replace_sem_model_v4_draft_v6(
                &inserted,
                "model:editable-draft",
                &"0".repeat(64),
                replacement.clone(),
            ),
            Err(ProjectArchiveV6Error::ModelDocumentDigestMismatch {
                model_id,
                current,
                ..
            }) if model_id == "model:editable-draft" && current == current_digest
        ));
        assert!(matches!(
            promote_sem_model_v4_draft_v6(&inserted, "model:editable-draft", &"f".repeat(64),),
            Err(ProjectArchiveV6Error::ModelDocumentDigestMismatch { .. })
        ));

        let mut wrong_identity = replacement.clone();
        wrong_identity.id = "model:different-revision".into();
        assert!(matches!(
            replace_sem_model_v4_draft_v6(
                &inserted,
                "model:editable-draft",
                &current_digest,
                wrong_identity,
            ),
            Err(ProjectArchiveV6Error::ModelMutationIdentityMismatch { expected, observed })
                if expected == "model:editable-draft" && observed == "model:different-revision"
        ));

        let mut invalid = replacement;
        invalid.name = " ".into();
        assert!(matches!(
            replace_sem_model_v4_draft_v6(
                &inserted,
                "model:editable-draft",
                &current_digest,
                invalid.clone(),
            ),
            Err(ProjectArchiveV6Error::InvalidSemModel(_))
        ));
        invalid.id = "model:invalid-new-draft".into();
        assert!(matches!(
            insert_sem_model_v4_draft_v6(&source, invalid),
            Err(ProjectArchiveV6Error::InvalidSemModel(_))
        ));
        assert_eq!(serde_json::to_vec(&inserted).unwrap(), inserted_before);

        let mut tampered = inserted.clone();
        let ProjectModelPayloadV6::SemModelV4Draft { model, .. } =
            &mut tampered.models.last_mut().unwrap().payload
        else {
            unreachable!()
        };
        model.name = "Tampered without a matching envelope digest".into();
        assert!(matches!(
            promote_sem_model_v4_draft_v6(
                &tampered,
                "model:editable-draft",
                &current_digest,
            ),
            Err(ProjectArchiveV6Error::ModelDigestOrIdentity(model_id))
                if model_id == "model:editable-draft"
        ));
    }

    #[test]
    fn draft_promotion_fails_closed_when_authoring_content_is_not_ready() {
        let source = plan_project_upgrade_to_v6(
            &project_with_historical_result(AnalysisMethod::Cbsem),
            &request(),
        )
        .unwrap()
        .document;
        let mut incomplete = explicit_model(&source).clone();
        incomplete.id = "model:underidentified-draft".into();
        incomplete.name = "Underidentified draft".into();
        let identification = incomplete
            .variables
            .iter_mut()
            .find_map(|variable| match variable {
                SemVariableV4::CommonFactor { identification, .. } => Some(identification),
                _ => None,
            })
            .expect("CB-SEM fixture must contain a common factor");
        *identification = FactorIdentificationV4::FixedVariance;
        incomplete.ensure_authoring_integrity().unwrap();
        assert!(incomplete.ensure_valid().is_err());

        let inserted = insert_sem_model_v4_draft_v6(&source, incomplete).unwrap();
        let digest = draft_digest(&inserted, "model:underidentified-draft");
        let before = serde_json::to_vec(&inserted).unwrap();
        assert!(matches!(
            promote_sem_model_v4_draft_v6(&inserted, "model:underidentified-draft", &digest,),
            Err(ProjectArchiveV6Error::InvalidSemModel(_))
        ));
        assert_eq!(serde_json::to_vec(&inserted).unwrap(), before);
    }

    #[test]
    fn recipe_and_canonical_result_references_freeze_model_authority() {
        let project = project_with_historical_result(AnalysisMethod::PlsPm);
        let mut recipe_bound = plan_project_upgrade_to_v6(&project, &request())
            .unwrap()
            .document;
        recipe_bound.recipes.push(explicit_recipe_v4(
            &project.recipes[0],
            LegacyBasicModelInterpretationV4::PlsComposite,
            Uuid::from_u128(9_200),
        ));
        recipe_bound.ensure_valid().unwrap();
        let recipe_bound_before = serde_json::to_vec(&recipe_bound).unwrap();
        let model_id = recipe_bound.models[0].model_id.clone();
        let model_document_sha256 = explicit_model(&recipe_bound)
            .model_document_sha256()
            .unwrap();
        let mut replacement = explicit_model(&recipe_bound).clone();
        replacement.name = "Forbidden in-place scientific rewrite".into();
        assert!(matches!(
            replace_sem_model_v4_draft_v6(
                &recipe_bound,
                &model_id,
                &model_document_sha256,
                replacement,
            ),
            Err(ProjectArchiveV6Error::ModelMutationReferenced(referenced))
                if referenced == model_id
        ));
        assert!(matches!(
            promote_sem_model_v4_draft_v6(
                &recipe_bound,
                &model_id,
                &model_document_sha256,
            ),
            Err(ProjectArchiveV6Error::ModelMutationReferenced(referenced))
                if referenced == model_id
        ));
        assert_eq!(
            serde_json::to_vec(&recipe_bound).unwrap(),
            recipe_bound_before
        );

        let mut canonical_bound = plan_project_upgrade_to_v6(&project, &request())
            .unwrap()
            .document;
        make_first_model_a_draft(&mut canonical_bound);
        canonical_bound.ensure_valid().unwrap();
        let model_id = canonical_bound.models[0].model_id.clone();
        let digest = draft_digest(&canonical_bound, &model_id);
        let mut canonical = canonical_result_document(
            canonical_bound.project_id,
            "result.document:freezes-draft-authority",
            "run-freezes-draft-authority",
        );
        canonical.provenance.model_id = model_id.clone();
        canonical.provenance.model_digest = digest.clone();
        let canonical_bound =
            attach_canonical_result_document_v2_v6(&canonical_bound, canonical).unwrap();
        let canonical_bound_before = serde_json::to_vec(&canonical_bound).unwrap();
        let ProjectModelPayloadV6::SemModelV4Draft {
            model: replacement, ..
        } = &canonical_bound.models[0].payload
        else {
            unreachable!()
        };
        let mut replacement = replacement.clone();
        replacement.name = "Forbidden result-bound draft rewrite".into();
        assert!(matches!(
            replace_sem_model_v4_draft_v6(
                &canonical_bound,
                &model_id,
                &digest,
                replacement,
            ),
            Err(ProjectArchiveV6Error::ModelMutationReferenced(referenced))
                if referenced == model_id
        ));
        assert!(matches!(
            promote_sem_model_v4_draft_v6(&canonical_bound, &model_id, &digest),
            Err(ProjectArchiveV6Error::ModelMutationReferenced(referenced))
                if referenced == model_id
        ));
        assert_eq!(
            serde_json::to_vec(&canonical_bound).unwrap(),
            canonical_bound_before
        );

        let mut result_reserved = plan_project_upgrade_to_v6(&project, &request())
            .unwrap()
            .document;
        let mut canonical = canonical_result_document(
            result_reserved.project_id,
            "result.document:reserves-model-id",
            "run-reserves-model-id",
        );
        canonical.provenance.model_id = "model:reserved-by-result".into();
        result_reserved =
            attach_canonical_result_document_v2_v6(&result_reserved, canonical).unwrap();
        let mut identity_takeover = explicit_model(&result_reserved).clone();
        identity_takeover.id = "model:reserved-by-result".into();
        identity_takeover.name = "Forbidden identity takeover".into();
        assert!(matches!(
            insert_sem_model_v4_draft_v6(&result_reserved, identity_takeover),
            Err(ProjectArchiveV6Error::ModelMutationReferenced(referenced))
                if referenced == "model:reserved-by-result"
        ));
    }

    #[test]
    fn legacy_foundation_wire_reads_top_level_lineage_and_normalizes_corrected_origin() {
        let source = project_with_historical_result(AnalysisMethod::PlsPm);
        let mut document = plan_project_upgrade_to_v6(&source, &request())
            .unwrap()
            .document;
        document.recipes.push(explicit_recipe_v4(
            &source.recipes[0],
            LegacyBasicModelInterpretationV4::PlsComposite,
            source.recipes[0].id,
        ));

        let mut legacy_wire = serde_json::to_value(&document).unwrap();
        let object = legacy_wire.as_object_mut().unwrap();
        let origin = object.remove("origin").unwrap();
        object.insert(
            "upgrade_lineage".into(),
            origin.get("lineage").unwrap().clone(),
        );
        object.remove("historical_recipes");
        for result in object
            .get_mut("historical_results")
            .unwrap()
            .as_array_mut()
            .unwrap()
        {
            result.as_object_mut().unwrap().remove("source_recipe");
        }

        let reopened =
            deserialize_project_document_v6(&serde_json::to_vec(&legacy_wire).unwrap()).unwrap();
        assert!(matches!(
            reopened.origin,
            ProjectOriginV6::UpgradedCopy { .. }
        ));
        assert!(reopened.historical_recipes.is_empty());
        assert_eq!(reopened.recipes[0].id, source.recipes[0].id);
        assert!(matches!(
            reopened.historical_results[0].source_recipe(),
            HistoricalResultRecipeBindingV6::UnboundLegacy
        ));

        let corrected: Value =
            serde_json::from_slice(&serialize_project_document_v6(&reopened).unwrap()).unwrap();
        assert!(corrected.get("upgrade_lineage").is_none());
        assert_eq!(corrected["origin"]["kind"], "upgraded_copy");
        assert_eq!(
            corrected["historical_results"][0]["source_recipe"]["kind"],
            "unbound_legacy"
        );
    }

    #[test]
    fn new_project_origin_serializes_and_writes_without_fabricated_lineage() {
        let mut project = Project::new("New schema-v6 project");
        project.models.push(legacy_model());
        project.recipes.push(recipe(AnalysisMethod::PlsPm));
        let mut document = plan_project_upgrade_to_v6(&project, &request())
            .unwrap()
            .document;
        document.datasets.clear();
        document.models.clear();
        document.recipes.clear();
        document.historical_recipes.clear();
        document.layouts.clear();
        document.historical_results.clear();
        document.origin = ProjectOriginV6::NewProject;
        document.sem_generation = None;
        document.ensure_valid().unwrap();

        let first = serialize_project_document_v6(&document).unwrap();
        let second = serialize_project_document_v6(&document).unwrap();
        assert_eq!(first, second);
        let value: Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(value["origin"]["kind"], "new_project");
        assert!(value.get("upgrade_lineage").is_none());
        assert!(value.get("sem_generation").is_none());

        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("new-project-v6.json");
        let receipt = write_project_document_v6_new(&destination, &document).unwrap();
        assert_eq!(
            receipt.destination_archive_path,
            destination.to_string_lossy().into_owned()
        );
        assert!(matches!(
            read_project_document_v6(&destination).unwrap().origin,
            ProjectOriginV6::NewProject
        ));
    }

    #[test]
    fn general_sem_generation_is_explicit_persistent_and_new_project_only() {
        let created_at = Utc.with_ymd_and_hms(2026, 8, 18, 9, 30, 0).unwrap();
        let document = ProjectArchiveDocumentV6::new_general_sem_v1(
            Uuid::from_u128(0x6e65772d67656e6572616c2d73656d),
            "General SEM v1",
            created_at,
        );
        document.ensure_valid().unwrap();
        assert!(document.supports_general_sem_v1());

        let bytes = serialize_project_document_v6(&document).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["origin"]["kind"], "new_project");
        assert_eq!(value["sem_generation"], "general_sem_v1");
        let reopened = deserialize_project_document_v6(&bytes).unwrap();
        assert!(reopened.supports_general_sem_v1());

        let source = project_with_historical_result(AnalysisMethod::PlsPm);
        let mut upgraded = plan_project_upgrade_to_v6(&source, &request())
            .unwrap()
            .document;
        upgraded.sem_generation = Some(ProjectSemGenerationV6::GeneralSemV1);
        assert!(matches!(
            upgraded.ensure_valid(),
            Err(ProjectArchiveV6Error::GeneralSemGenerationRequiresNewProject)
        ));
    }

    fn general_sem_schema6_authority_fixture()
    -> (ProjectArchiveDocumentV6, CanonicalResultDocumentV2) {
        let dataset = qpls_data::import_delimited_bytes(
            b"x1,x2,m11,m12,m21,m22,y1,y2\n1,2,2,1,1,3,2,1\n2,1,3,2,2,2,3,2\n3,4,4,3,4,3,5,4\n4,3,5,5,3,5,6,5\n5,6,7,6,6,7,8,7\n6,5,6,7,7,6,9,8\n7,8,9,7,8,9,11,9\n8,7,8,9,9,8,10,11\n9,10,11,10,10,12,13,12\n10,9,12,11,12,10,14,13\n11,12,13,12,13,14,16,15\n12,11,14,13,14,13,17,16\n",
            "general-sem-schema6-authority.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let source_model = ModelSpec {
            id: Uuid::from_u128(0x6a01),
            name: "General SEM schema-6 authority".into(),
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
        let source_recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x6a02),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: source_model.clone(),
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                bootstrap_samples: 20,
                bootstrap_test_tail: qpls_core::PlsBootstrapTestTail::TwoSided,
                studentized_inner_samples: 0,
                confidence_level: 0.95,
                seed: 42,
                workers: 2,
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
        let model_scientific_sha256 = model.scientific_sha256().unwrap();
        recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: model.id.clone(),
            scientific_sha256: model_scientific_sha256.clone(),
        };
        recipe.general_sem_config = Some(GeneralSemConfigV1 {
            inference: GeneralSemInferenceV1::CaseBootstrap {
                resamples: 20,
                seed: 42,
                confidence_level: 0.95,
                interval: GeneralSemBootstrapIntervalV1::Percentile,
                tail: GeneralSemInferenceTailV1::TwoSided,
            },
            ..GeneralSemConfigV1::default()
        });
        recipe.ensure_valid().unwrap();
        let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap();
        let execution = qpls_runner::run_compiled_general_sem_pls_recipe_v1(
            &dataset,
            &recipe,
            &model,
            &artifact,
            || false,
            |_| {},
        )
        .unwrap();
        let general_sem_results = execution.canonical_general_sem_results_v1().unwrap();

        let mut project = ProjectArchiveDocumentV6::new_general_sem_v1(
            Uuid::from_u128(0x6a03),
            "General SEM authority fixture",
            Utc.timestamp_opt(1_800_000_000, 0).unwrap(),
        );
        project.datasets.push(DatasetDescriptor::from(&dataset));
        project.models.push(ProjectModelRecordV6 {
            model_id: model.id.clone(),
            payload: ProjectModelPayloadV6::SemModelV4 {
                model: model.clone(),
                scientific_sha256: model_scientific_sha256.clone(),
            },
        });
        project.recipes.push(recipe.clone());
        crate::write_project_data_lineage_v1(
            &mut project.layouts,
            &crate::ProjectDataLineageV1 {
                schema_version: crate::PROJECT_DATA_LINEAGE_SCHEMA_VERSION_V1,
                records: vec![crate::ProjectDatasetVersionRecordV1 {
                    dataset_id: dataset.id.to_string(),
                    parent_dataset_id: None,
                    operation: crate::ProjectDatasetVersionOperationV1::Import,
                    created_at: None,
                    summary: "Imported General SEM authority fixture".into(),
                    source_column: None,
                    target_column: None,
                    transformation: None,
                }],
            },
        )
        .unwrap();

        let canonical = CanonicalResultDocumentV2 {
            schema_version: 2,
            document_id: "general_sem_authority_document".into(),
            title: "General SEM PLS results".into(),
            provenance: CanonicalResultProvenanceV2 {
                run_id: "general_sem_authority_run".into(),
                project_id: project.project_id.to_string(),
                model_id: model.id,
                model_digest: model_scientific_sha256,
                dataset_id: dataset.id.to_string(),
                dataset_fingerprint: dataset.fingerprint.0,
                recipe_id: recipe.id.to_string(),
                recipe_digest: artifact.recipe_analytical_sha256().into(),
                capability_cell: project_capability_cell_v2(artifact.capability_cell()),
                method_version: GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1.into(),
                engine_version: execution.adapter_version().into(),
                seed: Some(recipe.settings.seed),
                workers: recipe.settings.workers as u32,
                started_at: "2026-08-19T00:00:00Z".into(),
                completed_at: "2026-08-19T00:00:01Z".into(),
            },
            capability_cells: Some(vec![
                project_capability_cell_v2(
                    &qpls_core::general_sem_pls_bootstrap_capability_cell_v1(),
                ),
                project_capability_cell_v2(artifact.capability_cell()),
            ]),
            general_sem_results: Some(general_sem_results),
            sections: Vec::new(),
            tables: Vec::new(),
            charts: Vec::new(),
            notices: Vec::new(),
            exclusions: Vec::new(),
            footnotes: Vec::new(),
            presentation: CanonicalResultPresentationV2 {
                default_section_id: None,
                default_table_id: None,
                precision: 4,
                missing_value_label: "—".into(),
                chart_defaults: CanonicalChartDisplayOptionsV2::default(),
            },
        };
        canonical.ensure_valid().unwrap();
        project
            .canonical_result_documents
            .push(CanonicalResultDocumentAttachmentV2::from_document(canonical.clone()).unwrap());
        (project, canonical)
    }

    fn add_schema6_two_way_interaction(
        model: &mut SemModelV4,
        interaction_id: &str,
        focal_predictor_id: &str,
        moderator_id: &str,
    ) {
        let focal_relation = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == focal_predictor_id && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .expect("the moderation fixture has the requested focal path");
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

    fn moderation_test_columns(ids: &[&str], number_ids: &[&str]) -> Vec<CanonicalResultColumnV2> {
        ids.iter()
            .map(|id| {
                nonlinear_test_column(
                    id,
                    if number_ids.contains(id) {
                        CanonicalColumnTypeV2::Number
                    } else {
                        CanonicalColumnTypeV2::Text
                    },
                )
            })
            .collect()
    }

    fn moderation_test_columns_with_booleans(
        ids: &[&str],
        number_ids: &[&str],
        boolean_ids: &[&str],
    ) -> Vec<CanonicalResultColumnV2> {
        ids.iter()
            .map(|id| {
                nonlinear_test_column(
                    id,
                    if number_ids.contains(id) {
                        CanonicalColumnTypeV2::Number
                    } else if boolean_ids.contains(id) {
                        CanonicalColumnTypeV2::Boolean
                    } else {
                        CanonicalColumnTypeV2::Text
                    },
                )
            })
            .collect()
    }

    fn moderation_test_boolean(value: bool) -> CanonicalResultCellV2 {
        CanonicalResultCellV2::Boolean { value }
    }

    fn moderation_test_missing() -> CanonicalResultCellV2 {
        CanonicalResultCellV2::Missing {
            reason: CanonicalMissingReasonV2::NotEstimated,
            display: None,
        }
    }

    fn moderation_test_stage_one_table(
        id: &str,
        owner: &crate::CapabilityCellReferenceV2,
    ) -> CanonicalResultTableV2 {
        nonlinear_test_table(
            id,
            vec![nonlinear_test_column("value", CanonicalColumnTypeV2::Text)],
            vec![CanonicalResultRowV2 {
                id: format!("{id}_row"),
                cells: vec![nonlinear_test_text("stage_one")],
            }],
            owner.clone(),
        )
    }

    fn moderation_test_point_estimate(value: f64) -> qpls_core::CanonicalGeneralSemEstimateV1 {
        qpls_core::CanonicalGeneralSemEstimateV1 {
            estimate: value,
            bootstrap_mean: None,
            bootstrap_bias: None,
            standard_error: None,
            lower: None,
            upper: None,
            p_value: None,
            bootstrap_usable_replicates: None,
            bootstrap_two_sided_exceedances: None,
        }
    }

    fn populate_moderation_test_joint_stage_ledger(
        results: &mut qpls_core::CanonicalGeneralSemResultsV1,
        execution: &qpls_runner::RecipeV4GeneralSemPlsExecutionResultV1,
        artifact: &qpls_core::CompiledGeneralSemPlsRecipeV1,
        model_id: &str,
    ) {
        let interactions = execution
            .interaction_point_estimation()
            .expect("the moderation fixture executed the joint stage");
        let trace = qpls_core::CanonicalGeneralSemResultTraceV1 {
            model_id: model_id.to_string(),
            capability_cell: artifact.capability_cell().clone(),
        };
        results.joint_stage_structural_coefficients = interactions
            .structural_coefficients()
            .iter()
            .map(|coefficient| {
                let relation = artifact
                    .plan()
                    .base_plan()
                    .paths()
                    .iter()
                    .find(|relation| relation.relation_id() == coefficient.relation_id())
                    .expect("joint-stage coefficient belongs to the compiled base plan");
                qpls_core::CanonicalJointStageStructuralCoefficientResultV1 {
                    relation_id: coefficient.relation_id().to_string(),
                    parameter_id: relation.parameter_id().to_string(),
                    trace: trace.clone(),
                    source_id: coefficient.source_id().to_string(),
                    target_id: coefficient.target_id().to_string(),
                    role: match relation.role() {
                        StructuralRelationRoleV4::Structural => {
                            qpls_core::CanonicalStructuralRelationRoleV1::Structural
                        }
                        StructuralRelationRoleV4::Control => {
                            qpls_core::CanonicalStructuralRelationRoleV1::Control
                        }
                    },
                    estimate: moderation_test_point_estimate(coefficient.estimate()),
                    stage: qpls_core::CanonicalStructuralEstimateStageV1::JointStageTwo,
                    method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1.into(),
                }
            })
            .collect();
    }

    fn moderation_test_result_tables(
        results: &qpls_core::CanonicalGeneralSemResultsV1,
        base: &crate::CapabilityCellReferenceV2,
        moderation: &crate::CapabilityCellReferenceV2,
        bootstrap: Option<&crate::CapabilityCellReferenceV2>,
    ) -> Vec<CanonicalResultTableV2> {
        let structural_rows = results
            .joint_stage_structural_coefficients
            .iter()
            .filter(|coefficient| {
                coefficient.role == qpls_core::CanonicalStructuralRelationRoleV1::Structural
            })
            .enumerate()
            .map(|(index, coefficient)| CanonicalResultRowV2 {
                id: format!("joint_path_{index:04}"),
                cells: vec![
                    nonlinear_test_text(&coefficient.relation_id),
                    nonlinear_test_text(&coefficient.parameter_id),
                    nonlinear_test_text(&coefficient.source_id),
                    nonlinear_test_text(&coefficient.target_id),
                    nonlinear_test_number(coefficient.estimate.estimate),
                ],
            })
            .collect();
        let structural = nonlinear_test_table(
            "structural_paths",
            moderation_test_columns(GENERAL_SEM_STRUCTURAL_PATH_COLUMNS_V1, &["coefficient"]),
            structural_rows,
            moderation.clone(),
        );

        let interaction_rows = results
            .interaction_effects
            .iter()
            .enumerate()
            .map(|(index, effect)| CanonicalResultRowV2 {
                id: format!("interaction_effect_{index:04}"),
                cells: vec![
                    nonlinear_test_text(&effect.effect_id),
                    nonlinear_test_text(&effect.interaction_id),
                    nonlinear_test_text(&effect.focal_relation_id),
                    nonlinear_test_text(&effect.interaction_effect_relation_id),
                    nonlinear_test_text(&effect.interaction_effect_parameter_id),
                    nonlinear_test_text(&effect.focal_predictor_id),
                    nonlinear_test_text(&effect.moderator_id),
                    nonlinear_test_text(&effect.outcome_id),
                    nonlinear_test_text(&effect.generated_product_column_id),
                    nonlinear_test_text(&effect.stage_one_model_scientific_sha256),
                    nonlinear_test_number(f64::from(effect.observation_count)),
                    nonlinear_test_number(effect.standardized_product_coefficient.estimate),
                    nonlinear_test_number(effect.scientific_rescaled_gamma.estimate),
                    nonlinear_test_number(effect.unstandardized_product_mean),
                    nonlinear_test_number(effect.unstandardized_product_sample_standard_deviation),
                    nonlinear_test_text("two_stage"),
                    nonlinear_test_text(&effect.product_scale_version),
                    nonlinear_test_text("strong"),
                    nonlinear_test_text(&effect.hierarchy_policy_version),
                    nonlinear_test_text(&effect.conditioning_policy_version),
                    nonlinear_test_text(&effect.method_version),
                ],
            })
            .collect();
        let interactions = nonlinear_test_table(
            GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1,
            moderation_test_columns(
                GENERAL_SEM_INTERACTION_EFFECT_COLUMNS_V1,
                &[
                    "observation_count",
                    "standardized_product_coefficient",
                    "scientific_rescaled_gamma",
                    "product_mean",
                    "product_sample_sd",
                ],
            ),
            interaction_rows,
            moderation.clone(),
        );

        let interactions_by_id = results
            .interaction_effects
            .iter()
            .map(|effect| (effect.interaction_id.as_str(), effect))
            .collect::<BTreeMap<_, _>>();
        let conditional_rows = results
            .conditional_effects
            .iter()
            .enumerate()
            .map(|(index, effect)| {
                let interaction = interactions_by_id[effect.interaction_id.as_str()];
                CanonicalResultRowV2 {
                    id: format!("conditional_slope_{index:04}"),
                    cells: vec![
                        nonlinear_test_text(&effect.effect_id),
                        nonlinear_test_text(&effect.interaction_id),
                        nonlinear_test_text(
                            effect
                                .interaction_effect_id
                                .as_deref()
                                .expect("typed conditional effect is interaction-bound"),
                        ),
                        nonlinear_test_text(&effect.focal_relation_id),
                        nonlinear_test_text(&effect.probe_id),
                        nonlinear_test_number(f64::from(effect.probe_value_index)),
                        nonlinear_test_text(&effect.moderator_id),
                        nonlinear_test_text(&interaction.outcome_id),
                        nonlinear_test_number(effect.moderator_value),
                        nonlinear_test_number(effect.value.estimate),
                        nonlinear_test_text(&interaction.conditioning_policy_version),
                    ],
                }
            })
            .collect();
        let conditional = nonlinear_test_table(
            GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1,
            moderation_test_columns(
                GENERAL_SEM_CONDITIONAL_SLOPE_COLUMNS_V1,
                &["probe_value_index", "moderator_value", "estimate"],
            ),
            conditional_rows,
            moderation.clone(),
        );

        let plot_rows = results
            .interaction_plots
            .iter()
            .flat_map(|plot| {
                plot.series.iter().flat_map(move |series| {
                    series.points.iter().map(move |point| (plot, series, point))
                })
            })
            .enumerate()
            .map(|(index, (plot, series, point))| CanonicalResultRowV2 {
                id: format!("interaction_plot_point_{index:04}"),
                cells: vec![
                    nonlinear_test_text(&plot.plot_id),
                    nonlinear_test_text(&plot.interaction_id),
                    nonlinear_test_text(
                        plot.interaction_effect_id
                            .as_deref()
                            .expect("typed plot is interaction-bound"),
                    ),
                    nonlinear_test_text(&plot.focal_relation_id),
                    nonlinear_test_text(&plot.focal_predictor_id),
                    nonlinear_test_text(&plot.moderator_id),
                    nonlinear_test_text(&plot.outcome_id),
                    nonlinear_test_text(&series.series_id),
                    nonlinear_test_text(&series.probe_id),
                    nonlinear_test_number(f64::from(series.probe_value_index)),
                    nonlinear_test_number(series.moderator_value),
                    nonlinear_test_number(point.focal_value),
                    nonlinear_test_number(point.predicted_value),
                    point
                        .lower
                        .map_or_else(moderation_test_missing, nonlinear_test_number),
                    point
                        .upper
                        .map_or_else(moderation_test_missing, nonlinear_test_number),
                ],
            })
            .collect();
        let plots = nonlinear_test_table(
            GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1,
            moderation_test_columns(
                GENERAL_SEM_INTERACTION_PLOT_COLUMNS_V1,
                &[
                    "probe_value_index",
                    "moderator_value",
                    "focal_value",
                    "predicted_value",
                    "lower",
                    "upper",
                ],
            ),
            plot_rows,
            moderation.clone(),
        );

        let mut tables = vec![
            moderation_test_stage_one_table("estimation_summary", base),
            moderation_test_stage_one_table("outer_model", base),
            structural,
            moderation_test_stage_one_table(
                crate::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1,
                base,
            ),
            moderation_test_stage_one_table(crate::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1, base),
            interactions,
            conditional,
            plots,
        ];
        if let Some(bootstrap) = bootstrap {
            let receipt = results
                .inference_receipt
                .as_ref()
                .expect("bootstrap tables require a typed receipt");
            let gamma_rows = results
                .interaction_effects
                .iter()
                .enumerate()
                .map(|(index, effect)| {
                    let gamma = &effect.scientific_rescaled_gamma;
                    CanonicalResultRowV2 {
                        id: format!("moderation_gamma_inference_{index:04}"),
                        cells: vec![
                            nonlinear_test_text(&effect.effect_id),
                            nonlinear_test_text(&effect.interaction_id),
                            nonlinear_test_text(&effect.focal_relation_id),
                            nonlinear_test_text(&effect.interaction_effect_relation_id),
                            nonlinear_test_text(&effect.interaction_effect_parameter_id),
                            nonlinear_test_text(&effect.generated_product_column_id),
                            nonlinear_test_text(&effect.focal_predictor_id),
                            nonlinear_test_text(&effect.moderator_id),
                            nonlinear_test_text(&effect.outcome_id),
                            nonlinear_test_text(&effect.stage_one_model_scientific_sha256),
                            nonlinear_test_text(&effect.product_scale_version),
                            nonlinear_test_text(&effect.method_version),
                            nonlinear_test_number(gamma.estimate),
                            nonlinear_test_number(gamma.bootstrap_mean.unwrap()),
                            nonlinear_test_number(gamma.bootstrap_bias.unwrap()),
                            nonlinear_test_number(gamma.standard_error.unwrap()),
                            nonlinear_test_number(gamma.lower.unwrap()),
                            nonlinear_test_number(gamma.upper.unwrap()),
                            nonlinear_test_number(gamma.p_value.unwrap()),
                            nonlinear_test_number(f64::from(
                                gamma.bootstrap_usable_replicates.unwrap(),
                            )),
                            nonlinear_test_number(f64::from(
                                gamma.bootstrap_two_sided_exceedances.unwrap(),
                            )),
                        ],
                    }
                })
                .collect();
            tables.push(nonlinear_test_table(
                GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1,
                moderation_test_columns(
                    GENERAL_SEM_MODERATION_GAMMA_INFERENCE_COLUMNS_V1,
                    &[
                        "estimate",
                        "bootstrap_mean",
                        "bootstrap_bias",
                        "standard_error",
                        "lower",
                        "upper",
                        "p_value",
                        "bootstrap_usable_replicates",
                        "bootstrap_two_sided_exceedances",
                    ],
                ),
                gamma_rows,
                bootstrap.clone(),
            ));

            let stage_one_digest = results.interaction_effects[0]
                .stage_one_model_scientific_sha256
                .clone();
            let receipt_row = CanonicalResultRowV2 {
                id: "moderation_bootstrap_receipt".into(),
                cells: vec![
                    nonlinear_test_text(&receipt.capability_cell.capability_id),
                    nonlinear_test_text(&receipt.capability_cell.cell_id),
                    nonlinear_test_text(&receipt.capability_cell.capability_version),
                    nonlinear_test_text(&receipt.method_version),
                    nonlinear_test_text(
                        qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
                    ),
                    nonlinear_test_text(&receipt.resampling_operation_version),
                    nonlinear_test_text(&receipt.resampling_stream_version),
                    nonlinear_test_text(&receipt.quantile_method_version),
                    nonlinear_test_text(&receipt.standard_error_method_version),
                    nonlinear_test_text(&receipt.summation_method_version),
                    nonlinear_test_text(&receipt.p_value_method_version),
                    nonlinear_test_text(&receipt.failure_policy_version),
                    nonlinear_test_text(
                        qpls_resampling::GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1,
                    ),
                    nonlinear_test_text(qpls_core::GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1),
                    nonlinear_test_text(
                        qpls_resampling::GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1,
                    ),
                    nonlinear_test_text(&receipt.compilation_artifact_identity_sha256),
                    nonlinear_test_text(&receipt.compiled_plan_sha256),
                    nonlinear_test_text(&receipt.general_sem_config_sha256),
                    nonlinear_test_text(&receipt.recipe_analytical_sha256),
                    nonlinear_test_text(&receipt.model_scientific_sha256),
                    nonlinear_test_text(stage_one_digest),
                    nonlinear_test_text(&receipt.source_dataset_fingerprint),
                    nonlinear_test_text(&receipt.complete_case_frame_sha256),
                    nonlinear_test_text(&receipt.usable_replicate_indices_sha256),
                    nonlinear_test_text(&receipt.effect_identity_set_sha256),
                    nonlinear_test_text("percentile_type7"),
                    nonlinear_test_text("two_sided"),
                    nonlinear_test_number(receipt.confidence_level),
                    nonlinear_test_number(f64::from(receipt.resamples_requested)),
                    nonlinear_test_number(f64::from(receipt.resamples_usable)),
                    nonlinear_test_number(f64::from(receipt.minimum_usable_resamples)),
                    nonlinear_test_text(&receipt.seed),
                    nonlinear_test_number(f64::from(receipt.workers)),
                    moderation_test_boolean(true),
                    moderation_test_boolean(true),
                    moderation_test_boolean(true),
                    moderation_test_boolean(true),
                    moderation_test_boolean(true),
                    moderation_test_boolean(true),
                    nonlinear_test_number(receipt.failed_replicates.len() as f64),
                ],
            };
            tables.push(nonlinear_test_table(
                GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
                moderation_test_columns_with_booleans(
                    GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_COLUMNS_V1,
                    &[
                        "confidence_level",
                        "resamples_requested",
                        "resamples_usable",
                        "minimum_usable_resamples",
                        "workers",
                        "failed_replicate_count",
                    ],
                    &[
                        "complete_model_reestimated_per_replicate",
                        "shared_stage_one_reestimated_per_replicate",
                        "score_vectors_sign_aligned_before_products",
                        "product_scaling_recomputed_per_replicate",
                        "joint_stage_two_reestimated_per_replicate",
                        "complete_joint_point_contract_validated_per_replicate",
                    ],
                ),
                vec![receipt_row],
                bootstrap.clone(),
            ));
        }
        tables
    }

    fn moderation_test_charts(
        results: &qpls_core::CanonicalGeneralSemResultsV1,
    ) -> Vec<CanonicalResultChartV2> {
        results
            .interaction_plots
            .iter()
            .enumerate()
            .map(|(plot_index, plot)| CanonicalResultChartV2 {
                id: format!("general_sem_interaction_chart_{plot_index:04}"),
                title: format!("Interaction {}", plot.interaction_id),
                description: "Exact typed interaction plot fixture".into(),
                kind: CanonicalChartKindV2::Line,
                series: plot
                    .series
                    .iter()
                    .map(|series| CanonicalChartSeriesV2 {
                        id: series.series_id.clone(),
                        label: format!("{} = {:.4}", plot.moderator_id, series.moderator_value),
                        group: Some(plot.interaction_id.clone()),
                        points: series
                            .points
                            .iter()
                            .map(|point| CanonicalChartPointV2 {
                                x: CanonicalChartXValueV2::Number(point.focal_value),
                                y: point.predicted_value,
                                lower: point.lower,
                                upper: point.upper,
                                label: None,
                            })
                            .collect(),
                    })
                    .collect(),
                source_table_id: Some(GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1.into()),
                display: CanonicalChartDisplayOptionsV2 {
                    palette: None,
                    show_legend: Some(true),
                    show_values: Some(false),
                    x_axis_label: Some(format!("{} (standardized)", plot.focal_predictor_id)),
                    y_axis_label: Some(format!("{} (predicted standardized)", plot.outcome_id)),
                },
            })
            .collect()
    }

    fn general_sem_schema6_moderation_authority_fixture_with_inference(
        same_focal: bool,
        bootstrap: bool,
    ) -> (ProjectArchiveDocumentV6, CanonicalResultDocumentV2) {
        let source_model = ModelSpec {
            id: Uuid::from_u128(0x4d4f_4452_5343_4845_4d41_3601),
            name: "Schema-6 simultaneous moderation authority".into(),
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
            let y = 0.25 * x + 0.20 * w - 0.15 * z + 0.65 * x * w - 0.40 * x * z;
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
        let dataset = qpls_data::import_delimited_bytes(
            csv.as_bytes(),
            "schema6-simultaneous-moderation.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let source_recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x4d4f_4452_5343_4845_4d41_3602),
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
        add_schema6_two_way_interaction(
            &mut model,
            "interaction:x_by_w",
            "construct:x",
            "construct:w",
        );
        if same_focal {
            add_schema6_two_way_interaction(
                &mut model,
                "interaction:x_by_z",
                "construct:x",
                "construct:z",
            );
        } else {
            add_schema6_two_way_interaction(
                &mut model,
                "interaction:w_by_z",
                "construct:w",
                "construct:z",
            );
        }
        let model_scientific_sha256 = model.scientific_sha256().unwrap();
        recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: model.id.clone(),
            scientific_sha256: model_scientific_sha256.clone(),
        };
        recipe.general_sem_config = Some(if bootstrap {
            recipe.settings.bootstrap_samples = 20;
            recipe.settings.bootstrap_test_tail = qpls_core::PlsBootstrapTestTail::TwoSided;
            recipe.settings.studentized_inner_samples = 0;
            recipe.settings.confidence_level = 0.95;
            recipe.settings.seed = 42;
            recipe.settings.workers = 2;
            GeneralSemConfigV1 {
                inference: GeneralSemInferenceV1::CaseBootstrap {
                    resamples: 20,
                    seed: 42,
                    confidence_level: 0.95,
                    interval: GeneralSemBootstrapIntervalV1::Percentile,
                    tail: GeneralSemInferenceTailV1::TwoSided,
                },
                ..GeneralSemConfigV1::default()
            }
        } else {
            GeneralSemConfigV1::default()
        });
        recipe.ensure_valid().unwrap();
        let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap();
        let execution = qpls_runner::run_compiled_general_sem_pls_recipe_v1(
            &dataset,
            &recipe,
            &model,
            &artifact,
            || false,
            |_| {},
        )
        .unwrap();
        let mut general_sem_results = execution.canonical_general_sem_results_v1().unwrap();
        populate_moderation_test_joint_stage_ledger(
            &mut general_sem_results,
            &execution,
            &artifact,
            &model.id,
        );

        let mut project = ProjectArchiveDocumentV6::new_general_sem_v1(
            Uuid::from_u128(0x4d4f_4452_5343_4845_4d41_3603),
            "Schema-6 simultaneous moderation authority",
            Utc.timestamp_opt(1_800_000_000, 0).unwrap(),
        );
        project.datasets.push(DatasetDescriptor::from(&dataset));
        project.models.push(ProjectModelRecordV6 {
            model_id: model.id.clone(),
            payload: ProjectModelPayloadV6::SemModelV4 {
                model: model.clone(),
                scientific_sha256: model_scientific_sha256.clone(),
            },
        });
        project.recipes.push(recipe.clone());
        crate::write_project_data_lineage_v1(
            &mut project.layouts,
            &crate::ProjectDataLineageV1 {
                schema_version: crate::PROJECT_DATA_LINEAGE_SCHEMA_VERSION_V1,
                records: vec![crate::ProjectDatasetVersionRecordV1 {
                    dataset_id: dataset.id.to_string(),
                    parent_dataset_id: None,
                    operation: crate::ProjectDatasetVersionOperationV1::Import,
                    created_at: None,
                    summary: "Imported schema-6 simultaneous moderation authority fixture".into(),
                    source_column: None,
                    target_column: None,
                    transformation: None,
                }],
            },
        )
        .unwrap();

        let moderation_cell = project_capability_cell_v2(artifact.capability_cell());
        let base_cell = moderation_base_capability_cell_v1(&recipe);
        let bootstrap_cell = bootstrap.then(|| {
            project_capability_cell_v2(
                &qpls_core::pls_general_multiple_moderation_bootstrap_capability_cell_v1(),
            )
        });
        let mut capability_cells = vec![base_cell.clone(), moderation_cell.clone()];
        if let Some(bootstrap_cell) = &bootstrap_cell {
            capability_cells.push(bootstrap_cell.clone());
        }
        sort_project_capability_cells_v1(&mut capability_cells);
        let tables = moderation_test_result_tables(
            &general_sem_results,
            &base_cell,
            &moderation_cell,
            bootstrap_cell.as_ref(),
        );
        let charts = moderation_test_charts(&general_sem_results);
        let chart_ids = charts.iter().map(|chart| chart.id.clone()).collect();
        let mut sections = vec![
            CanonicalResultSectionV2 {
                id: "run_details".into(),
                title: "Stage-one score estimation".into(),
                description: Some("Stage-one fixture".into()),
                table_ids: vec![
                    "estimation_summary".into(),
                    crate::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1.into(),
                    crate::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1.into(),
                ],
                chart_ids: Vec::new(),
                capability_cells: Some(vec![base_cell.clone()]),
            },
            CanonicalResultSectionV2 {
                id: "measurement_model".into(),
                title: "Stage-one measurement model".into(),
                description: Some("Stage-one measurement fixture".into()),
                table_ids: vec!["outer_model".into()],
                chart_ids: Vec::new(),
                capability_cells: Some(vec![base_cell]),
            },
            CanonicalResultSectionV2 {
                id: "structural_model".into(),
                title: "Joint stage-two structural model".into(),
                description: Some("Final joint-stage fixture".into()),
                table_ids: vec!["structural_paths".into()],
                chart_ids: Vec::new(),
                capability_cells: Some(vec![moderation_cell.clone()]),
            },
            CanonicalResultSectionV2 {
                id: GENERAL_SEM_MODERATION_SECTION_ID_V1.into(),
                title: "Moderation effects".into(),
                description: Some("Typed moderation fixture".into()),
                table_ids: vec![
                    GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1.into(),
                    GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1.into(),
                    GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1.into(),
                ],
                chart_ids,
                capability_cells: Some(vec![moderation_cell.clone()]),
            },
        ];
        if let Some(bootstrap_cell) = &bootstrap_cell {
            sections.push(CanonicalResultSectionV2 {
                id: GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1.into(),
                title: "Moderation bootstrap inference".into(),
                description: Some("Scientific gamma inference and full-pipeline receipt".into()),
                table_ids: vec![
                    GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1.into(),
                    GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1.into(),
                ],
                chart_ids: Vec::new(),
                capability_cells: Some(vec![bootstrap_cell.clone()]),
            });
        }
        let exclusions = if let Some(bootstrap_cell) = &bootstrap_cell {
            GENERAL_SEM_MODERATION_BOOTSTRAP_EXCLUSION_IDS_V1
                .iter()
                .map(|id| crate::CanonicalResultExclusionV2 {
                    id: (*id).into(),
                    capability_cell: Some(bootstrap_cell.clone()),
                    title: match *id {
                        "moderation_bootstrap_scientific_gamma_only" => {
                            "Only scientific gamma is bootstrap-inferred"
                        }
                        "moderation_beta_joint_coefficients_slopes_plots_point_only" => {
                            "Other moderation surfaces remain point-only"
                        }
                        _ => "Joint-model effects and fit not estimated",
                    }
                    .into(),
                    reason: "Parity-first moderation-bootstrap v1 boundary".into(),
                })
                .collect()
        } else {
            vec![
                crate::CanonicalResultExclusionV2 {
                    id: GENERAL_SEM_MODERATION_EXCLUSION_IDS_V1[0].into(),
                    capability_cell: Some(moderation_cell.clone()),
                    title: "Moderation inference not qualified".into(),
                    reason: "Point-only fixture".into(),
                },
                crate::CanonicalResultExclusionV2 {
                    id: GENERAL_SEM_MODERATION_EXCLUSION_IDS_V1[1].into(),
                    capability_cell: Some(moderation_cell.clone()),
                    title: "Joint-model effects and fit not estimated".into(),
                    reason: "No qualified joint fit fixture".into(),
                },
            ]
        };
        let canonical = CanonicalResultDocumentV2 {
            schema_version: 2,
            document_id: "general_sem_moderation_authority_document".into(),
            title: if bootstrap {
                "General SEM simultaneous two-way PLS moderation bootstrap inference"
            } else {
                "General SEM simultaneous two-way PLS moderation point estimates"
            }
            .into(),
            provenance: CanonicalResultProvenanceV2 {
                run_id: "general_sem_moderation_authority_run".into(),
                project_id: project.project_id.to_string(),
                model_id: model.id,
                model_digest: model_scientific_sha256,
                dataset_id: dataset.id.to_string(),
                dataset_fingerprint: dataset.fingerprint.0,
                recipe_id: recipe.id.to_string(),
                recipe_digest: artifact.recipe_analytical_sha256().into(),
                capability_cell: moderation_cell.clone(),
                method_version: if bootstrap {
                    qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
                } else {
                    GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
                }
                .into(),
                engine_version: execution.adapter_version().into(),
                seed: Some(recipe.settings.seed),
                workers: recipe.settings.workers as u32,
                started_at: "2026-08-19T00:00:00Z".into(),
                completed_at: "2026-08-19T00:00:01Z".into(),
            },
            capability_cells: Some(capability_cells),
            general_sem_results: Some(general_sem_results),
            sections,
            tables,
            charts,
            notices: Vec::new(),
            exclusions,
            footnotes: Vec::new(),
            presentation: CanonicalResultPresentationV2 {
                default_section_id: Some(if bootstrap {
                    GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1
                } else {
                    GENERAL_SEM_MODERATION_SECTION_ID_V1
                }
                .into()),
                default_table_id: Some(if bootstrap {
                    GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1
                } else {
                    GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1
                }
                .into()),
                precision: 4,
                missing_value_label: "—".into(),
                chart_defaults: CanonicalChartDisplayOptionsV2::default(),
            },
        };
        canonical.ensure_valid().unwrap();
        project
            .canonical_result_documents
            .push(CanonicalResultDocumentAttachmentV2::from_document(canonical.clone()).unwrap());
        (project, canonical)
    }

    fn general_sem_schema6_moderation_authority_fixture(
        same_focal: bool,
    ) -> (ProjectArchiveDocumentV6, CanonicalResultDocumentV2) {
        general_sem_schema6_moderation_authority_fixture_with_inference(same_focal, false)
    }

    fn general_sem_schema6_moderation_bootstrap_authority_fixture(
        same_focal: bool,
    ) -> (ProjectArchiveDocumentV6, CanonicalResultDocumentV2) {
        general_sem_schema6_moderation_authority_fixture_with_inference(same_focal, true)
    }

    fn assert_moderation_authority_rejects(
        project: &ProjectArchiveDocumentV6,
        document: CanonicalResultDocumentV2,
        expected: &str,
    ) {
        document
            .ensure_valid()
            .expect("tamper fixture must remain generically canonical");
        let mut tampered = project.clone();
        tampered.canonical_result_documents =
            vec![CanonicalResultDocumentAttachmentV2::from_document(document).unwrap()];
        assert!(
            matches!(
                tampered.ensure_valid(),
                Err(ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message))
                    if message.contains(expected)
            ),
            "expected schema-6 moderation authority error containing {expected:?}"
        );
    }

    fn moderation_test_table_mut<'a>(
        document: &'a mut CanonicalResultDocumentV2,
        table_id: &str,
    ) -> &'a mut CanonicalResultTableV2 {
        document
            .tables
            .iter_mut()
            .find(|table| table.id == table_id)
            .unwrap_or_else(|| panic!("missing moderation fixture table {table_id}"))
    }

    fn moderation_test_number_cell_mut(cell: &mut CanonicalResultCellV2) -> &mut f64 {
        let CanonicalResultCellV2::Number {
            value,
            display: None,
        } = cell
        else {
            panic!("expected an undisplayed numeric moderation fixture cell")
        };
        value
    }

    fn coherently_tamper_joint_stage_coefficient(
        document: &mut CanonicalResultDocumentV2,
        relation_id: &str,
        delta: f64,
    ) {
        let coefficient = document
            .general_sem_results
            .as_mut()
            .unwrap()
            .joint_stage_structural_coefficients
            .iter_mut()
            .find(|coefficient| coefficient.relation_id == relation_id)
            .expect("focal relation exists in the joint-stage ledger");
        coefficient.estimate.estimate += delta;
        let table = moderation_test_table_mut(document, "structural_paths");
        let row = table
            .rows
            .iter_mut()
            .find(|row| {
                matches!(
                    row.cells.first(),
                    Some(CanonicalResultCellV2::Text { value }) if value == relation_id
                )
            })
            .expect("focal relation exists in structural_paths");
        *moderation_test_number_cell_mut(&mut row.cells[4]) += delta;
    }

    #[test]
    fn general_sem_result_is_bound_to_resident_schema6_authorities() {
        assert_eq!(
            GENERAL_SEM_PLS_POINT_EXECUTION_ADAPTER_VERSION_V1,
            qpls_runner::RECIPE_V4_GENERAL_SEM_PLS_EXECUTION_ADAPTER_VERSION_V1
        );
        assert_eq!(
            GENERAL_SEM_PLS_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
            qpls_runner::RECIPE_V4_GENERAL_SEM_PLS_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1
        );
        assert_eq!(
            GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1,
            qpls_runner::RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1
        );
        assert_eq!(
            GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
            qpls_runner::RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1
        );
        let (project, canonical) = general_sem_schema6_authority_fixture();
        project.ensure_valid().unwrap();

        let mut no_generation = project.clone();
        no_generation.sem_generation = None;
        assert!(matches!(
            no_generation.ensure_valid(),
            Err(ProjectArchiveV6Error::GeneralSemFeatureRequiresGeneration { .. })
        ));

        let mut no_recipe = project.clone();
        no_recipe.recipes.clear();
        assert!(matches!(
            no_recipe.ensure_valid(),
            Err(ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message))
                if message.contains("Recipe-v4")
        ));

        let mut artifact_tamper = canonical.clone();
        artifact_tamper
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap()
            .compilation_artifact_identity_sha256 = "0".repeat(64);
        artifact_tamper.ensure_valid().unwrap();
        let mut artifact_project = project.clone();
        artifact_project.canonical_result_documents =
            vec![CanonicalResultDocumentAttachmentV2::from_document(artifact_tamper).unwrap()];
        assert!(matches!(
            artifact_project.ensure_valid(),
            Err(ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message))
                if message.contains("compiled artifact")
        ));

        let mut method_tamper = canonical.clone();
        method_tamper.provenance.method_version = GENERAL_SEM_EFFECTS_V1_METHOD_VERSION.into();
        method_tamper.ensure_valid().unwrap();
        let mut method_project = project.clone();
        method_project.canonical_result_documents =
            vec![CanonicalResultDocumentAttachmentV2::from_document(method_tamper).unwrap()];
        assert!(matches!(
            method_project.ensure_valid(),
            Err(ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message))
                if message.contains("method/engine")
        ));

        let mut engine_tamper = canonical.clone();
        engine_tamper.provenance.engine_version =
            GENERAL_SEM_PLS_POINT_EXECUTION_ADAPTER_VERSION_V1.into();
        engine_tamper.ensure_valid().unwrap();
        let mut engine_project = project.clone();
        engine_project.canonical_result_documents =
            vec![CanonicalResultDocumentAttachmentV2::from_document(engine_tamper).unwrap()];
        assert!(matches!(
            engine_project.ensure_valid(),
            Err(ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message))
                if message.contains("method/engine")
        ));

        let mut bootstrap_tamper = canonical;
        bootstrap_tamper
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap()
            .confidence_level = 0.9;
        bootstrap_tamper.ensure_valid().unwrap();
        let mut bootstrap_project = project;
        bootstrap_project.canonical_result_documents =
            vec![CanonicalResultDocumentAttachmentV2::from_document(bootstrap_tamper).unwrap()];
        assert!(matches!(
            bootstrap_project.ensure_valid(),
            Err(ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message))
                if message.contains("bootstrap settings")
        ));
    }

    #[test]
    fn general_sem_moderation_archive_accepts_same_and_different_focal_models() {
        for same_focal in [true, false] {
            let (project, canonical) = general_sem_schema6_moderation_authority_fixture(same_focal);
            project.ensure_valid().unwrap();
            let results = canonical.general_sem_results.as_ref().unwrap();
            assert_eq!(results.interaction_effects.len(), 2);
            assert_eq!(results.conditional_effect_probes.len(), 2);
            assert_eq!(results.conditional_effects.len(), 6);
            assert_eq!(results.interaction_plots.len(), 2);
            assert_eq!(results.joint_stage_structural_coefficients.len(), 3);
            let focal_relation_ids = results
                .interaction_effects
                .iter()
                .map(|effect| effect.focal_relation_id.as_str())
                .collect::<BTreeSet<_>>();
            assert_eq!(focal_relation_ids.len(), if same_focal { 1 } else { 2 });
        }
    }

    #[test]
    fn general_sem_moderation_bootstrap_archive_accepts_same_and_different_focal_models() {
        for same_focal in [true, false] {
            let (project, canonical) =
                general_sem_schema6_moderation_bootstrap_authority_fixture(same_focal);
            project.ensure_valid().unwrap();
            let results = canonical.general_sem_results.as_ref().unwrap();
            let receipt = results.inference_receipt.as_ref().unwrap();
            assert_eq!(
                receipt.capability_cell,
                qpls_core::pls_general_multiple_moderation_bootstrap_capability_cell_v1()
            );
            assert_eq!(
                receipt.method_version,
                qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
            );
            assert_eq!(canonical.capability_cells.as_ref().unwrap().len(), 3);
            assert_eq!(
                canonical.sections.last().unwrap().id,
                GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1
            );
            assert_eq!(
                canonical.tables[canonical.tables.len() - 2].id,
                GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1
            );
            assert_eq!(
                canonical.tables.last().unwrap().id,
                GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1
            );
            assert!(results.interaction_effects.iter().all(|effect| {
                moderation_estimate_has_complete_inference_v1(&effect.scientific_rescaled_gamma)
                    && !moderation_estimate_has_inference_v1(
                        &effect.standardized_product_coefficient,
                    )
            }));
            assert!(
                results
                    .joint_stage_structural_coefficients
                    .iter()
                    .all(|coefficient| {
                        !moderation_estimate_has_inference_v1(&coefficient.estimate)
                    })
            );
            assert!(
                results
                    .conditional_effects
                    .iter()
                    .all(|effect| { !moderation_estimate_has_inference_v1(&effect.value) })
            );
            assert!(results.interaction_plots.iter().all(|plot| {
                plot.series.iter().all(|series| {
                    series
                        .points
                        .iter()
                        .all(|point| point.lower.is_none() && point.upper.is_none())
                })
            }));
            let focal_relation_ids = results
                .interaction_effects
                .iter()
                .map(|effect| effect.focal_relation_id.as_str())
                .collect::<BTreeSet<_>>();
            assert_eq!(focal_relation_ids.len(), if same_focal { 1 } else { 2 });
        }
    }

    #[test]
    fn general_sem_moderation_bootstrap_archive_rejects_coherent_resident_binding_tamper() {
        let (same_project, mut same_document) =
            general_sem_schema6_moderation_bootstrap_authority_fixture(true);
        same_document
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap()
            .compiled_plan_sha256 = "0".repeat(64);
        moderation_test_table_mut(
            &mut same_document,
            GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
        )
        .rows[0]
            .cells[16] = nonlinear_test_text("0".repeat(64));
        assert_moderation_authority_rejects(
            &same_project,
            same_document,
            "resident compiled artifact, plan, config, recipe, model, dataset, or bootstrap settings",
        );

        let (different_project, mut different_document) =
            general_sem_schema6_moderation_bootstrap_authority_fixture(false);
        let tampered_product_id = "derived:coherent_product_tamper";
        different_document
            .general_sem_results
            .as_mut()
            .unwrap()
            .interaction_effects[0]
            .generated_product_column_id = tampered_product_id.into();
        moderation_test_table_mut(
            &mut different_document,
            GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1,
        )
        .rows[0]
            .cells[8] = nonlinear_test_text(tampered_product_id);
        moderation_test_table_mut(
            &mut different_document,
            GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1,
        )
        .rows[0]
            .cells[5] = nonlinear_test_text(tampered_product_id);
        let identity_digest = {
            let results = different_document.general_sem_results.as_ref().unwrap();
            qpls_core::general_sem_effect_identity_set_sha256_v1(
                &qpls_core::canonical_general_sem_effect_identities_v1(results),
            )
        };
        different_document
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap()
            .effect_identity_set_sha256 = identity_digest.clone();
        moderation_test_table_mut(
            &mut different_document,
            GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
        )
        .rows[0]
            .cells[24] = nonlinear_test_text(identity_digest);
        assert_moderation_authority_rejects(
            &different_project,
            different_document,
            "compiled interaction contract",
        );
    }

    #[test]
    fn general_sem_moderation_bootstrap_archive_reconciles_gamma_and_pipeline_receipt_tables() {
        let (project, canonical) = general_sem_schema6_moderation_bootstrap_authority_fixture(true);
        project.ensure_valid().unwrap();

        let mut gamma_table_tamper = canonical.clone();
        let gamma_table = moderation_test_table_mut(
            &mut gamma_table_tamper,
            GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1,
        );
        *moderation_test_number_cell_mut(&mut gamma_table.rows[0].cells[13]) += 0.125;
        assert_moderation_authority_rejects(
            &project,
            gamma_table_tamper,
            "gamma bootstrap mean differs bitwise",
        );

        let mut pipeline_tamper = canonical;
        moderation_test_table_mut(
            &mut pipeline_tamper,
            GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
        )
        .rows[0]
            .cells[35] = moderation_test_boolean(false);
        assert_moderation_authority_rejects(
            &project,
            pipeline_tamper,
            "score vectors sign aligned differs",
        );
    }

    #[test]
    fn general_sem_moderation_archive_rejects_extra_inventory_and_drifted_ownership() {
        let (project, canonical) = general_sem_schema6_moderation_authority_fixture(true);
        project.ensure_valid().unwrap();

        let mut extra_capability = canonical.clone();
        extra_capability
            .capability_cells
            .as_mut()
            .unwrap()
            .push(project_capability_cell_v2(
                &qpls_core::general_sem_pls_bootstrap_capability_cell_v1(),
            ));
        sort_project_capability_cells_v1(extra_capability.capability_cells.as_mut().unwrap());
        assert_moderation_authority_rejects(
            &project,
            extra_capability,
            "exact moderation document capability set",
        );

        let moderation_cell = canonical.provenance.capability_cell.clone();
        let mut extra_section = canonical.clone();
        extra_section.sections.push(CanonicalResultSectionV2 {
            id: "unexpected_moderation_section".into(),
            title: "Unexpected moderation section".into(),
            description: None,
            table_ids: Vec::new(),
            chart_ids: Vec::new(),
            capability_cells: Some(vec![moderation_cell.clone()]),
        });
        assert_moderation_authority_rejects(&project, extra_section, "section inventory or order");

        let base_cell = canonical.sections[0].capability_cells.as_ref().unwrap()[0].clone();
        let mut extra_table = canonical.clone();
        extra_table.tables.push(moderation_test_stage_one_table(
            "unexpected_stage_one_table",
            &base_cell,
        ));
        extra_table.sections[0]
            .table_ids
            .push("unexpected_stage_one_table".into());
        assert_moderation_authority_rejects(
            &project,
            extra_table,
            "section table/chart membership or order",
        );

        let mut extra_chart = canonical.clone();
        let mut chart = extra_chart.charts[0].clone();
        chart.id = "unexpected_moderation_chart".into();
        extra_chart.charts.push(chart);
        extra_chart.sections[3]
            .chart_ids
            .push("unexpected_moderation_chart".into());
        assert_moderation_authority_rejects(
            &project,
            extra_chart,
            "section table/chart membership or order",
        );

        let mut extra_exclusion = canonical.clone();
        let mut exclusion = extra_exclusion.exclusions[0].clone();
        exclusion.id = "unexpected_moderation_exclusion".into();
        extra_exclusion.exclusions.push(exclusion);
        assert_moderation_authority_rejects(&project, extra_exclusion, "exclusion inventory");

        let mut drifted_owner = canonical;
        let table = moderation_test_table_mut(&mut drifted_owner, "estimation_summary");
        table.capability_cells = Some(vec![moderation_cell.clone()]);
        let mut section_cells = vec![base_cell, moderation_cell];
        sort_project_capability_cells_v1(&mut section_cells);
        drifted_owner.sections[0].capability_cells = Some(section_cells);
        assert_moderation_authority_rejects(
            &project,
            drifted_owner,
            "stage-one moderation sections",
        );
    }

    #[test]
    fn general_sem_moderation_archive_reconciles_every_typed_table_and_chart_value() {
        let (project, canonical) = general_sem_schema6_moderation_authority_fixture(true);
        project.ensure_valid().unwrap();

        let mut structural_number = canonical.clone();
        let table = moderation_test_table_mut(&mut structural_number, "structural_paths");
        *moderation_test_number_cell_mut(&mut table.rows[0].cells[4]) += 0.125;
        assert_moderation_authority_rejects(&project, structural_number, "structural coefficient");

        let mut coherent_parameter_identity = canonical.clone();
        let relation_id = coherent_parameter_identity
            .general_sem_results
            .as_ref()
            .unwrap()
            .joint_stage_structural_coefficients[0]
            .relation_id
            .clone();
        coherent_parameter_identity
            .general_sem_results
            .as_mut()
            .unwrap()
            .joint_stage_structural_coefficients[0]
            .parameter_id = "parameter:coherent_tamper".into();
        let structural_table =
            moderation_test_table_mut(&mut coherent_parameter_identity, "structural_paths");
        let structural_row = structural_table
            .rows
            .iter_mut()
            .find(|row| {
                matches!(
                    row.cells.first(),
                    Some(CanonicalResultCellV2::Text { value }) if value == &relation_id
                )
            })
            .unwrap();
        structural_row.cells[1] = nonlinear_test_text("parameter:coherent_tamper");
        assert_moderation_authority_rejects(
            &project,
            coherent_parameter_identity,
            "compiled relation/parameter contract",
        );

        let mut product_scale_number = canonical.clone();
        let table = moderation_test_table_mut(
            &mut product_scale_number,
            GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1,
        );
        *moderation_test_number_cell_mut(&mut table.rows[0].cells[12]) += 0.25;
        assert_moderation_authority_rejects(
            &project,
            product_scale_number,
            "scientific rescaled gamma",
        );

        let mut conditional_cross_reference = canonical.clone();
        let table = moderation_test_table_mut(
            &mut conditional_cross_reference,
            GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1,
        );
        table.rows[0].cells[2] = nonlinear_test_text("relation:foreign_interaction_effect");
        assert_moderation_authority_rejects(
            &project,
            conditional_cross_reference,
            "conditional interaction effect",
        );

        let mut plot_number = canonical.clone();
        let table =
            moderation_test_table_mut(&mut plot_number, GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1);
        *moderation_test_number_cell_mut(&mut table.rows[0].cells[12]) += 0.375;
        assert_moderation_authority_rejects(&project, plot_number, "plot predicted value");

        let mut chart_number = canonical;
        chart_number.charts[0].series[0].points[0].y += 0.5;
        assert_moderation_authority_rejects(&project, chart_number, "point differs bitwise");
    }

    #[test]
    fn general_sem_moderation_archive_rejects_same_and_different_focal_coherent_tampering() {
        let (same_project, mut same_document) =
            general_sem_schema6_moderation_authority_fixture(true);
        let first_effect_id = same_document
            .general_sem_results
            .as_ref()
            .unwrap()
            .interaction_effects[0]
            .effect_id
            .clone();
        let zero_index = same_document
            .general_sem_results
            .as_ref()
            .unwrap()
            .conditional_effects
            .iter()
            .position(|effect| {
                effect.interaction_effect_id.as_deref() == Some(first_effect_id.as_str())
                    && effect.probe_value_index == 1
            })
            .unwrap();
        same_document
            .general_sem_results
            .as_mut()
            .unwrap()
            .conditional_effects[zero_index]
            .value
            .estimate += 0.2;
        let conditional_table = moderation_test_table_mut(
            &mut same_document,
            GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1,
        );
        *moderation_test_number_cell_mut(&mut conditional_table.rows[zero_index].cells[9]) += 0.2;
        assert_moderation_authority_rejects(
            &same_project,
            same_document,
            "zero-probe slope differs",
        );

        let (different_project, mut different_document) =
            general_sem_schema6_moderation_authority_fixture(false);
        let second_focal_relation_id = different_document
            .general_sem_results
            .as_ref()
            .unwrap()
            .interaction_effects[1]
            .focal_relation_id
            .clone();
        coherently_tamper_joint_stage_coefficient(
            &mut different_document,
            &second_focal_relation_id,
            0.3,
        );
        assert_moderation_authority_rejects(
            &different_project,
            different_document,
            "zero-probe slope differs",
        );
    }

    #[test]
    fn general_sem_moderation_archive_rejects_coherent_plan_digest_and_section_tampering() {
        let (project, canonical) = general_sem_schema6_moderation_authority_fixture(true);
        project.ensure_valid().unwrap();
        assert_eq!(
            canonical
                .general_sem_results
                .as_ref()
                .unwrap()
                .interaction_effects
                .len(),
            2
        );

        let mut coherent_identity_tamper = canonical.clone();
        let results = coherent_identity_tamper
            .general_sem_results
            .as_mut()
            .unwrap();
        let interaction_effect_id = results.interaction_effects[0].effect_id.clone();
        let tampered_interaction_id = "interaction:coherent_tamper".to_string();
        results.interaction_effects[0].interaction_id = tampered_interaction_id.clone();
        for effect in &mut results.conditional_effects {
            if effect.interaction_effect_id.as_deref() == Some(interaction_effect_id.as_str()) {
                effect.interaction_id = tampered_interaction_id.clone();
            }
        }
        for plot in &mut results.interaction_plots {
            if plot.interaction_effect_id.as_deref() == Some(interaction_effect_id.as_str()) {
                plot.interaction_id = tampered_interaction_id.clone();
            }
        }
        coherent_identity_tamper.ensure_valid().unwrap();
        let mut identity_project = project.clone();
        identity_project.canonical_result_documents = vec![
            CanonicalResultDocumentAttachmentV2::from_document(coherent_identity_tamper).unwrap(),
        ];
        assert!(matches!(
            identity_project.ensure_valid(),
            Err(ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message))
                if message.contains("compiled interaction")
        ));

        let mut coherent_digest_tamper = canonical.clone();
        for effect in &mut coherent_digest_tamper
            .general_sem_results
            .as_mut()
            .unwrap()
            .interaction_effects
        {
            effect.stage_one_model_scientific_sha256 = "0".repeat(64);
        }
        coherent_digest_tamper.ensure_valid().unwrap();
        let mut digest_project = project.clone();
        digest_project.canonical_result_documents = vec![
            CanonicalResultDocumentAttachmentV2::from_document(coherent_digest_tamper).unwrap(),
        ];
        assert!(matches!(
            digest_project.ensure_valid(),
            Err(ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message))
                if message.contains("stage-one projection digest")
        ));

        let mut non_interaction_section = canonical;
        let trace = non_interaction_section
            .general_sem_results
            .as_ref()
            .unwrap()
            .interaction_effects[0]
            .trace
            .clone();
        let relation_ids = non_interaction_section
            .general_sem_results
            .as_ref()
            .unwrap()
            .joint_stage_structural_coefficients
            .iter()
            .take(2)
            .map(|coefficient| coefficient.relation_id.clone())
            .collect::<Vec<_>>();
        let path_identity = qpls_core::specific_directed_path_identity_v1(&relation_ids);
        let model_id = non_interaction_section.provenance.model_id.clone();
        let results = non_interaction_section
            .general_sem_results
            .as_mut()
            .unwrap();
        results.specific_indirect_effects.push(
            qpls_core::CanonicalSpecificIndirectEffectResultV1 {
                effect_id: path_identity,
                estimand_id: "estimand:foreign_mediation".into(),
                trace: trace.clone(),
                source_id: "construct:x".into(),
                target_id: "construct:y".into(),
                ordered_relation_ids: relation_ids,
                value: moderation_test_point_estimate(0.1),
            },
        );
        results
            .higher_order_stages
            .push(qpls_core::CanonicalHocStageResultV1 {
                stage_id: "hoc_stage:foreign_lower_order".into(),
                trace: trace.clone(),
                higher_order_construct_id: "hoc:foreign".into(),
                stage_number: 1,
                kind: qpls_core::CanonicalHocStageKindV1::LowerOrderScoreEstimation,
                input_construct_ids: vec!["construct:x".into()],
                output_variable_ids: vec!["score:foreign_x".into()],
                relation_estimates: Vec::new(),
            });
        results
            .cbsem_fit
            .push(qpls_core::CanonicalCbsemFitResultV1 {
                fit_id: "cbsem_fit:foreign".into(),
                trace: trace.clone(),
                chi_square: 1.0,
                degrees_of_freedom: 1,
                chi_square_p_value: Some(0.5),
                rmsea: Some(0.01),
                rmsea_interval: None,
                cfi: Some(0.99),
                tli: Some(0.98),
                srmr: Some(0.02),
                aic: Some(10.0),
                bic: Some(12.0),
            });
        results
            .identification_diagnostics
            .push(qpls_core::CanonicalIdentificationDiagnosticV1 {
                diagnostic_id: "identification:unexpected_model_section".into(),
                trace,
                scope: qpls_core::CanonicalIdentificationScopeV1::Model,
                subject_id: model_id,
                status: qpls_core::CanonicalIdentificationStatusV1::Identified,
                code: "identified".into(),
                message: "Unexpected non-interaction result section.".into(),
                degrees_of_freedom: Some(0),
            });
        non_interaction_section.ensure_valid().unwrap();
        let mut section_project = project;
        section_project.canonical_result_documents = vec![
            CanonicalResultDocumentAttachmentV2::from_document(non_interaction_section).unwrap(),
        ];
        assert!(matches!(
            section_project.ensure_valid(),
            Err(ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message))
                if message.contains("point-only General SEM PLS interaction cell")
        ));
    }

    #[test]
    fn advanced_model_authoring_requires_the_general_sem_generation_marker() {
        let source = plan_project_upgrade_to_v6(
            &project_with_historical_result(AnalysisMethod::PlsPm),
            &request(),
        )
        .unwrap()
        .document;
        let mut advanced = explicit_model(&source).clone();
        assert!(!sem_model_requires_general_sem_v1(&advanced));

        advanced
            .derived_terms
            .push(qpls_core::SemDerivedTermV4::InteractionV2 {
                id: "term:three-way".into(),
                output: "derived:three-way".into(),
                operands: vec![
                    "construct:x".into(),
                    "construct:y".into(),
                    "construct:z".into(),
                ],
                focal_relation: "relation:x:y".into(),
                method: qpls_core::InteractionMethodV4::TwoStage,
                hierarchy_policy: qpls_core::InteractionHierarchyPolicyV2::Strong,
                product_indicator: None,
            });
        assert!(sem_model_requires_general_sem_v1(&advanced));
        assert!(matches!(
            ensure_general_sem_v1_model_authority(&source, &advanced),
            Err(ProjectArchiveV6Error::GeneralSemFeatureRequiresGeneration { subject })
                if subject.contains(&advanced.id)
        ));

        let general_sem = ProjectArchiveDocumentV6::new_general_sem_v1(
            Uuid::from_u128(0x67656e6572616c2d73656d2d61757468),
            "General SEM authority",
            Utc.with_ymd_and_hms(2026, 8, 18, 11, 0, 0).unwrap(),
        );
        ensure_general_sem_v1_model_authority(&general_sem, &advanced).unwrap();
    }

    #[test]
    fn migration_preserves_historical_recipe_and_binds_result_without_creating_recipe_v4() {
        let source = project_with_historical_result(AnalysisMethod::PlsPm);
        let plan = plan_project_upgrade_to_v6(&source, &request()).unwrap();
        let document = &plan.document;
        assert!(document.recipes.is_empty());
        assert_eq!(document.historical_recipes.len(), 1);
        let historical = &document.historical_recipes[0];
        let expected_value = serde_json::to_value(&source.recipes[0]).unwrap();
        assert_eq!(historical.recipe_id(), source.recipes[0].id);
        assert_eq!(
            historical.source_recipe_schema_version(),
            source.recipes[0].schema_version
        );
        assert_eq!(historical.recipe_document(), &expected_value);
        assert_eq!(
            historical.recipe_document_sha256(),
            sha256_json(&expected_value)
        );
        assert!(matches!(
            document.historical_results[0].source_recipe(),
            HistoricalResultRecipeBindingV6::Bound {
                source_recipe_id,
                recipe_document_sha256,
            } if *source_recipe_id == source.recipes[0].id
                && recipe_document_sha256 == historical.recipe_document_sha256()
        ));

        let mut reused_id = document.clone();
        reused_id.recipes.push(explicit_recipe_v4(
            &source.recipes[0],
            LegacyBasicModelInterpretationV4::PlsComposite,
            source.recipes[0].id,
        ));
        assert!(matches!(
            reused_id.ensure_valid(),
            Err(ProjectArchiveV6Error::DuplicateRecipeId(id)) if id == source.recipes[0].id
        ));

        let mut fresh = document.clone();
        fresh.recipes.push(explicit_recipe_v4(
            &source.recipes[0],
            LegacyBasicModelInterpretationV4::PlsComposite,
            Uuid::from_u128(9_002),
        ));
        fresh.ensure_valid().unwrap();
    }

    #[test]
    fn schema1_or_2_synthesized_result_provenance_remains_explicitly_unbound() {
        let mut source = project_with_historical_result(AnalysisMethod::PlsPm);
        source.source_archive_version = 2;
        source.recipes[0].schema_version = 2;
        source.recipes[0].method_config = None;
        let document = plan_project_upgrade_to_v6(&source, &request())
            .unwrap()
            .document;
        assert!(matches!(
            document.historical_results[0].source_recipe(),
            HistoricalResultRecipeBindingV6::UnboundLegacy
        ));
        document.ensure_valid().unwrap();
    }

    #[test]
    fn historical_recipe_and_result_binding_tampering_fail_closed() {
        let source = project_with_historical_result(AnalysisMethod::PlsPm);
        let document = plan_project_upgrade_to_v6(&source, &request())
            .unwrap()
            .document;
        let encoded = serialize_project_document_v6(&document).unwrap();

        let mut recipe_tamper: Value = serde_json::from_slice(&encoded).unwrap();
        recipe_tamper["historical_recipes"][0]["recipe_document"]["settings"]["seed"] =
            serde_json::json!(999);
        assert!(matches!(
            deserialize_project_document_v6(&serde_json::to_vec(&recipe_tamper).unwrap()),
            Err(ProjectArchiveV6Error::HistoricalRecipeDigest(_))
        ));

        let mut digest_tamper: Value = serde_json::from_slice(&encoded).unwrap();
        digest_tamper["historical_results"][0]["source_recipe"]["recipe_document_sha256"] =
            serde_json::json!("0".repeat(64));
        assert!(matches!(
            deserialize_project_document_v6(&serde_json::to_vec(&digest_tamper).unwrap()),
            Err(ProjectArchiveV6Error::HistoricalResultRecipeBinding(_))
        ));

        let mut invented_unbound: Value = serde_json::from_slice(&encoded).unwrap();
        invented_unbound["historical_results"][0]["source_recipe"] =
            serde_json::json!({"kind": "unbound_legacy"});
        assert!(matches!(
            deserialize_project_document_v6(&serde_json::to_vec(&invented_unbound).unwrap()),
            Err(ProjectArchiveV6Error::HistoricalResultRecipeBinding(_))
        ));
    }

    #[test]
    fn draft_payload_is_document_bound_and_cannot_bind_a_recipe() {
        let source = project_with_historical_result(AnalysisMethod::PlsPm);
        let mut document = plan_project_upgrade_to_v6(&source, &request())
            .unwrap()
            .document;
        make_first_model_a_draft(&mut document);
        document.ensure_valid().unwrap();
        let encoded = serialize_project_document_v6(&document).unwrap();
        let value: Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(value["models"][0]["payload"]["kind"], "sem_model_v4_draft");
        assert!(
            value["models"][0]["payload"]
                .get("scientific_sha256")
                .is_none()
        );

        let mut tampered: Value = serde_json::from_slice(&encoded).unwrap();
        tampered["models"][0]["payload"]["model"]["name"] = "Tampered".into();
        assert!(matches!(
            deserialize_project_document_v6(&serde_json::to_vec(&tampered).unwrap()),
            Err(ProjectArchiveV6Error::ModelDigestOrIdentity(_))
        ));

        document.recipes.push(explicit_recipe_v4(
            &source.recipes[0],
            LegacyBasicModelInterpretationV4::PlsComposite,
            Uuid::from_u128(9_003),
        ));
        assert!(matches!(
            document.ensure_valid(),
            Err(ProjectArchiveV6Error::RecipeModelReference { .. })
        ));
    }

    #[test]
    fn pls_legacy_model_auto_converts_but_covariance_drawing_remains_an_annotation() {
        let mut project = Project::new("Migration");
        project.models.push(legacy_model());
        project.recipes.push(recipe(AnalysisMethod::PlsPm));
        let plan = plan_project_upgrade_to_v6(&project, &request()).unwrap();
        assert!(plan.source_must_remain_unchanged);
        assert!(plan.destination_must_be_new);
        let ProjectModelPayloadV6::SemModelV4 { model, .. } = &plan.document.models[0].payload
        else {
            panic!("PLS model should convert to a composite SemModelV4");
        };
        assert!(model.annotations.iter().any(|annotation| matches!(
            annotation,
            qpls_core::SemAnnotationV4::DisplayOnlyCovariance { .. }
        )));
        assert!(
            !model
                .relations
                .iter()
                .any(|relation| matches!(relation, qpls_core::SemRelationV4::Covariance { .. }))
        );
        assert!(plan.document.recipes.is_empty());
        assert_eq!(plan.document.historical_recipes.len(), 1);
        assert_eq!(
            plan.document.historical_recipes[0].recipe_id(),
            project.recipes[0].id
        );
        assert_eq!(
            plan.document.historical_recipes[0].recipe_document(),
            &serde_json::to_value(&project.recipes[0]).unwrap()
        );
    }

    #[test]
    fn schema6_weight_declarations_round_trip_with_exact_resolved_provenance() {
        let cases = [
            (
                SemWeightBindingV4::Case {
                    variable: "observed:weight".into(),
                },
                serde_json::json!({
                    "kind": "case",
                    "variable_id": "observed:weight",
                    "source_column": "survey_weight"
                }),
            ),
            (
                SemWeightBindingV4::Frequency {
                    variable: "observed:weight".into(),
                },
                serde_json::json!({
                    "kind": "frequency",
                    "variable_id": "observed:weight",
                    "source_column": "survey_weight"
                }),
            ),
            (
                SemWeightBindingV4::Sampling {
                    variable: "observed:weight".into(),
                    normalization: SamplingWeightNormalizationV4::None,
                },
                serde_json::json!({
                    "kind": "sampling",
                    "variable_id": "observed:weight",
                    "source_column": "survey_weight",
                    "normalization": "none"
                }),
            ),
            (
                SemWeightBindingV4::Sampling {
                    variable: "observed:weight".into(),
                    normalization: SamplingWeightNormalizationV4::MeanOne,
                },
                serde_json::json!({
                    "kind": "sampling",
                    "variable_id": "observed:weight",
                    "source_column": "survey_weight",
                    "normalization": "mean_one"
                }),
            ),
            (
                SemWeightBindingV4::Sampling {
                    variable: "observed:weight".into(),
                    normalization: SamplingWeightNormalizationV4::SumToSampleSize,
                },
                serde_json::json!({
                    "kind": "sampling",
                    "variable_id": "observed:weight",
                    "source_column": "survey_weight",
                    "normalization": "sum_to_sample_size"
                }),
            ),
        ];
        for (binding, expected_binding) in cases {
            let document = document_with_weight_binding(binding);
            let bytes = serialize_project_document_v6(&document).unwrap();
            let reopened = deserialize_project_document_v6(&bytes).unwrap();
            let declaration = resolve_weight_declaration_v1(explicit_model(&reopened))
                .unwrap()
                .unwrap();
            assert_eq!(declaration.contract_version(), "sem_weight_declaration_v1");
            assert_eq!(declaration.dataset_id(), "dataset:survey");
            assert_eq!(
                serde_json::to_value(declaration.binding()).unwrap(),
                expected_binding
            );
        }
    }

    #[test]
    fn schema6_rejects_weight_kind_variable_and_normalization_tampering() {
        let mut kind = document_with_weight_binding(SemWeightBindingV4::Case {
            variable: "observed:weight".into(),
        });
        let SemDataBindingV4::Raw { weight, .. } = &mut explicit_model_mut(&mut kind).data_binding
        else {
            unreachable!()
        };
        *weight = Some(SemWeightBindingV4::Frequency {
            variable: "observed:weight".into(),
        });
        refresh_stored_model_digest_only(&mut kind);
        assert!(matches!(
            kind.ensure_valid(),
            Err(ProjectArchiveV6Error::RecipeModelDigest(_))
        ));

        let mut variable = document_with_weight_binding(SemWeightBindingV4::Case {
            variable: "observed:weight".into(),
        });
        let SemDataBindingV4::Raw { weight, .. } =
            &mut explicit_model_mut(&mut variable).data_binding
        else {
            unreachable!()
        };
        *weight = Some(SemWeightBindingV4::Case {
            variable: "observed:alternate_weight".into(),
        });
        refresh_stored_model_digest_only(&mut variable);
        assert!(matches!(
            variable.ensure_valid(),
            Err(ProjectArchiveV6Error::RecipeModelDigest(_))
        ));

        let mut normalization = document_with_weight_binding(SemWeightBindingV4::Sampling {
            variable: "observed:weight".into(),
            normalization: SamplingWeightNormalizationV4::MeanOne,
        });
        let SemDataBindingV4::Raw { weight, .. } =
            &mut explicit_model_mut(&mut normalization).data_binding
        else {
            unreachable!()
        };
        *weight = Some(SemWeightBindingV4::Sampling {
            variable: "observed:weight".into(),
            normalization: SamplingWeightNormalizationV4::SumToSampleSize,
        });
        refresh_stored_model_digest_only(&mut normalization);
        assert!(matches!(
            normalization.ensure_valid(),
            Err(ProjectArchiveV6Error::RecipeModelDigest(_))
        ));
    }

    #[test]
    fn legacy_wpls_recipe_remains_historical_and_is_not_compiled_as_recipe_v4() {
        let mut project = Project::new("Legacy WPLS declaration boundary");
        project.models.push(legacy_model());
        let mut legacy_recipe = recipe(AnalysisMethod::Wpls);
        legacy_recipe.method_config = Some(MethodConfig::Wpls);
        legacy_recipe.settings.case_weight_column = Some("legacy_case_weight".into());
        project.recipes.push(legacy_recipe);
        let document = plan_project_upgrade_to_v6(&project, &request())
            .unwrap()
            .document;
        document.ensure_valid().unwrap();
        assert!(matches!(
            &explicit_model(&document).data_binding,
            SemDataBindingV4::Raw { weight: None, .. }
        ));

        let bytes = serialize_project_document_v6(&document).unwrap();
        let reopened = deserialize_project_document_v6(&bytes).unwrap();
        assert!(reopened.recipes.is_empty());
        assert_eq!(reopened.historical_recipes.len(), 1);
        assert_eq!(
            reopened.historical_recipes[0].recipe_document()["settings"]["case_weight_column"],
            "legacy_case_weight"
        );
    }

    #[test]
    fn schema6_copies_reserved_lineage_after_v5_replay_and_revalidates_descriptors_only() {
        let dataset = qpls_data::import_delimited_bytes(
            b"x,y\n1,2\n3,4\n",
            "source.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let mut project = project_with_historical_result(AnalysisMethod::PlsPm);
        project.datasets.push(dataset.clone());
        crate::write_project_data_lineage_v1(
            &mut project.layouts,
            &crate::ProjectDataLineageV1 {
                schema_version: crate::PROJECT_DATA_LINEAGE_SCHEMA_VERSION_V1,
                records: vec![crate::ProjectDatasetVersionRecordV1 {
                    dataset_id: dataset.id.to_string(),
                    parent_dataset_id: None,
                    operation: crate::ProjectDatasetVersionOperationV1::Import,
                    created_at: None,
                    summary: "Imported source".into(),
                    source_column: None,
                    target_column: None,
                    transformation: None,
                }],
            },
        )
        .unwrap();
        project.layouts.insert(
            "unreserved_layout".into(),
            serde_json::json!({"preserved": true}),
        );

        let plan = plan_project_upgrade_to_v6(&project, &request()).unwrap();
        assert_eq!(
            plan.document.layouts[crate::PROJECT_DATA_LINEAGE_LAYOUT_KEY_V1],
            project.layouts[crate::PROJECT_DATA_LINEAGE_LAYOUT_KEY_V1]
        );
        assert_eq!(
            plan.document.layouts["unreserved_layout"],
            serde_json::json!({"preserved": true})
        );

        let mut malformed = plan.document;
        malformed
            .layouts
            .get_mut(crate::PROJECT_DATA_LINEAGE_LAYOUT_KEY_V1)
            .unwrap()["schemaVersion"] = serde_json::json!(2);
        assert!(matches!(
            malformed.ensure_valid(),
            Err(ProjectArchiveV6Error::DataLineage(
                ProjectDataLineageV1Error::UnsupportedSchema(2)
            ))
        ));
    }

    #[test]
    fn cbsem_legacy_reflective_model_auto_converts_to_common_factors() {
        let mut project = Project::new("CB-SEM migration");
        project.models.push(legacy_model());
        project.recipes.push(recipe(AnalysisMethod::Cbsem));
        let plan = plan_project_upgrade_to_v6(&project, &request()).unwrap();
        let ProjectModelPayloadV6::SemModelV4 { model, .. } = &plan.document.models[0].payload
        else {
            panic!("CB-SEM model should convert to a common-factor SemModelV4");
        };
        assert_eq!(
            model
                .variables
                .iter()
                .filter(|variable| matches!(
                    variable,
                    qpls_core::SemVariableV4::CommonFactor { .. }
                ))
                .count(),
            2
        );
        assert!(plan.document.recipes.is_empty());
        assert_eq!(plan.document.historical_recipes.len(), 1);
        assert_eq!(
            plan.document.historical_recipes[0].recipe_document(),
            &serde_json::to_value(&project.recipes[0]).unwrap()
        );
    }

    #[test]
    fn method_neutral_model_stays_pending_until_explicit_confirmation() {
        let mut project = Project::new("Migration");
        project.models.push(legacy_model());
        project.recipes.push(recipe(AnalysisMethod::Pca));
        let plan = plan_project_upgrade_to_v6(&project, &request()).unwrap();
        assert!(matches!(
            plan.document.models[0].payload,
            ProjectModelPayloadV6::LegacyEstimandUnspecified { .. }
        ));
        assert!(plan.document.recipes.is_empty());
        assert_eq!(plan.document.historical_recipes.len(), 1);

        let confirmed = confirm_project_legacy_estimand_v6(
            &plan.document,
            &legacy_model().id.to_string(),
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        assert!(matches!(
            confirmed.models[0].payload,
            ProjectModelPayloadV6::SemModelV4 { .. }
        ));
        assert!(confirmed.recipes.is_empty());
        assert_eq!(
            confirmed.historical_recipes,
            plan.document.historical_recipes
        );
    }

    #[test]
    fn historical_result_value_and_digest_are_unchanged_by_estimand_confirmation() {
        let mut project = Project::new("Migration");
        project.models.push(legacy_model());
        project.recipes.push(recipe(AnalysisMethod::Pca));
        let recipe_id = project.recipes[0].id;
        let timestamp = Utc.timestamp_opt(1_700_000_001, 0).unwrap();
        project.results.push(qpls_core::AnalysisResult {
            schema_version: RESULT_SCHEMA_VERSION,
            id: Uuid::from_u128(30),
            status: RunStatus::Completed,
            provenance: RunProvenance {
                recipe_id,
                dataset_fingerprint: "dataset".into(),
                method: AnalysisMethod::Pca,
                method_version: "historical".into(),
                engine_version: "historical".into(),
                seed: 1,
                settings: project.recipes[0].settings.clone(),
                started_at: timestamp,
                completed_at: timestamp,
            },
            diagnostics: Vec::new(),
            payload: AnalysisPayload::Legacy {
                value: serde_json::json!({"coefficient": 0.25}),
            },
        });
        let plan = plan_project_upgrade_to_v6(&project, &request()).unwrap();
        let before_value = plan.document.historical_results[0].result().clone();
        let before_hash = plan.document.historical_results[0]
            .result_sha256()
            .to_owned();
        let before_recipe_binding = plan.document.historical_results[0].source_recipe().clone();
        let confirmed = confirm_project_legacy_estimand_v6(
            &plan.document,
            &legacy_model().id.to_string(),
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        assert_eq!(confirmed.historical_results[0].result(), &before_value);
        assert_eq!(confirmed.historical_results[0].result_sha256(), before_hash);
        assert_eq!(
            confirmed.historical_results[0].source_recipe(),
            &before_recipe_binding
        );
    }

    #[test]
    fn tampered_historical_result_and_same_path_upgrade_fail_closed() {
        let mut project = Project::new("Migration");
        project.models.push(legacy_model());
        project.recipes.push(recipe(AnalysisMethod::PlsPm));
        let plan = plan_project_upgrade_to_v6(&project, &request()).unwrap();
        let mut value = serde_json::to_value(&plan.document).unwrap();
        value["historical_results"] = serde_json::json!([{
            "result_id": Uuid::nil(),
            "source_result_schema_version": 1,
            "result": {"schema_version": 1, "id": Uuid::nil(), "changed": true},
            "result_sha256": "0".repeat(64)
        }]);
        let tampered: ProjectArchiveDocumentV6 = serde_json::from_value(value).unwrap();
        assert!(matches!(
            tampered.ensure_valid(),
            Err(ProjectArchiveV6Error::HistoricalResultDigest(_))
        ));

        let mut unsafe_request = request();
        unsafe_request.destination_archive_path = unsafe_request.source_archive_path.clone();
        assert!(matches!(
            plan_project_upgrade_to_v6(&project, &unsafe_request),
            Err(ProjectArchiveV6Error::DestinationMustBeNew)
        ));
    }

    #[test]
    fn future_document_is_visible_only_as_a_read_only_summary() {
        let bytes = br#"{"datasets":[{}],"models":[{},{}],"recipes":[{}],"results":[{}]}"#;
        let ProjectArchiveInspectionV6::FutureReadOnly(summary) =
            inspect_project_document_v6(7, bytes).unwrap()
        else {
            panic!("future archive should use the read-only path");
        };
        assert!(summary.read_only);
        assert_eq!(summary.model_count, 2);
        assert_eq!(summary.result_count, 1);
    }

    #[test]
    fn current_document_unknown_fields_and_duplicate_keys_fail_closed() {
        let mut project = Project::new("Migration");
        project.models.push(legacy_model());
        project.recipes.push(recipe(AnalysisMethod::PlsPm));
        let plan = plan_project_upgrade_to_v6(&project, &request()).unwrap();
        let mut value = serde_json::to_value(&plan.document).unwrap();
        value["unknown"] = Value::Bool(true);
        assert!(inspect_project_document_v6(6, &serde_json::to_vec(&value).unwrap()).is_err());
        assert!(
            inspect_project_document_v6(6, br#"{"schema_version":6,"schema_version":6}"#).is_err()
        );
    }

    #[test]
    fn deterministic_serialization_has_strict_semantic_readback() {
        let mut project = Project::new("Deterministic v6");
        project.models.push(legacy_model());
        project.recipes.push(recipe(AnalysisMethod::PlsPm));
        let plan = plan_project_upgrade_to_v6(&project, &request()).unwrap();

        let first = serialize_project_document_v6(&plan.document).unwrap();
        let second = serialize_project_document_v6(&plan.document).unwrap();
        assert_eq!(first, second);
        let decoded = deserialize_project_document_v6(&first).unwrap();
        assert_eq!(serialize_project_document_v6(&decoded).unwrap(), first);
        assert_eq!(decoded.project_id, plan.document.project_id);

        let mut duplicate = String::from_utf8(first.clone()).unwrap();
        duplicate = duplicate.replacen(
            "\"schema_version\":6",
            "\"schema_version\":6,\"schema_version\":6",
            1,
        );
        assert!(matches!(
            deserialize_project_document_v6(duplicate.as_bytes()),
            Err(ProjectArchiveV6Error::LegacyArchiveLayer(_))
        ));

        let mut tampered: Value = serde_json::from_slice(&first).unwrap();
        tampered["historical_results"] = serde_json::json!([{
            "result_id": Uuid::nil(),
            "source_result_schema_version": 1,
            "result": {"schema_version": 1, "id": Uuid::nil(), "changed": true},
            "result_sha256": "0".repeat(64)
        }]);
        assert!(matches!(
            deserialize_project_document_v6(&serde_json::to_vec(&tampered).unwrap()),
            Err(ProjectArchiveV6Error::HistoricalResultDigest(_))
        ));
    }

    #[test]
    fn canonical_result_digest_preserves_exact_f64_values_across_archive_json() {
        let mut project = Project::new("Exact numeric canonical result");
        project.models.push(legacy_model());
        project.recipes.push(recipe(AnalysisMethod::PlsPm));
        let plan = plan_project_upgrade_to_v6(&project, &request()).unwrap();
        let mut canonical = canonical_result_document(
            plan.document.project_id,
            "result.numeric:1",
            "run-numeric-1",
        );
        canonical.tables[0]
            .columns
            .push(crate::CanonicalResultColumnV2 {
                id: "loading".to_string(),
                label: "Loading".to_string(),
                data_type: crate::CanonicalColumnTypeV2::Number,
                description: "Archive round-trip regression value".to_string(),
                role: Some(crate::CanonicalColumnRoleV2::Estimate),
                unit: None,
                default_precision: Some(12),
            });
        let value = 0.995_439_694_535_406_3_f64;
        canonical.tables[0].rows[0]
            .cells
            .push(crate::CanonicalResultCellV2::Number {
                value,
                display: None,
            });
        canonical.ensure_valid().unwrap();

        let attached = attach_canonical_result_document_v2_v6(&plan.document, canonical).unwrap();
        let stored_digest = attached.canonical_result_documents[0]
            .canonical_document_sha256()
            .to_owned();
        let reopened =
            deserialize_project_document_v6(&serialize_project_document_v6(&attached).unwrap())
                .unwrap();
        assert_eq!(
            reopened.canonical_result_documents[0].canonical_document_sha256(),
            stored_digest
        );
        let crate::CanonicalResultCellV2::Number {
            value: reopened_value,
            ..
        } = &reopened.canonical_result_documents[0]
            .canonical_document()
            .tables[0]
            .rows[0]
            .cells[1]
        else {
            panic!("numeric canonical result cell must remain numeric");
        };
        assert_eq!(reopened_value.to_bits(), value.to_bits());
    }

    #[test]
    fn canonical_result_attachment_saves_reopens_and_preserves_historical_result() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("canonical-result-v6.json");
        let mut upgrade_request = request();
        upgrade_request.destination_archive_path = destination.to_str().unwrap().to_owned();
        let plan = plan_project_upgrade_to_v6(
            &project_with_historical_result(AnalysisMethod::PlsPm),
            &upgrade_request,
        )
        .unwrap();
        let historical_value = plan.document.historical_results[0].result().clone();
        let historical_digest = plan.document.historical_results[0]
            .result_sha256()
            .to_owned();
        let canonical =
            canonical_result_document(plan.document.project_id, "result.document:1", "run-1");
        let expected_document = canonical.clone();
        let expected_digest = crate::canonical_result_document_v2_sha256(&canonical).unwrap();
        let attached = attach_canonical_result_document_v2_v6(&plan.document, canonical).unwrap();

        assert_eq!(attached.historical_results[0].result(), &historical_value);
        assert_eq!(
            attached.historical_results[0].result_sha256(),
            historical_digest
        );
        assert_eq!(attached.canonical_result_documents.len(), 1);
        assert!(attached.canonical_result_documents[0].immutable());
        assert_eq!(
            attached.canonical_result_documents[0].canonical_document(),
            &expected_document
        );
        assert_eq!(
            attached.canonical_result_documents[0].canonical_document_sha256(),
            expected_digest
        );

        write_project_document_v6_new(&destination, &attached).unwrap();
        let reopened = read_project_document_v6(&destination).unwrap();
        assert_eq!(
            reopened.canonical_result_documents,
            attached.canonical_result_documents
        );
        assert_eq!(reopened.historical_results[0].result(), &historical_value);
        assert_eq!(
            serialize_project_document_v6(&reopened).unwrap(),
            fs::read(destination).unwrap()
        );
    }

    #[test]
    fn canonical_result_file_append_preserves_origin_draft_and_historical_envelopes() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("append-canonical-result-v6.json");
        let source = directory.path().join("legacy.qpls");
        let source_bytes = b"immutable legacy archive fixture";
        fs::write(&source, source_bytes).unwrap();
        let mut plan = filesystem_plan(&source, &destination, source_bytes);
        make_first_model_a_draft(&mut plan.document);
        plan.ensure_valid().unwrap();
        let origin_before = serde_json::to_value(&plan.document.origin).unwrap();
        let models_before = serde_json::to_value(&plan.document.models).unwrap();
        let recipes_before = serde_json::to_value(&plan.document.historical_recipes).unwrap();
        let results_before = serde_json::to_value(&plan.document.historical_results).unwrap();
        write_project_document_v6_new(&destination, &plan.document).unwrap();
        let historical_value = plan.document.historical_results[0].result().clone();
        let historical_digest = plan.document.historical_results[0]
            .result_sha256()
            .to_owned();
        let before = fs::read(&destination).unwrap();
        let before_sha256 = sha256_bytes(&before);
        let canonical = canonical_result_document(
            plan.document.project_id,
            "result.document:append",
            "run-append",
        );

        let receipt =
            append_canonical_result_document_v2_file_v6(&destination, &before_sha256, canonical)
                .unwrap();

        assert_eq!(receipt.source_document_sha256, before_sha256);
        assert_eq!(
            receipt.updated_document_sha256,
            sha256_file_v6(&destination).unwrap()
        );
        assert_eq!(receipt.canonical_document_id, "result.document:append");
        assert_eq!(receipt.run_id, "run-append");
        assert_eq!(receipt.canonical_result_document_count, 1);
        assert!(receipt.source_verified_at_commit);
        assert!(receipt.post_write_validated);
        assert!(receipt.rollback_copy_removed);
        let reopened = read_project_document_v6(&destination).unwrap();
        assert_eq!(reopened.canonical_result_documents.len(), 1);
        assert_eq!(
            serde_json::to_value(&reopened.origin).unwrap(),
            origin_before
        );
        assert_eq!(
            serde_json::to_value(&reopened.models).unwrap(),
            models_before
        );
        assert_eq!(
            serde_json::to_value(&reopened.historical_recipes).unwrap(),
            recipes_before
        );
        assert_eq!(
            serde_json::to_value(&reopened.historical_results).unwrap(),
            results_before
        );
        assert_eq!(reopened.historical_results[0].result(), &historical_value);
        assert_eq!(
            reopened.historical_results[0].result_sha256(),
            historical_digest
        );
        assert_no_upgrade_temporary_files(directory.path());
    }

    #[test]
    fn canonical_result_file_append_cancels_before_commit_and_rejects_stale_or_locked_sources() {
        use std::cell::Cell;

        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("cancel-canonical-result-v6.json");
        let source = directory.path().join("legacy.qpls");
        let source_bytes = b"immutable legacy archive fixture";
        fs::write(&source, source_bytes).unwrap();
        let plan = filesystem_plan(&source, &destination, source_bytes);
        write_project_document_v6_new(&destination, &plan.document).unwrap();
        let before = fs::read(&destination).unwrap();
        let before_sha256 = sha256_bytes(&before);
        let checks = Cell::new(0_u32);
        let cancelled = append_canonical_result_document_v2_file_v6_with_cancel(
            &destination,
            &before_sha256,
            canonical_result_document(
                plan.document.project_id,
                "result.document:cancel",
                "run-cancel",
            ),
            || {
                checks.set(checks.get() + 1);
                checks.get() >= 2
            },
        );
        assert!(matches!(
            cancelled,
            Err(ProjectArchiveV6Error::AppendCancelled)
        ));
        assert_eq!(fs::read(&destination).unwrap(), before);
        assert_no_upgrade_temporary_files(directory.path());

        let stale = append_canonical_result_document_v2_file_v6(
            &destination,
            &"0".repeat(64),
            canonical_result_document(
                plan.document.project_id,
                "result.document:stale",
                "run-stale",
            ),
        );
        assert!(matches!(
            stale,
            Err(ProjectArchiveV6Error::SourceDigestMismatch { .. })
        ));
        assert_eq!(fs::read(&destination).unwrap(), before);

        let lock_path = append_private_path_v6(&destination, "lock").unwrap();
        fs::write(&lock_path, b"another writer").unwrap();
        let locked = append_canonical_result_document_v2_file_v6(
            &destination,
            &before_sha256,
            canonical_result_document(
                plan.document.project_id,
                "result.document:locked",
                "run-locked",
            ),
        );
        assert!(matches!(
            locked,
            Err(ProjectArchiveV6Error::AppendAlreadyInProgress(path)) if path == destination
        ));
        assert_eq!(fs::read(&destination).unwrap(), before);
        fs::remove_file(lock_path).unwrap();
        assert_no_upgrade_temporary_files(directory.path());
    }

    #[test]
    fn canonical_result_tampering_and_duplicate_attachment_fail_closed() {
        let plan = plan_project_upgrade_to_v6(
            &project_with_historical_result(AnalysisMethod::PlsPm),
            &request(),
        )
        .unwrap();
        let canonical =
            canonical_result_document(plan.document.project_id, "result.document:1", "run-1");
        let attached =
            attach_canonical_result_document_v2_v6(&plan.document, canonical.clone()).unwrap();
        assert!(matches!(
            attach_canonical_result_document_v2_v6(&attached, canonical),
            Err(ProjectArchiveV6Error::DuplicateCanonicalResultDocumentId(_))
        ));

        let mut tampered = serde_json::to_value(&attached).unwrap();
        tampered["canonical_result_documents"][0]["canonical_document"]["tables"][0]["rows"][0]["cells"]
            [0]["value"] = Value::String("changed".into());
        assert!(matches!(
            deserialize_project_document_v6(&serde_json::to_vec(&tampered).unwrap()),
            Err(ProjectArchiveV6Error::CanonicalResultDocument(
                CanonicalResultDocumentV2Error::DigestMismatch { .. }
            ))
        ));

        let mut unknown = serde_json::to_value(&attached).unwrap();
        unknown["canonical_result_documents"][0]["canonical_document"]["tables"][0]["rows"][0]["cells"]
            [0]["unknown"] = Value::Bool(true);
        assert!(matches!(
            deserialize_project_document_v6(&serde_json::to_vec(&unknown).unwrap()),
            Err(ProjectArchiveV6Error::Json(_))
        ));

        let mut wrong_cell_type =
            canonical_result_document(plan.document.project_id, "result.document:2", "run-2");
        wrong_cell_type.tables[0].rows[0].cells[0] = crate::CanonicalResultCellV2::Number {
            value: 0.25,
            display: None,
        };
        assert!(matches!(
            CanonicalResultDocumentAttachmentV2::from_document(wrong_cell_type),
            Err(CanonicalResultDocumentV2Error::Invalid(_))
        ));

        let mut wrong_project =
            canonical_result_document(plan.document.project_id, "result.document:3", "run-3");
        wrong_project.provenance.project_id = Uuid::from_u128(999).to_string();
        assert!(matches!(
            attach_canonical_result_document_v2_v6(&plan.document, wrong_project),
            Err(ProjectArchiveV6Error::CanonicalResultDocument(
                CanonicalResultDocumentV2Error::Invalid(_)
            ))
        ));
    }

    #[test]
    fn schema6_without_canonical_result_field_remains_readable_as_an_empty_attachment_set() {
        let plan = plan_project_upgrade_to_v6(
            &project_with_historical_result(AnalysisMethod::PlsPm),
            &request(),
        )
        .unwrap();
        let bytes = serialize_project_document_v6(&plan.document).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(value.get("canonical_result_documents").is_none());

        let reopened = deserialize_project_document_v6(&bytes).unwrap();
        assert!(reopened.canonical_result_documents.is_empty());
        assert_eq!(
            reopened.historical_results,
            plan.document.historical_results
        );
    }

    #[test]
    fn embedded_schema_inspection_keeps_future_documents_read_only() {
        let bytes = br#"{"schema_version":7,"datasets":[{}],"models":[{},{}],"recipes":[{}],"historical_results":[{}],"canonical_result_documents":[{},{}]}"#;
        let ProjectArchiveInspectionV6::FutureReadOnly(summary) =
            inspect_project_document_bytes_v6(bytes).unwrap()
        else {
            panic!("future archive should use the read-only inspection path");
        };
        assert_eq!(summary.schema_version, 7);
        assert_eq!(summary.document_sha256, sha256_bytes(bytes));
        assert_eq!(summary.result_count, 1);
        assert_eq!(summary.canonical_result_document_count, 2);
        assert!(summary.read_only);
        assert!(matches!(
            inspect_project_document_bytes_v6(br#"{"datasets":[]}"#),
            Err(ProjectArchiveV6Error::InvalidEmbeddedSchemaVersion)
        ));
    }

    #[test]
    fn upgrade_copy_preserves_source_and_historical_results_exactly() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("legacy.qpls");
        let destination = directory.path().join("upgraded-v6.json");
        let source_bytes = b"immutable legacy archive fixture";
        fs::write(&source, source_bytes).unwrap();
        let plan = filesystem_plan(&source, &destination, source_bytes);
        let historical_value = plan.document.historical_results[0].result().clone();
        let historical_sha256 = plan.document.historical_results[0]
            .result_sha256()
            .to_owned();

        let receipt = execute_project_upgrade_copy_v6(&source, &destination, &plan).unwrap();
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
        assert!(receipt.source_verified_unchanged);
        assert!(receipt.historical_results_immutable);
        assert_eq!(
            receipt.write.document_sha256,
            sha256_file_v6(&destination).unwrap()
        );

        let reopened = read_project_document_v6(&destination).unwrap();
        assert_eq!(reopened.historical_results[0].result(), &historical_value);
        assert_eq!(
            reopened.historical_results[0].result_sha256(),
            historical_sha256
        );
        assert_eq!(
            serialize_project_document_v6(&reopened).unwrap(),
            fs::read(&destination).unwrap()
        );
        assert_no_upgrade_temporary_files(directory.path());
    }

    #[test]
    fn destination_collision_never_overwrites_and_cleans_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("legacy.qpls");
        let destination = directory.path().join("existing-v6.json");
        let source_bytes = b"legacy source";
        let destination_bytes = b"existing destination";
        fs::write(&source, source_bytes).unwrap();
        fs::write(&destination, destination_bytes).unwrap();
        let plan = filesystem_plan(&source, &destination, source_bytes);

        assert!(matches!(
            execute_project_upgrade_copy_v6(&source, &destination, &plan),
            Err(ProjectArchiveV6Error::DestinationExists(path)) if path == destination
        ));
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
        assert_eq!(fs::read(&destination).unwrap(), destination_bytes);
        assert_no_upgrade_temporary_files(directory.path());
    }

    #[test]
    fn interrupted_or_tampered_temporary_write_is_cleaned_without_a_destination() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("legacy.qpls");
        let interrupted_destination = directory.path().join("interrupted-v6.json");
        let source_bytes = b"legacy source";
        fs::write(&source, source_bytes).unwrap();
        let interrupted_plan = filesystem_plan(&source, &interrupted_destination, source_bytes);

        let interrupted = write_project_document_v6_new_with_checks(
            &interrupted_destination,
            &interrupted_plan.document,
            |_| {
                Err(
                    std::io::Error::new(std::io::ErrorKind::Interrupted, "simulated interruption")
                        .into(),
                )
            },
            |_| Ok(()),
        );
        assert!(
            matches!(interrupted, Err(ProjectArchiveV6Error::Io(error)) if error.kind() == std::io::ErrorKind::Interrupted)
        );
        assert!(!interrupted_destination.exists());
        assert_no_upgrade_temporary_files(directory.path());

        let tampered_destination = directory.path().join("tampered-v6.json");
        let tampered_plan = filesystem_plan(&source, &tampered_destination, source_bytes);
        let tampered = write_project_document_v6_new_with_checks(
            &tampered_destination,
            &tampered_plan.document,
            |temporary| {
                fs::write(temporary, b"{\"schema_version\":6,\"schema_version\":6}")?;
                Ok(())
            },
            |_| Ok(()),
        );
        assert!(matches!(
            tampered,
            Err(ProjectArchiveV6Error::TemporaryDocumentChanged)
        ));
        assert!(!tampered_destination.exists());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
        assert_no_upgrade_temporary_files(directory.path());

        let post_write_destination = directory.path().join("post-write-tampered-v6.json");
        let post_write_plan = filesystem_plan(&source, &post_write_destination, source_bytes);
        let post_write_tampered = write_project_document_v6_new_with_checks(
            &post_write_destination,
            &post_write_plan.document,
            |_| Ok(()),
            |persisted| {
                fs::write(persisted, b"post-write tamper")?;
                Ok(())
            },
        );
        assert!(matches!(
            post_write_tampered,
            Err(ProjectArchiveV6Error::PostWriteValidation)
        ));
        assert!(!post_write_destination.exists());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
        assert_no_upgrade_temporary_files(directory.path());
    }

    #[test]
    fn publication_race_cannot_replace_a_newly_created_destination() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("legacy.qpls");
        let destination = directory.path().join("raced-v6.json");
        let source_bytes = b"legacy source";
        fs::write(&source, source_bytes).unwrap();
        let plan = filesystem_plan(&source, &destination, source_bytes);

        let result = write_project_document_v6_new_with_checks(
            &destination,
            &plan.document,
            |_| {
                fs::write(&destination, b"racing writer")?;
                Ok(())
            },
            |_| Ok(()),
        );
        assert!(matches!(
            result,
            Err(ProjectArchiveV6Error::DestinationExists(path)) if path == destination
        ));
        assert_eq!(fs::read(&destination).unwrap(), b"racing writer");
        assert_no_upgrade_temporary_files(directory.path());
    }

    #[test]
    fn source_digest_and_path_must_match_upgrade_lineage_exactly() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("legacy.qpls");
        let destination = directory.path().join("upgraded-v6.json");
        let source_bytes = b"legacy source";
        fs::write(&source, source_bytes).unwrap();
        let plan = filesystem_plan(&source, &destination, source_bytes);

        fs::write(&source, b"changed source").unwrap();
        assert!(matches!(
            execute_project_upgrade_copy_v6(&source, &destination, &plan),
            Err(ProjectArchiveV6Error::SourceDigestMismatch { .. })
        ));
        assert!(!destination.exists());

        fs::write(&source, source_bytes).unwrap();
        let alias = directory.path().join("source-alias.qpls");
        fs::copy(&source, &alias).unwrap();
        assert!(matches!(
            execute_project_upgrade_copy_v6(&alias, &destination, &plan),
            Err(ProjectArchiveV6Error::UpgradePathBinding {
                field: "source_archive_path",
                ..
            })
        ));
        assert!(!destination.exists());
        assert_no_upgrade_temporary_files(directory.path());
    }
}
