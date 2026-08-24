use crate::project_schema_v6::{atomic_replace_with_rollback_v6, restore_rollback_v6};
use crate::{
    CanonicalResultDocumentAttachmentV2, CanonicalResultDocumentV2,
    PROJECT_ARCHIVE_SCHEMA_V6_VERSION, ProjectArchiveDocumentV6, ProjectArchiveV6Error,
    ProjectError, ProjectManifest, load_project_archive_v6, serialize_project_document_v6,
};
use arrow::{
    array::{
        ArrayRef, BooleanArray, Float64Array, StringArray, StringDictionaryBuilder, UInt8Array,
        UInt32Array, UInt64Array,
    },
    datatypes::{DataType, Field, Schema, UInt32Type},
    ipc::{reader::StreamReader, writer::StreamWriter},
    record_batch::RecordBatch,
};
use qpls_core::{
    AnalysisRecipeV4, MULTIMOD_RESULT_SIDECAR_DESCRIPTOR_V1_SCHEMA_VERSION,
    MULTIMOD_SIDECAR_MAX_BYTES_V1, MultiModAnalysisResultV1, MultimodResultSidecarDescriptorV1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const MULTIMOD_RESULT_ATTACHMENT_SCHEMA_VERSION_V1: u32 = 1;
pub const MULTIMOD_ARROW_SIDECAR_MEDIA_TYPE_V1: &str = "application/vnd.apache.arrow.stream";
pub const MULTIMOD_ARROW_SIDECAR_COMPRESSION_V1: &str = "zip_deflate";
pub const MULTIMOD_ARROW_SCHEMA_IDENTITY_CONTRACT_V1: &str =
    "qpls.multimod.arrow_schema_identity.v1";
pub const MULTIMOD_ARROW_EVIDENCE_CONTRACT_VERSION_V1: u32 = 1;
const MULTIMOD_ARROW_EVIDENCE_ROLE_METADATA_V1: &str = "qpls.multimod.evidence_role";
const MULTIMOD_ARROW_SCHEMA_CONTRACT_ID_METADATA_V1: &str = "qpls.multimod.schema_contract_id";
const MULTIMOD_ARROW_SCHEMA_CONTRACT_VERSION_METADATA_V1: &str =
    "qpls.multimod.schema_contract_version";
const MULTIMOD_ARROW_BASE_SCHEMA_SHA256_METADATA_V1: &str = "qpls.multimod.base_schema_sha256";
pub const MULTIMOD_RESULT_SCIENTIFIC_IDENTITY_CONTRACT_V1: &str =
    "qpls.multimod.result_scientific_identity.v1";

/// Stable, ordered schema projection used as the persisted Arrow identity.
/// `DataType::to_string` is Arrow's reversible logical-type representation;
/// it recursively includes nested field names, nullability, and sorted nested
/// metadata. Top-level field and schema metadata are sorted explicitly here.
#[derive(Serialize)]
struct CanonicalArrowSchemaIdentityV1 {
    contract: &'static str,
    fields: Vec<CanonicalArrowFieldIdentityV1>,
    metadata: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct CanonicalArrowFieldIdentityV1 {
    name: String,
    logical_type: String,
    nullable: bool,
    dictionary_is_ordered: Option<bool>,
    metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModResultAttachmentV1 {
    pub schema_version: u32,
    pub result_id: String,
    pub recipe_id: Uuid,
    pub result: MultiModAnalysisResultV1,
    pub result_sha256: String,
    pub identity_sha256: String,
    #[serde(default)]
    pub sidecars: Vec<MultimodResultSidecarDescriptorV1>,
}

impl MultiModResultAttachmentV1 {
    pub fn new(
        result_id: impl Into<String>,
        recipe_id: Uuid,
        result: MultiModAnalysisResultV1,
        sidecars: Vec<MultimodResultSidecarDescriptorV1>,
    ) -> Result<Self, MultiModArchiveErrorV1> {
        let result_id = result_id.into();
        let result_sha256 = sha256_json(&result)?;
        let identity_sha256 = multimod_result_identity_sha256_v1(&result)?;
        let attachment = Self {
            schema_version: MULTIMOD_RESULT_ATTACHMENT_SCHEMA_VERSION_V1,
            result_id,
            recipe_id,
            result,
            result_sha256,
            identity_sha256,
            sidecars,
        };
        attachment.ensure_valid()?;
        Ok(attachment)
    }

    pub fn ensure_valid(&self) -> Result<(), MultiModArchiveErrorV1> {
        if self.schema_version != MULTIMOD_RESULT_ATTACHMENT_SCHEMA_VERSION_V1 {
            return Err(MultiModArchiveErrorV1::AttachmentSchema(
                self.schema_version,
            ));
        }
        validate_stable_result_id(&self.result_id)?;
        self.result
            .ensure_valid()
            .map_err(|error| MultiModArchiveErrorV1::InvalidScientificResult(error.to_string()))?;
        if self.result.sidecars() != self.sidecars.as_slice() {
            return Err(MultiModArchiveErrorV1::ResultSidecarInventoryMismatch(
                self.result_id.clone(),
            ));
        }
        validate_sha256("result_sha256", &self.result_sha256)?;
        validate_sha256("identity_sha256", &self.identity_sha256)?;
        if sha256_json(&self.result)? != self.result_sha256 {
            return Err(MultiModArchiveErrorV1::ResultDigestMismatch(
                self.result_id.clone(),
            ));
        }
        if multimod_result_identity_sha256_v1(&self.result)? != self.identity_sha256 {
            return Err(MultiModArchiveErrorV1::ResultIdentityMismatch(
                self.result_id.clone(),
            ));
        }
        let mut entries = BTreeSet::new();
        for descriptor in &self.sidecars {
            validate_sidecar_descriptor_v1(&self.result_id, descriptor)?;
            if descriptor.identity_sha256 != self.identity_sha256 {
                return Err(MultiModArchiveErrorV1::SidecarIdentityMismatch(
                    descriptor.entry_name.clone(),
                ));
            }
            if !entries.insert(descriptor.entry_name.as_str()) {
                return Err(MultiModArchiveErrorV1::DuplicateSidecarEntry(
                    descriptor.entry_name.clone(),
                ));
            }
        }
        validate_multimod_sidecar_total_bytes_v1(&self.sidecars)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiModSidecarPayloadV1 {
    pub descriptor: MultimodResultSidecarDescriptorV1,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiModArchiveAppendReceiptV1 {
    pub schema_version: u32,
    pub project_id: Uuid,
    pub result_id: String,
    pub source_archive_sha256: String,
    pub updated_archive_sha256: String,
    pub sidecar_count: usize,
    pub source_verified_at_commit: bool,
    pub post_write_validated: bool,
    pub rollback_removed: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum MultiModArchiveErrorV1 {
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    ProjectV6(#[from] ProjectArchiveV6Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Arrow(#[from] arrow::error::ArrowError),
    #[error("MultiMod result attachment schema must equal 1 (found {0})")]
    AttachmentSchema(u32),
    #[error("invalid MultiMod result identifier {0}")]
    InvalidResultId(String),
    #[error("invalid SHA-256 field {0}")]
    InvalidSha256(&'static str),
    #[error("MultiMod result digest mismatch for {0}")]
    ResultDigestMismatch(String),
    #[error("MultiMod result identity mismatch for {0}")]
    ResultIdentityMismatch(String),
    #[error("MultiMod scientific result contract is invalid: {0}")]
    InvalidScientificResult(String),
    #[error("MultiMod result {0} embeds a sidecar inventory different from its attachment")]
    ResultSidecarInventoryMismatch(String),
    #[error("invalid MultiMod sidecar entry {0}")]
    InvalidSidecarEntry(String),
    #[error("duplicate MultiMod sidecar entry {0}")]
    DuplicateSidecarEntry(String),
    #[error("MultiMod sidecar identity mismatch for {0}")]
    SidecarIdentityMismatch(String),
    #[error("MultiMod sidecar digest mismatch for {0}")]
    SidecarDigestMismatch(String),
    #[error("MultiMod sidecar byte count mismatch for {0}")]
    SidecarSizeMismatch(String),
    #[error("MultiMod sidecar Arrow shape mismatch for {0}")]
    SidecarShapeMismatch(String),
    #[error("MultiMod sidecar Arrow schema identity mismatch for {0}")]
    SidecarSchemaMismatch(String),
    #[error("MultiMod sidecar exceeds the 512 MiB scientific archive cap")]
    SidecarTooLarge,
    #[error("MultiMod sidecar set exceeds the 512 MiB per-run scientific archive cap")]
    SidecarSetTooLarge,
    #[error("MultiMod sidecar payload set does not match the attachment descriptors")]
    SidecarSetMismatch,
    #[error("project archive already contains MultiMod result {0}")]
    DuplicateResult(String),
    #[error("project archive already contains Recipe V4 {0}")]
    DuplicateRecipe(Uuid),
    #[error("MultiMod result references unknown Recipe V4 {0}")]
    UnknownRecipe(Uuid),
    #[error("staged MultiMod Recipe V4 is invalid: {0}")]
    InvalidStagedRecipe(String),
    #[error("staged MultiMod Recipe V4 identity differs from its result attachment")]
    StagedRecipeIdentityMismatch,
    #[error("MultiMod canonical result binding is invalid: {0}")]
    CanonicalResultBinding(String),
    #[error("source archive changed before MultiMod result commit")]
    SourceChanged,
    #[error("MultiMod archive append was cancelled")]
    Cancelled,
    #[error("MultiMod archive rollback failed after validation error: {0}")]
    RollbackFailed(String),
}

fn attach_multimod_result_for_atomic_pair_v1(
    source: &ProjectArchiveDocumentV6,
    attachment: MultiModResultAttachmentV1,
) -> Result<ProjectArchiveDocumentV6, MultiModArchiveErrorV1> {
    source.ensure_valid()?;
    attachment.ensure_valid()?;
    if !source
        .recipes
        .iter()
        .any(|recipe| recipe.id == attachment.recipe_id)
    {
        return Err(MultiModArchiveErrorV1::UnknownRecipe(attachment.recipe_id));
    }
    if source
        .multimod_results
        .iter()
        .any(|current| current.result_id == attachment.result_id)
    {
        return Err(MultiModArchiveErrorV1::DuplicateResult(
            attachment.result_id,
        ));
    }
    let mut updated = source.clone();
    updated.multimod_results.push(attachment);
    updated
        .multimod_results
        .sort_by(|left, right| left.result_id.cmp(&right.result_id));
    Ok(updated)
}

pub fn multimod_result_identity_sha256_v1(
    result: &MultiModAnalysisResultV1,
) -> Result<String, MultiModArchiveErrorV1> {
    #[derive(Serialize)]
    struct Identity<'a> {
        contract: &'static str,
        result_without_sidecars: &'a MultiModAnalysisResultV1,
    }

    let mut scientific = result.clone();
    match &mut scientific {
        MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(value) => value.sidecars.clear(),
        MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(value) => value.sidecars.clear(),
        MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(value) => {
            value.sidecars.clear()
        }
        MultiModAnalysisResultV1::InterventionalMediationResultV1(value) => value.sidecars.clear(),
    }
    Ok(sha256_bytes(&serde_json::to_vec(&Identity {
        contract: MULTIMOD_RESULT_SCIENTIFIC_IDENTITY_CONTRACT_V1,
        result_without_sidecars: &scientific,
    })?))
}

/// Computes the deterministic scientific identity of an Arrow schema.
///
/// Field order is significant. Metadata map insertion order is not. Dictionary
/// IDs are intentionally excluded because Arrow does not treat them as logical
/// field equality and is removing their preservation; ordered-dictionary
/// semantics remain included.
pub fn multimod_arrow_schema_sha256_v1(schema: &Schema) -> Result<String, MultiModArchiveErrorV1> {
    let identity = CanonicalArrowSchemaIdentityV1 {
        contract: MULTIMOD_ARROW_SCHEMA_IDENTITY_CONTRACT_V1,
        fields: schema
            .fields()
            .iter()
            .map(|field| CanonicalArrowFieldIdentityV1 {
                name: field.name().clone(),
                logical_type: field.data_type().to_string(),
                nullable: field.is_nullable(),
                dictionary_is_ordered: field.dict_is_ordered(),
                metadata: field
                    .metadata()
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            })
            .collect(),
        metadata: schema
            .metadata
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    };
    Ok(sha256_bytes(&serde_json::to_vec(&identity)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrustedMultiModArrowSchemaV1 {
    Metadata,
    PartitionEntries,
    PermutationLedger,
    PairwisePointEstimates,
    PairwiseInference,
    OmnibusGroupPointEstimates,
    OmnibusInference,
    GroupBootstrapLedger,
    TargetLedger,
    GroupPointEstimates,
    ResampleLedger,
    MicomConstructs,
    MicomNullStatistics,
    GroupIndex,
    Membership,
    FimixAssignments,
    FimixCoefficients,
    FimixStartDiagnostics,
    FimixStartEffectiveSizes,
    HeterogeneityMultistartReceipts,
    HeterogeneityMultistartAssignments,
    HeterogeneityMultistartSignatures,
    HeterogeneityMultistartPosteriors,
    StartIndex,
    StartTrace,
    PosSegments,
    PosRSquared,
    PosParameterSignatures,
    PosOutcomeFitAudit,
    PosStartDiagnostics,
    PosCandidateRefitFailures,
    PooledCoefficients,
    SourceRowMap,
    HeterogeneityFimixInput,
    HeterogeneityBootstrapLedger,
    LabelMapping,
    LabelOverlap,
    TargetSummary,
    BcaJackknifeSummary,
    CommonMetricConstructGate,
    CommonMetricCompositionalPairs,
    CommonMetricStep3Pairs,
    CommonMetricBlockers,
    CommonMetricParameters,
    RequiredSourceColumns,
    AnalysisRows,
    ExcludedRows,
    RecordIndex,
    CaseRecords,
    CaseDrawRows,
    DeleteOneRecords,
    DeleteOneRetainedRows,
    FrequencyRecords,
    FrequencyCounts,
    StudentizedOuterRecords,
    StudentizedOuterDrawRows,
    StudentizedInnerRecords,
    StudentizedInnerDrawRows,
    StudentizedInnerTargetVectors,
}

macro_rules! multimod_schema_v1 {
    ($(($name:literal, $data_type:expr, $nullable:literal)),+ $(,)?) => {
        Schema::new(vec![$(Field::new($name, $data_type, $nullable)),+])
    };
}

fn dictionary_utf8_u32_type_v1() -> DataType {
    DataType::Dictionary(Box::new(DataType::UInt32), Box::new(DataType::Utf8))
}

fn trusted_multimod_arrow_schema_v1(kind: TrustedMultiModArrowSchemaV1) -> Schema {
    use TrustedMultiModArrowSchemaV1 as K;
    match kind {
        K::Metadata => multimod_schema_v1![
            ("evidence_kind", DataType::Utf8, false),
            ("field_name", DataType::Utf8, false),
            ("value_type", DataType::Utf8, false),
            ("string_value", DataType::Utf8, true),
            ("unsigned_value", DataType::UInt64, true),
            ("number_value", DataType::Float64, true),
            ("boolean_value", DataType::Boolean, true),
        ],
        K::PartitionEntries => multimod_schema_v1![
            ("replicate_index", DataType::UInt32, false),
            ("partition_sha256", DataType::Utf8, false),
        ],
        K::PermutationLedger => multimod_schema_v1![
            ("replicate_index", DataType::UInt32, false),
            ("partition_sha256", DataType::Utf8, false),
            ("replicate_usable", DataType::Boolean, false),
            ("group_index", DataType::UInt8, false),
            ("group_usable", DataType::Boolean, false),
            ("failure_code", DataType::Utf8, false),
            ("failure_detail", DataType::Utf8, false),
        ],
        K::PairwisePointEstimates => multimod_schema_v1![
            ("parameter_id", DataType::Utf8, false),
            ("parameter_family", DataType::Utf8, false),
            ("estimate_a", DataType::Float64, false),
            ("estimate_b", DataType::Float64, false),
            ("difference_a_minus_b", DataType::Float64, false),
        ],
        K::PairwiseInference => multimod_schema_v1![
            ("parameter_id", DataType::Utf8, false),
            ("parameter_family", DataType::Utf8, false),
            ("estimate_a", DataType::Float64, false),
            ("estimate_b", DataType::Float64, false),
            ("difference_a_minus_b", DataType::Float64, false),
            ("p_value_two_sided", DataType::Float64, false),
            ("p_value_greater", DataType::Float64, false),
            ("p_value_less", DataType::Float64, false),
            ("selected_alternative", DataType::Utf8, false),
            ("selected_probability", DataType::Float64, false),
        ],
        K::OmnibusGroupPointEstimates => multimod_schema_v1![
            ("group_index", DataType::UInt8, false),
            ("parameter_index", DataType::UInt32, false),
            ("parameter_id", DataType::Utf8, true),
            ("parameter_family", DataType::Utf8, true),
            ("estimate", DataType::Float64, false),
        ],
        K::OmnibusInference => multimod_schema_v1![
            ("parameter_id", DataType::Utf8, false),
            ("parameter_family", DataType::Utf8, false),
            ("observed_maximum_pairwise_spread", DataType::Float64, false),
            ("p_value_right_tailed", DataType::Float64, false),
        ],
        K::GroupBootstrapLedger => multimod_schema_v1![
            ("replicate_index", DataType::UInt32, false),
            ("replicate_usable", DataType::Boolean, false),
            ("group_index", DataType::UInt8, false),
            ("group_usable", DataType::Boolean, false),
            ("sample_sha256", DataType::Utf8, false),
            ("failure_code", DataType::Utf8, false),
            ("failure_detail", DataType::Utf8, false),
        ],
        K::TargetLedger => multimod_schema_v1![
            ("replicate_index", DataType::UInt32, false),
            ("target_id", dictionary_utf8_u32_type_v1(), false),
            ("value", DataType::Float64, true),
            ("valid", DataType::Boolean, false),
            ("failure_code", dictionary_utf8_u32_type_v1(), false),
        ],
        K::GroupPointEstimates => multimod_schema_v1![
            ("group_index", DataType::UInt8, false),
            ("parameter_id", DataType::Utf8, false),
            ("parameter_family", DataType::Utf8, false),
            ("estimate", DataType::Float64, false),
        ],
        K::ResampleLedger => multimod_schema_v1![
            ("replicate_index", DataType::UInt32, false),
            ("seed", DataType::UInt64, false),
            ("usable", DataType::Boolean, false),
            ("failure_code", DataType::Utf8, false),
            ("shard_id", DataType::Utf8, false),
        ],
        K::MicomConstructs => multimod_schema_v1![
            ("construct_id", DataType::Utf8, false),
            (
                "observed_compositional_correlation",
                DataType::Float64,
                false
            ),
            ("compositional_lower_quantile", DataType::Float64, true),
            (
                "compositional_invariance_probability",
                DataType::Float64,
                true
            ),
            ("compositional_invariance", DataType::Boolean, false),
            (
                "observed_mean_difference_a_minus_b",
                DataType::Float64,
                false
            ),
            (
                "mean_difference_two_sided_probability",
                DataType::Float64,
                true
            ),
            ("equal_means", DataType::Boolean, false),
            (
                "observed_log_variance_ratio_a_minus_b",
                DataType::Float64,
                false
            ),
            (
                "variance_difference_two_sided_probability",
                DataType::Float64,
                true
            ),
            ("equal_variances", DataType::Boolean, false),
            ("partial_measurement_invariance", DataType::Boolean, false),
            ("full_measurement_invariance", DataType::Boolean, false),
        ],
        // Construct identifiers are retained once in the sibling `constructs`
        // table. These ordinals therefore preserve the complete MICOM null
        // distribution without repeating UTF-8 identifiers for every draw.
        K::MicomNullStatistics => multimod_schema_v1![
            ("replicate_index", DataType::UInt32, false),
            ("construct_index", DataType::UInt32, false),
            ("statistic_kind", DataType::UInt8, false),
            ("value", DataType::Float64, false),
        ],
        K::GroupIndex => multimod_schema_v1![("group_index", DataType::UInt8, false)],
        K::Membership => multimod_schema_v1![
            ("row_index", DataType::UInt32, false),
            ("stable_row_token", DataType::Utf8, false),
            ("class_id", DataType::UInt8, false),
            ("posterior_probability", DataType::Float64, false),
        ],
        K::FimixAssignments => multimod_schema_v1![
            ("row_index", DataType::UInt32, false),
            ("stable_row_token", DataType::Utf8, false),
            ("hard_class_id", DataType::UInt8, false),
        ],
        K::FimixCoefficients => multimod_schema_v1![
            ("class_id", DataType::Utf8, false),
            ("class_proportion", DataType::Float64, false),
            ("class_effective_observations", DataType::Float64, false),
            ("equation_id", DataType::Utf8, false),
            ("outcome_id", DataType::Utf8, false),
            ("parameter_id", DataType::Utf8, false),
            ("coefficient", DataType::Float64, false),
            ("residual_variance", DataType::Float64, false),
        ],
        K::FimixStartDiagnostics => multimod_schema_v1![
            ("start_index", DataType::UInt32, false),
            ("start_seed", DataType::UInt64, false),
            ("converged", DataType::Boolean, false),
            ("iterations", DataType::UInt32, false),
            ("final_log_likelihood", DataType::Float64, true),
            ("maximum_likelihood_decrease", DataType::Float64, false),
            ("failure_code", DataType::Utf8, true),
            ("failure_message", DataType::Utf8, true),
        ],
        K::FimixStartEffectiveSizes => multimod_schema_v1![
            ("start_index", DataType::UInt32, false),
            ("class_id", DataType::UInt8, false),
            ("effective_observations", DataType::Float64, false),
        ],
        K::HeterogeneityMultistartReceipts => multimod_schema_v1![
            ("start_index", DataType::UInt32, false),
            ("fit_statistic", DataType::Float64, false),
            ("partition_sha256", DataType::Utf8, false),
            ("numeric_signature_sha256", DataType::Utf8, false),
            ("posterior_sha256", DataType::Utf8, true),
            ("fit_statistic_sha256", DataType::Utf8, false),
        ],
        K::HeterogeneityMultistartAssignments => multimod_schema_v1![
            ("start_index", DataType::UInt32, false),
            ("row_index", DataType::UInt32, false),
            ("class_id", DataType::UInt8, false),
        ],
        K::HeterogeneityMultistartSignatures => multimod_schema_v1![
            ("start_index", DataType::UInt32, false),
            ("class_id", DataType::UInt8, false),
            ("parameter_index", DataType::UInt32, false),
            ("value", DataType::Float64, false),
        ],
        K::HeterogeneityMultistartPosteriors => multimod_schema_v1![
            ("start_index", DataType::UInt32, false),
            ("row_index", DataType::UInt32, false),
            ("class_id", DataType::UInt8, false),
            ("posterior_probability", DataType::Float64, false),
        ],
        K::StartIndex => multimod_schema_v1![("start_index", DataType::UInt32, false)],
        K::StartTrace => multimod_schema_v1![
            ("algorithm", DataType::Utf8, false),
            ("k", DataType::UInt8, false),
            ("start_index", DataType::UInt32, false),
            ("iteration", DataType::UInt32, false),
            ("objective", DataType::Float64, false),
            ("converged", DataType::Boolean, false),
            ("failure_code", DataType::Utf8, false),
        ],
        K::PosSegments => multimod_schema_v1![
            ("segment_id", DataType::Utf8, false),
            ("observations", DataType::UInt64, false),
            ("objective_contribution", DataType::Float64, false),
            ("receipt_method_version", DataType::Utf8, false),
            ("full_segment_pls_refit", DataType::Boolean, false),
            ("measurement_scores_reestimated", DataType::Boolean, false),
            ("score_orientation_reapplied", DataType::Boolean, false),
            ("interaction_stage_one_refit", DataType::Boolean, false),
            (
                "interaction_operands_restandardized_within_destination",
                DataType::Boolean,
                false
            ),
            (
                "interaction_products_rebuilt_within_destination",
                DataType::Boolean,
                false
            ),
            ("joint_structural_equations_refit", DataType::Boolean, false),
        ],
        K::PosRSquared => multimod_schema_v1![
            ("segment_id", DataType::Utf8, false),
            ("outcome_id", DataType::Utf8, false),
            ("r_squared", DataType::Float64, false),
        ],
        K::PosParameterSignatures => multimod_schema_v1![
            ("segment_id", DataType::Utf8, false),
            ("parameter_index", DataType::UInt32, false),
            ("value", DataType::Float64, false),
        ],
        K::PosOutcomeFitAudit => multimod_schema_v1![
            ("segment_id", DataType::Utf8, false),
            ("outcome_id", DataType::Utf8, false),
            ("source_row_index", DataType::UInt32, false),
            ("observed_score", DataType::Float64, false),
            ("fitted_score", DataType::Float64, false),
            ("observed_mean", DataType::Float64, false),
            ("centered_total_sum_of_squares", DataType::Float64, false),
        ],
        K::PosStartDiagnostics => multimod_schema_v1![
            ("start_index", DataType::UInt32, false),
            ("completed", DataType::Boolean, false),
            ("accepted_moves", DataType::UInt32, false),
            ("final_objective", DataType::Float64, true),
            ("failure_reason", DataType::Utf8, true),
        ],
        K::PosCandidateRefitFailures => multimod_schema_v1![
            ("start_index", DataType::UInt32, false),
            ("observation", DataType::UInt32, false),
            ("source_segment", DataType::UInt8, false),
            ("destination_segment", DataType::UInt8, false),
            ("reason", DataType::Utf8, false),
        ],
        K::PooledCoefficients => multimod_schema_v1![
            ("equation_id", DataType::Utf8, false),
            ("outcome_id", DataType::Utf8, false),
            ("parameter_id", DataType::Utf8, false),
            ("estimate", DataType::Float64, false),
            ("residual_variance", DataType::Float64, false),
            ("r_squared", DataType::Float64, false),
        ],
        K::SourceRowMap => multimod_schema_v1![
            ("analysis_position", DataType::UInt32, false),
            ("source_row_token", DataType::UInt64, false),
        ],
        K::HeterogeneityFimixInput => multimod_schema_v1![
            ("equation_id", DataType::Utf8, false),
            ("outcome_id", DataType::Utf8, false),
            ("row_index", DataType::UInt32, false),
            ("include_intercept", DataType::Boolean, false),
            ("predictor_id", DataType::Utf8, false),
            ("predictor_value", DataType::Float64, false),
            ("outcome_value", DataType::Float64, false),
        ],
        K::HeterogeneityBootstrapLedger => multimod_schema_v1![
            ("replicate_index", DataType::UInt32, false),
            ("seed", DataType::UInt64, false),
            ("status", DataType::Utf8, false),
            ("fit_statistic", DataType::Float64, true),
            ("target_payload_sha256", DataType::Utf8, true),
            ("failure_reason", DataType::Utf8, true),
            ("alignment_matched_observations", DataType::UInt64, true),
            ("alignment_match_share", DataType::Float64, true),
            ("alignment_ambiguous", DataType::Boolean, true),
            ("alignment_mutual_majority", DataType::Boolean, true),
        ],
        K::LabelMapping => multimod_schema_v1![
            ("replicate_index", DataType::UInt32, false),
            ("candidate_label", DataType::UInt8, false),
            ("reference_label", DataType::UInt8, false),
        ],
        K::LabelOverlap => multimod_schema_v1![
            ("replicate_index", DataType::UInt32, false),
            ("reference_label", DataType::UInt8, false),
            ("candidate_label", DataType::UInt8, false),
            ("overlap_count", DataType::UInt64, false),
        ],
        K::TargetSummary => multimod_schema_v1![
            ("target_id", DataType::Utf8, false),
            ("observed_standard_error", DataType::Float64, true),
        ],
        K::BcaJackknifeSummary => multimod_schema_v1![
            ("target_id", DataType::Utf8, false),
            ("delete_one_count", DataType::UInt32, false),
            ("jackknife_mean", DataType::Float64, false),
            ("centered_cube_sum", DataType::Float64, false),
            ("centered_square_sum", DataType::Float64, false),
            ("acceleration", DataType::Float64, false),
            ("complete", DataType::Boolean, false),
        ],
        K::CommonMetricConstructGate => multimod_schema_v1![
            ("construct_id", DataType::Utf8, false),
            ("configural_identity_passed", DataType::Boolean, false),
            ("retained_by_gate", DataType::Boolean, false),
        ],
        K::CommonMetricCompositionalPairs => multimod_schema_v1![
            ("construct_id", DataType::Utf8, false),
            ("left_segment", DataType::UInt32, false),
            ("right_segment", DataType::UInt32, false),
            ("compositional_invariance_passed", DataType::Boolean, false),
            ("permutation_p_value", DataType::Float64, true),
        ],
        K::CommonMetricStep3Pairs => multimod_schema_v1![
            ("construct_id", DataType::Utf8, false),
            ("left_segment", DataType::UInt32, false),
            ("right_segment", DataType::UInt32, false),
            ("mean_equality_passed", DataType::Boolean, false),
            ("variance_equality_passed", DataType::Boolean, false),
        ],
        K::CommonMetricBlockers => multimod_schema_v1![
            ("blocker_code", DataType::Utf8, false),
            ("construct_id", DataType::Utf8, true),
            ("left_segment", DataType::UInt32, true),
            ("right_segment", DataType::UInt32, true),
            ("message", DataType::Utf8, false),
        ],
        K::CommonMetricParameters => multimod_schema_v1![
            ("segment_id", DataType::UInt8, false),
            ("target_id", DataType::Utf8, false),
            ("target_kind", DataType::Utf8, false),
            ("metric", DataType::Utf8, false),
            ("estimate", DataType::Float64, false),
            ("standard_error", DataType::Float64, true),
            ("p_value", DataType::Float64, true),
            ("interval_family", DataType::Utf8, true),
            ("interval_alternative", DataType::Utf8, true),
            ("interval_confidence_level", DataType::Float64, true),
            ("interval_lower", DataType::Float64, true),
            ("interval_upper", DataType::Float64, true),
        ],
        K::RequiredSourceColumns => multimod_schema_v1![
            ("column_index", DataType::UInt32, false),
            ("source_column", DataType::Utf8, false),
        ],
        K::AnalysisRows => multimod_schema_v1![
            ("stratum_index", DataType::UInt32, false),
            ("group_id", DataType::Utf8, true),
            ("stratum_position", DataType::UInt32, false),
            ("source_row", DataType::UInt32, false),
            ("stable_row_token", DataType::Utf8, false),
            ("case_weight", DataType::Float64, true),
            ("frequency", DataType::UInt64, true),
        ],
        K::ExcludedRows => multimod_schema_v1![
            ("source_row", DataType::UInt32, false),
            ("stable_row_token", DataType::Utf8, false),
            ("reason", DataType::Utf8, false),
        ],
        K::RecordIndex => multimod_schema_v1![("record_index", DataType::UInt32, false)],
        K::CaseRecords => multimod_schema_v1![
            ("record_index", DataType::UInt32, false),
            ("attempt_count", DataType::UInt8, false),
            ("draw_replicate_index", DataType::UInt32, false),
            ("source_rows_sha256", DataType::Utf8, false),
            ("case_weights_sha256", DataType::Utf8, true),
            ("draw_identity_sha256", DataType::Utf8, false),
            ("outcome_usable", DataType::Boolean, false),
            ("outcome_identity_sha256", DataType::Utf8, false),
            ("failure_code", DataType::Utf8, false),
            ("failure_message", DataType::Utf8, false),
            ("record_identity_sha256", DataType::Utf8, false),
        ],
        K::CaseDrawRows => multimod_schema_v1![
            ("record_index", DataType::UInt32, false),
            ("draw_position", DataType::UInt32, false),
            ("source_row", DataType::UInt32, false),
            ("case_weight", DataType::Float64, true),
        ],
        K::DeleteOneRecords => multimod_schema_v1![
            ("record_index", DataType::UInt32, false),
            ("attempt_count", DataType::UInt8, false),
            ("omitted_row", DataType::UInt32, false),
            ("retained_rows_sha256", DataType::Utf8, false),
            ("case_weights_sha256", DataType::Utf8, true),
            ("draw_identity_sha256", DataType::Utf8, false),
            ("outcome_usable", DataType::Boolean, false),
            ("outcome_identity_sha256", DataType::Utf8, false),
            ("failure_code", DataType::Utf8, false),
            ("failure_message", DataType::Utf8, false),
            ("record_identity_sha256", DataType::Utf8, false),
        ],
        K::DeleteOneRetainedRows => multimod_schema_v1![
            ("record_index", DataType::UInt32, false),
            ("retained_position", DataType::UInt32, false),
            ("source_row", DataType::UInt32, false),
            ("case_weight", DataType::Float64, true),
        ],
        K::FrequencyRecords => multimod_schema_v1![
            ("record_index", DataType::UInt32, false),
            ("attempt_count", DataType::UInt8, false),
            ("draw_replicate_index", DataType::UInt32, false),
            ("total_count", DataType::UInt64, false),
            ("counts_sha256", DataType::Utf8, false),
            ("draw_identity_sha256", DataType::Utf8, false),
            ("outcome_usable", DataType::Boolean, false),
            ("outcome_identity_sha256", DataType::Utf8, false),
            ("failure_code", DataType::Utf8, false),
            ("failure_message", DataType::Utf8, false),
            ("record_identity_sha256", DataType::Utf8, false),
        ],
        K::FrequencyCounts => multimod_schema_v1![
            ("record_index", DataType::UInt32, false),
            ("compact_position", DataType::UInt32, false),
            ("count", DataType::UInt64, false),
        ],
        K::StudentizedOuterRecords => multimod_schema_v1![
            ("outer_record_index", DataType::UInt32, false),
            ("attempt_count", DataType::UInt8, false),
            ("draw_replicate_index", DataType::UInt32, false),
            ("source_rows_sha256", DataType::Utf8, false),
            ("case_weights_sha256", DataType::Utf8, true),
            ("draw_identity_sha256", DataType::Utf8, false),
            ("outcome_usable", DataType::Boolean, false),
            ("outcome_identity_sha256", DataType::Utf8, false),
            ("failure_code", DataType::Utf8, false),
            ("failure_message", DataType::Utf8, false),
            ("record_identity_sha256", DataType::Utf8, false),
            ("inner_ledger_identity_sha256", DataType::Utf8, false),
            ("inner_record_count", DataType::UInt32, false),
            ("inner_usable_count", DataType::UInt32, false),
        ],
        K::StudentizedOuterDrawRows => multimod_schema_v1![
            ("outer_record_index", DataType::UInt32, false),
            ("draw_position", DataType::UInt32, false),
            ("source_row", DataType::UInt32, false),
            ("case_weight", DataType::Float64, true),
        ],
        K::StudentizedInnerRecords => multimod_schema_v1![
            ("outer_record_index", DataType::UInt32, false),
            ("inner_record_index", DataType::UInt32, false),
            ("attempt_count", DataType::UInt8, false),
            ("draw_outer_replicate_index", DataType::UInt32, false),
            ("draw_inner_replicate_index", DataType::UInt32, false),
            ("outer_draw_identity_sha256", DataType::Utf8, false),
            ("source_rows_sha256", DataType::Utf8, false),
            ("case_weights_sha256", DataType::Utf8, true),
            ("draw_identity_sha256", DataType::Utf8, false),
            ("outcome_usable", DataType::Boolean, false),
            ("outcome_identity_sha256", DataType::Utf8, false),
            ("failure_code", DataType::Utf8, false),
            ("failure_message", DataType::Utf8, false),
            ("record_identity_sha256", DataType::Utf8, false),
        ],
        K::StudentizedInnerDrawRows => multimod_schema_v1![
            ("outer_record_index", DataType::UInt32, false),
            ("inner_record_index", DataType::UInt32, false),
            ("draw_position", DataType::UInt32, false),
            ("source_row", DataType::UInt32, false),
            ("case_weight", DataType::Float64, true),
        ],
        K::StudentizedInnerTargetVectors => multimod_schema_v1![
            ("outer_record_index", DataType::UInt32, false),
            ("inner_record_index", DataType::UInt32, false),
            ("target_index", DataType::UInt32, false),
            ("value", DataType::Float64, true),
            ("valid", DataType::Boolean, false),
            ("failure_code", DataType::Utf8, false),
        ],
    }
}

fn strip_indexed_table_role_v1<'a>(table_role: &'a str, prefix: &str) -> Option<&'a str> {
    let remainder = table_role.strip_prefix(prefix)?.strip_prefix('-')?;
    let (index, suffix) = remainder.split_once('-')?;
    (index.len() == 2 && index.bytes().all(|byte| byte.is_ascii_digit())).then_some(suffix)
}

fn trusted_multimod_arrow_schema_kind_v1(
    evidence_role: &str,
) -> Result<TrustedMultiModArrowSchemaV1, MultiModArchiveErrorV1> {
    use TrustedMultiModArrowSchemaV1 as K;
    let (kind, table) = evidence_role
        .split_once(':')
        .ok_or_else(|| MultiModArchiveErrorV1::InvalidSidecarEntry(evidence_role.into()))?;
    let resolved = match (kind, table) {
        ("mga-pairwise-partition-plan" | "mga-frequency-pairwise-partition-plan", "metadata") => {
            K::Metadata
        }
        ("mga-pairwise-partition-plan" | "mga-frequency-pairwise-partition-plan", "entries") => {
            K::PartitionEntries
        }
        ("mga-pairwise-permutation", "metadata") => K::Metadata,
        ("mga-pairwise-permutation", "ledger") => K::PermutationLedger,
        ("mga-pairwise-permutation", "point-estimates") => K::PairwisePointEstimates,
        ("mga-pairwise-permutation", "inference") => K::PairwiseInference,
        ("mga-pairwise-permutation", "null-target-vectors") => K::TargetLedger,
        ("mga-omnibus-permutation", "metadata") => K::Metadata,
        ("mga-omnibus-permutation", "ledger") => K::PermutationLedger,
        ("mga-omnibus-permutation", "group-point-estimates") => K::OmnibusGroupPointEstimates,
        ("mga-omnibus-permutation", "inference") => K::OmnibusInference,
        ("mga-omnibus-permutation", "null-target-vectors") => K::TargetLedger,
        ("mga-bootstrap-banks", "metadata") => K::Metadata,
        ("mga-bootstrap-banks", "ledger") => K::GroupBootstrapLedger,
        ("mga-bootstrap-banks", "target-vectors") => K::TargetLedger,
        ("mga-bootstrap-banks", "point-estimates") => K::GroupPointEstimates,
        ("mga-micom-pair", "metadata") => K::Metadata,
        ("mga-micom-pair", "ledger") => K::ResampleLedger,
        ("mga-micom-pair", "constructs") => K::MicomConstructs,
        ("mga-micom-pair", "null-statistics") => K::MicomNullStatistics,
        ("mga-ordinary-pls-path-standard-error" | "mga-pairwise-parametric", "metadata") => {
            K::Metadata
        }
        ("mga-parametric-wald", "metadata") => K::Metadata,
        ("mga-parametric-wald", "groups") => K::GroupIndex,
        ("fimix-candidate", "metadata") => K::Metadata,
        ("fimix-candidate", "posteriors") => K::Membership,
        ("fimix-candidate", "hard-assignments") => K::FimixAssignments,
        ("fimix-candidate", "class-coefficients") => K::FimixCoefficients,
        ("fimix-candidate", "start-diagnostics") => K::FimixStartDiagnostics,
        ("fimix-candidate", "start-effective-sizes") => K::FimixStartEffectiveSizes,
        ("fimix-candidate", "reproducing-starts") => K::StartIndex,
        ("fimix-candidate", "start-traces") => K::StartTrace,
        ("fimix-candidate", "multistart-receipts") => K::HeterogeneityMultistartReceipts,
        ("fimix-candidate", "multistart-assignments") => K::HeterogeneityMultistartAssignments,
        ("fimix-candidate", "multistart-coefficient-signatures") => {
            K::HeterogeneityMultistartSignatures
        }
        ("fimix-candidate", "multistart-posteriors") => K::HeterogeneityMultistartPosteriors,
        ("pls-pos-candidate", "metadata") => K::Metadata,
        ("pls-pos-candidate", "memberships") => K::Membership,
        ("pls-pos-candidate", "segments") => K::PosSegments,
        ("pls-pos-candidate", "segment-r-squared") => K::PosRSquared,
        ("pls-pos-candidate", "parameter-signatures") => K::PosParameterSignatures,
        ("pls-pos-candidate", "outcome-fit-audit") => K::PosOutcomeFitAudit,
        ("pls-pos-candidate", "start-diagnostics") => K::PosStartDiagnostics,
        ("pls-pos-candidate", "reproducing-starts") => K::StartIndex,
        ("pls-pos-candidate", "start-traces") => K::StartTrace,
        ("pls-pos-candidate", "candidate-refit-failures") => K::PosCandidateRefitFailures,
        ("pls-pos-candidate", "multistart-receipts") => K::HeterogeneityMultistartReceipts,
        ("pls-pos-candidate", "multistart-assignments") => K::HeterogeneityMultistartAssignments,
        ("pls-pos-candidate", "multistart-parameter-signatures") => {
            K::HeterogeneityMultistartSignatures
        }
        ("heterogeneity-pooled-baseline", "metadata") => K::Metadata,
        ("heterogeneity-pooled-baseline", "coefficients") => K::PooledCoefficients,
        ("heterogeneity-raw-preparation", "metadata") => K::Metadata,
        ("heterogeneity-raw-preparation", "source-row-map") => K::SourceRowMap,
        ("heterogeneity-raw-preparation", "fimix-input") => K::HeterogeneityFimixInput,
        ("heterogeneity-bootstrap", "metadata") => K::Metadata,
        ("heterogeneity-bootstrap", "ledger") => K::HeterogeneityBootstrapLedger,
        ("heterogeneity-bootstrap", "target-vectors") => K::TargetLedger,
        ("heterogeneity-bootstrap", "label-mapping") => K::LabelMapping,
        ("heterogeneity-bootstrap", "label-overlap") => K::LabelOverlap,
        ("conditional-inference" | "interventional-bootstrap", "metadata") => K::Metadata,
        ("conditional-inference" | "interventional-bootstrap", "resample-ledger") => {
            K::ResampleLedger
        }
        (
            "conditional-inference" | "interventional-bootstrap",
            "target-vectors" | "studentized-outer-standard-errors" | "delete-one-target-vectors",
        ) => K::TargetLedger,
        ("conditional-inference" | "interventional-bootstrap", "target-summary") => {
            K::TargetSummary
        }
        ("conditional-inference" | "interventional-bootstrap", "bca-jackknife-summary") => {
            K::BcaJackknifeSummary
        }
        ("conditional-raw-preparation", "metadata") => K::Metadata,
        ("conditional-raw-preparation", "required-source-columns") => K::RequiredSourceColumns,
        ("conditional-raw-preparation", "analysis-rows") => K::AnalysisRows,
        ("conditional-raw-preparation", "excluded-rows") => K::ExcludedRows,
        ("interventional-full-refit-ledger", "metadata") => K::Metadata,
        ("interventional-full-refit-ledger", "records") => K::CaseRecords,
        ("interventional-full-refit-ledger", "draw-rows") => K::CaseDrawRows,
        ("interventional-full-refit-ledger", "usable-indices") => K::RecordIndex,
        ("interventional-full-refit-ledger", "target-vectors") => K::TargetLedger,
        ("heterogeneity-pos-common-metric", "metadata") => K::Metadata,
        ("heterogeneity-pos-common-metric", "construct-gate") => K::CommonMetricConstructGate,
        ("heterogeneity-pos-common-metric", "compositional-pairs") => {
            K::CommonMetricCompositionalPairs
        }
        ("heterogeneity-pos-common-metric", "step3-pairs") => K::CommonMetricStep3Pairs,
        ("heterogeneity-pos-common-metric", "blockers") => K::CommonMetricBlockers,
        ("heterogeneity-pos-common-metric", "common-metric-parameters") => {
            K::CommonMetricParameters
        }
        _ => {
            if kind == "heterogeneity-pos-common-metric" {
                match strip_indexed_table_role_v1(table, "micom") {
                    Some("metadata") => K::Metadata,
                    Some("ledger") => K::ResampleLedger,
                    Some("constructs") => K::MicomConstructs,
                    Some("null-statistics") => K::TargetLedger,
                    _ => {
                        return Err(MultiModArchiveErrorV1::InvalidSidecarEntry(
                            evidence_role.into(),
                        ));
                    }
                }
            } else if kind == "conditional-raw-full-refit" {
                let case_schema = |suffix| match suffix {
                    "metadata" => Some(K::Metadata),
                    "records" => Some(K::CaseRecords),
                    "draw-rows" => Some(K::CaseDrawRows),
                    "usable-indices" => Some(K::RecordIndex),
                    "target-vectors" => Some(K::TargetLedger),
                    _ => None,
                };
                let delete_schema = |suffix| match suffix {
                    "metadata" => Some(K::Metadata),
                    "records" => Some(K::DeleteOneRecords),
                    "retained-rows" => Some(K::DeleteOneRetainedRows),
                    "usable-indices" => Some(K::RecordIndex),
                    "target-vectors" => Some(K::TargetLedger),
                    _ => None,
                };
                let studentized_schema = |suffix| match suffix {
                    "metadata" => Some(K::Metadata),
                    "outer-records" => Some(K::StudentizedOuterRecords),
                    "outer-draw-rows" => Some(K::StudentizedOuterDrawRows),
                    "inner-records" => Some(K::StudentizedInnerRecords),
                    "inner-draw-rows" => Some(K::StudentizedInnerDrawRows),
                    "inner-target-vectors" => Some(K::StudentizedInnerTargetVectors),
                    "usable-outer-indices" => Some(K::RecordIndex),
                    "outer-target-vectors" => Some(K::TargetLedger),
                    _ => None,
                };
                let frequency_schema = |suffix| match suffix {
                    "metadata" => Some(K::Metadata),
                    "records" => Some(K::FrequencyRecords),
                    "counts" => Some(K::FrequencyCounts),
                    "usable-indices" => Some(K::RecordIndex),
                    "target-vectors" => Some(K::TargetLedger),
                    _ => None,
                };
                let resolved = table
                    .strip_prefix("case-")
                    .and_then(case_schema)
                    .or_else(|| table.strip_prefix("bootstrap-").and_then(case_schema))
                    .or_else(|| table.strip_prefix("delete-one-").and_then(delete_schema))
                    .or_else(|| table.strip_prefix("nested-").and_then(studentized_schema))
                    .or_else(|| table.strip_prefix("observed-inner-").and_then(case_schema))
                    .or_else(|| table.strip_prefix("frequency-").and_then(frequency_schema))
                    .or_else(|| {
                        strip_indexed_table_role_v1(table, "group").and_then(|suffix| {
                            if suffix == "identity" {
                                Some(K::Metadata)
                            } else {
                                case_schema(suffix)
                            }
                        })
                    });
                resolved.ok_or_else(|| {
                    MultiModArchiveErrorV1::InvalidSidecarEntry(evidence_role.into())
                })?
            } else {
                return Err(MultiModArchiveErrorV1::InvalidSidecarEntry(
                    evidence_role.into(),
                ));
            }
        }
    };
    Ok(resolved)
}

fn trusted_multimod_arrow_schema_for_role_v1(
    evidence_role: &str,
) -> Result<Schema, MultiModArchiveErrorV1> {
    Ok(trusted_multimod_arrow_schema_v1(
        trusted_multimod_arrow_schema_kind_v1(evidence_role)?,
    ))
}

pub fn encode_multimod_arrow_sidecar_v1(
    result_id: &str,
    leaf_name: &str,
    identity_sha256: &str,
    evidence_role: &str,
    batch: &RecordBatch,
) -> Result<MultiModSidecarPayloadV1, MultiModArchiveErrorV1> {
    validate_stable_result_id(result_id)?;
    if leaf_name.is_empty()
        || leaf_name.contains('/')
        || leaf_name.contains('\\')
        || !leaf_name.ends_with(".arrow")
    {
        return Err(MultiModArchiveErrorV1::InvalidSidecarEntry(
            leaf_name.into(),
        ));
    }
    validate_sha256("identity_sha256", identity_sha256)?;
    let expected_base_schema = trusted_multimod_arrow_schema_for_role_v1(evidence_role)?;
    let leaf_stem = leaf_name
        .strip_suffix(".arrow")
        .expect("validated Arrow leaf");
    let (evidence_kind, table_role) = evidence_role
        .split_once(':')
        .ok_or_else(|| MultiModArchiveErrorV1::InvalidSidecarEntry(leaf_name.into()))?;
    if !(leaf_stem.starts_with(&format!("{evidence_kind}-"))
        || leaf_stem.contains(&format!("-{evidence_kind}-")))
        || !leaf_stem.ends_with(&format!("-{table_role}"))
    {
        return Err(MultiModArchiveErrorV1::InvalidSidecarEntry(
            leaf_name.into(),
        ));
    }
    let observed_base_schema_sha256 = multimod_arrow_schema_sha256_v1(batch.schema().as_ref())?;
    let base_schema_sha256 = multimod_arrow_schema_sha256_v1(&expected_base_schema)?;
    if observed_base_schema_sha256 != base_schema_sha256 {
        return Err(MultiModArchiveErrorV1::SidecarSchemaMismatch(
            leaf_name.into(),
        ));
    }
    let arrow_schema_contract_id =
        format!("qpls.multimod.arrow.{evidence_role}.v1.{base_schema_sha256}");
    let mut metadata = batch.schema().metadata.clone();
    for key in [
        MULTIMOD_ARROW_EVIDENCE_ROLE_METADATA_V1,
        MULTIMOD_ARROW_SCHEMA_CONTRACT_ID_METADATA_V1,
        MULTIMOD_ARROW_SCHEMA_CONTRACT_VERSION_METADATA_V1,
        MULTIMOD_ARROW_BASE_SCHEMA_SHA256_METADATA_V1,
    ] {
        if metadata.contains_key(key) {
            return Err(MultiModArchiveErrorV1::SidecarSchemaMismatch(
                leaf_name.into(),
            ));
        }
    }
    metadata.insert(
        MULTIMOD_ARROW_EVIDENCE_ROLE_METADATA_V1.into(),
        evidence_role.into(),
    );
    metadata.insert(
        MULTIMOD_ARROW_SCHEMA_CONTRACT_ID_METADATA_V1.into(),
        arrow_schema_contract_id.clone(),
    );
    metadata.insert(
        MULTIMOD_ARROW_SCHEMA_CONTRACT_VERSION_METADATA_V1.into(),
        MULTIMOD_ARROW_EVIDENCE_CONTRACT_VERSION_V1.to_string(),
    );
    metadata.insert(
        MULTIMOD_ARROW_BASE_SCHEMA_SHA256_METADATA_V1.into(),
        base_schema_sha256,
    );
    let contractual_schema = Arc::new(Schema::new_with_metadata(
        batch.schema().fields().clone(),
        metadata,
    ));
    let contractual_batch = RecordBatch::try_new(contractual_schema, batch.columns().to_vec())?;
    let mut bytes = Vec::new();
    {
        let mut writer = StreamWriter::try_new(&mut bytes, contractual_batch.schema().as_ref())?;
        writer.write(&contractual_batch)?;
        writer.finish()?;
    }
    if bytes.len() as u64 > MULTIMOD_SIDECAR_MAX_BYTES_V1 {
        return Err(MultiModArchiveErrorV1::SidecarTooLarge);
    }
    let entry_name = format!("results/{result_id}/{leaf_name}");
    let descriptor = MultimodResultSidecarDescriptorV1 {
        schema_version: MULTIMOD_RESULT_SIDECAR_DESCRIPTOR_V1_SCHEMA_VERSION,
        entry_name,
        evidence_role: evidence_role.into(),
        arrow_schema_contract_id,
        arrow_schema_contract_version: MULTIMOD_ARROW_EVIDENCE_CONTRACT_VERSION_V1,
        media_type: MULTIMOD_ARROW_SIDECAR_MEDIA_TYPE_V1.into(),
        compression: MULTIMOD_ARROW_SIDECAR_COMPRESSION_V1.into(),
        arrow_schema_sha256: multimod_arrow_schema_sha256_v1(contractual_batch.schema().as_ref())?,
        row_count: contractual_batch.num_rows() as u64,
        column_count: contractual_batch.num_columns() as u32,
        uncompressed_bytes: bytes.len() as u64,
        sha256: sha256_bytes(&bytes),
        identity_sha256: identity_sha256.into(),
        required_for_scientific_reopen: true,
    };
    Ok(MultiModSidecarPayloadV1 { descriptor, bytes })
}

pub fn validate_multimod_sidecar_payload_v1(
    result_id: &str,
    payload: &MultiModSidecarPayloadV1,
) -> Result<(), MultiModArchiveErrorV1> {
    validate_sidecar_descriptor_v1(result_id, &payload.descriptor)?;
    if payload.bytes.len() as u64 != payload.descriptor.uncompressed_bytes {
        return Err(MultiModArchiveErrorV1::SidecarSizeMismatch(
            payload.descriptor.entry_name.clone(),
        ));
    }
    if sha256_bytes(&payload.bytes) != payload.descriptor.sha256 {
        return Err(MultiModArchiveErrorV1::SidecarDigestMismatch(
            payload.descriptor.entry_name.clone(),
        ));
    }
    validate_multimod_sidecar_stream_v1(
        result_id,
        &payload.descriptor,
        Cursor::new(payload.bytes.as_slice()),
    )
}

/// Validates one already checksum/size-bound Arrow stream without retaining
/// its potentially 512 MiB payload in memory. Archive readers must bind the
/// descriptor digest and byte count to their verified ZIP manifest before
/// calling this streaming seam.
pub fn validate_multimod_sidecar_stream_v1<R: Read>(
    result_id: &str,
    descriptor: &MultimodResultSidecarDescriptorV1,
    reader: R,
) -> Result<(), MultiModArchiveErrorV1> {
    validate_sidecar_descriptor_v1(result_id, descriptor)?;
    let reader = StreamReader::try_new(reader, None)?;
    let schema = reader.schema();
    let mut base_metadata = schema.metadata.clone();
    let base_schema_sha256 = base_metadata.remove(MULTIMOD_ARROW_BASE_SCHEMA_SHA256_METADATA_V1);
    base_metadata.remove(MULTIMOD_ARROW_EVIDENCE_ROLE_METADATA_V1);
    base_metadata.remove(MULTIMOD_ARROW_SCHEMA_CONTRACT_ID_METADATA_V1);
    base_metadata.remove(MULTIMOD_ARROW_SCHEMA_CONTRACT_VERSION_METADATA_V1);
    let base_schema = Schema::new_with_metadata(schema.fields().clone(), base_metadata);
    let observed_base_schema_sha256 = multimod_arrow_schema_sha256_v1(&base_schema)?;
    let trusted_base_schema = trusted_multimod_arrow_schema_for_role_v1(&descriptor.evidence_role)?;
    let trusted_base_schema_sha256 = multimod_arrow_schema_sha256_v1(&trusted_base_schema)?;
    let expected_contract_id = format!(
        "qpls.multimod.arrow.{}.v1.{trusted_base_schema_sha256}",
        descriptor.evidence_role
    );
    if multimod_arrow_schema_sha256_v1(schema.as_ref())? != descriptor.arrow_schema_sha256
        || schema
            .metadata
            .get(MULTIMOD_ARROW_EVIDENCE_ROLE_METADATA_V1)
            != Some(&descriptor.evidence_role)
        || schema
            .metadata
            .get(MULTIMOD_ARROW_SCHEMA_CONTRACT_ID_METADATA_V1)
            != Some(&descriptor.arrow_schema_contract_id)
        || schema
            .metadata
            .get(MULTIMOD_ARROW_SCHEMA_CONTRACT_VERSION_METADATA_V1)
            .map(String::as_str)
            != Some("1")
        || observed_base_schema_sha256 != trusted_base_schema_sha256
        || base_schema_sha256.as_deref() != Some(trusted_base_schema_sha256.as_str())
        || descriptor.arrow_schema_contract_id != expected_contract_id
    {
        return Err(MultiModArchiveErrorV1::SidecarSchemaMismatch(
            descriptor.entry_name.clone(),
        ));
    }
    if u32::try_from(schema.fields().len()).ok() != Some(descriptor.column_count) {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            descriptor.entry_name.clone(),
        ));
    }
    let mut rows = 0u64;
    for batch in reader {
        let batch = batch?;
        rows = rows.checked_add(batch.num_rows() as u64).ok_or_else(|| {
            MultiModArchiveErrorV1::SidecarShapeMismatch(descriptor.entry_name.clone())
        })?;
        let observed = batch.num_columns() as u32;
        if observed != descriptor.column_count {
            return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
                descriptor.entry_name.clone(),
            ));
        }
    }
    if rows != descriptor.row_count {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            descriptor.entry_name.clone(),
        ));
    }
    Ok(())
}

/// Reads one already-authorized Arrow entry from an independently pinned
/// archive handle. Callers must first strict-reopen and bind the descriptor to
/// that same unchanged archive identity; unlike the general loader, this seam
/// allocates only the selected optional-export payload.
pub fn read_multimod_sidecar_payload_from_file_v1(
    file: File,
    result_id: &str,
    descriptor: &MultimodResultSidecarDescriptorV1,
) -> Result<MultiModSidecarPayloadV1, MultiModArchiveErrorV1> {
    validate_sidecar_descriptor_v1(result_id, descriptor)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entry = archive.by_name(&descriptor.entry_name)?;
    if entry.size() != descriptor.uncompressed_bytes {
        return Err(MultiModArchiveErrorV1::SidecarSizeMismatch(
            descriptor.entry_name.clone(),
        ));
    }
    let capacity = usize::try_from(descriptor.uncompressed_bytes)
        .map_err(|_| MultiModArchiveErrorV1::SidecarTooLarge)?;
    let mut bytes = Vec::with_capacity(capacity);
    entry
        .by_ref()
        .take(descriptor.uncompressed_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    let payload = MultiModSidecarPayloadV1 {
        descriptor: descriptor.clone(),
        bytes,
    };
    validate_multimod_sidecar_payload_v1(result_id, &payload)?;
    Ok(payload)
}

/// Enforces the scientific evidence cap across the complete result, rather
/// than allowing a run to evade it by splitting evidence among several Arrow
/// streams.
pub fn validate_multimod_sidecar_total_bytes_v1(
    descriptors: &[MultimodResultSidecarDescriptorV1],
) -> Result<u64, MultiModArchiveErrorV1> {
    let total = descriptors.iter().try_fold(0u64, |current, descriptor| {
        current.checked_add(descriptor.uncompressed_bytes)
    });
    match total {
        Some(total) if total <= MULTIMOD_SIDECAR_MAX_BYTES_V1 => Ok(total),
        _ => Err(MultiModArchiveErrorV1::SidecarSetTooLarge),
    }
}

fn dictionary_utf8_array_v1(values: Vec<String>) -> Result<ArrayRef, MultiModArchiveErrorV1> {
    let mut builder = StringDictionaryBuilder::<UInt32Type>::new();
    for value in values {
        builder.append(value)?;
    }
    Ok(Arc::new(builder.finish()))
}

/// Convenience schema used by bootstrap ledgers and compact target vectors.
/// Repeated target identities and failure codes are dictionary encoded. This
/// is part of the trusted schema, rather than a best-effort IPC option, so the
/// runner can preflight retained evidence against the representation that the
/// archive will actually write.
pub fn multimod_target_ledger_batch_v1(
    replicate_index: Vec<u32>,
    target_id: Vec<String>,
    value: Vec<f64>,
    valid: Vec<bool>,
    failure_code: Vec<String>,
) -> Result<RecordBatch, MultiModArchiveErrorV1> {
    let length = replicate_index.len();
    if length == 0
        || [
            target_id.len(),
            value.len(),
            valid.len(),
            failure_code.len(),
        ]
        .into_iter()
        .any(|candidate| candidate != length)
        || target_id.iter().any(|target| target.trim().is_empty())
        || value
            .iter()
            .zip(&valid)
            .any(|(estimate, is_valid)| *is_valid && !estimate.is_finite())
        || valid
            .iter()
            .zip(&failure_code)
            .any(|(is_valid, code)| *is_valid != code.is_empty())
    {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            "target ledger columns or validity identities".into(),
        ));
    }
    let schema = Arc::new(Schema::new(vec![
        Field::new("replicate_index", DataType::UInt32, false),
        Field::new("target_id", dictionary_utf8_u32_type_v1(), false),
        Field::new("value", DataType::Float64, true),
        Field::new("valid", DataType::Boolean, false),
        Field::new("failure_code", dictionary_utf8_u32_type_v1(), false),
    ]));
    let arrays: Vec<ArrayRef> = vec![
        Arc::new(UInt32Array::from(replicate_index)),
        dictionary_utf8_array_v1(target_id)?,
        Arc::new(Float64Array::from(
            value
                .into_iter()
                .zip(valid.iter())
                .map(|(value, valid)| valid.then_some(value))
                .collect::<Vec<_>>(),
        )),
        Arc::new(BooleanArray::from(valid)),
        dictionary_utf8_array_v1(failure_code)?,
    ];
    Ok(RecordBatch::try_new(schema, arrays)?)
}

pub const MULTIMOD_MICOM_COMPOSITIONAL_CORRELATION_STATISTIC_V1: u8 = 0;
pub const MULTIMOD_MICOM_MEAN_DIFFERENCE_STATISTIC_V1: u8 = 1;
pub const MULTIMOD_MICOM_LOG_VARIANCE_RATIO_STATISTIC_V1: u8 = 2;

/// Compact MICOM null statistics. `construct_index` is the zero-based row in
/// the sibling trusted `constructs` table, and `statistic_kind` is one of the
/// constants above. This avoids repeating construct identifiers for every
/// permutation while preserving deterministic, typed readback.
pub fn multimod_micom_null_statistics_batch_v1(
    replicate_index: Vec<u32>,
    construct_index: Vec<u32>,
    statistic_kind: Vec<u8>,
    value: Vec<f64>,
) -> Result<RecordBatch, MultiModArchiveErrorV1> {
    let length = replicate_index.len();
    if length == 0
        || [construct_index.len(), statistic_kind.len(), value.len()]
            .into_iter()
            .any(|candidate| candidate != length)
        || statistic_kind.iter().any(|kind| {
            !matches!(
                *kind,
                MULTIMOD_MICOM_COMPOSITIONAL_CORRELATION_STATISTIC_V1
                    | MULTIMOD_MICOM_MEAN_DIFFERENCE_STATISTIC_V1
                    | MULTIMOD_MICOM_LOG_VARIANCE_RATIO_STATISTIC_V1
            )
        })
        || value.iter().any(|statistic| !statistic.is_finite())
        || replicate_index
            .iter()
            .zip(&construct_index)
            .zip(&statistic_kind)
            .map(|((replicate, construct), kind)| (*replicate, *construct, *kind))
            .collect::<BTreeSet<_>>()
            .len()
            != length
    {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            "MICOM null-statistic columns or ordinal identities".into(),
        ));
    }
    let schema = Arc::new(trusted_multimod_arrow_schema_v1(
        TrustedMultiModArrowSchemaV1::MicomNullStatistics,
    ));
    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(UInt32Array::from(replicate_index)),
            Arc::new(UInt32Array::from(construct_index)),
            Arc::new(UInt8Array::from(statistic_kind)),
            Arc::new(Float64Array::from(value)),
        ],
    )?)
}

/// One row per requested resample. Seeds and shard identities are retained so
/// cancellation/resume can prove that it reused the frozen no-retry ledger.
pub fn multimod_resample_ledger_batch_v1(
    replicate_index: Vec<u32>,
    seed: Vec<u64>,
    usable: Vec<bool>,
    failure_code: Vec<String>,
    shard_id: Vec<String>,
) -> Result<RecordBatch, MultiModArchiveErrorV1> {
    let length = replicate_index.len();
    if length == 0
        || [seed.len(), usable.len(), failure_code.len(), shard_id.len()]
            .into_iter()
            .any(|candidate| candidate != length)
        || replicate_index
            .iter()
            .enumerate()
            .any(|(expected, observed)| *observed as usize != expected)
        || usable
            .iter()
            .zip(&failure_code)
            .any(|(usable, code)| *usable != code.is_empty())
        || shard_id.iter().any(|value| value.trim().is_empty())
    {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            "resample ledger columns or status identities".into(),
        ));
    }
    let schema = Arc::new(Schema::new(vec![
        Field::new("replicate_index", DataType::UInt32, false),
        Field::new("seed", DataType::UInt64, false),
        Field::new("usable", DataType::Boolean, false),
        Field::new("failure_code", DataType::Utf8, false),
        Field::new("shard_id", DataType::Utf8, false),
    ]));
    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(UInt32Array::from(replicate_index)),
            Arc::new(UInt64Array::from(seed)),
            Arc::new(BooleanArray::from(usable)),
            Arc::new(StringArray::from(failure_code)),
            Arc::new(StringArray::from(shard_id)),
        ],
    )?)
}

/// Complete multi-start trace for FIMIX likelihood or PLS-POS objective
/// histories. The scientific engine validates objective monotonicity; this
/// archive boundary preserves every finite value and terminal status.
pub fn multimod_start_trace_batch_v1(
    algorithm: Vec<String>,
    k: Vec<u8>,
    start_index: Vec<u32>,
    iteration: Vec<u32>,
    objective: Vec<f64>,
    converged: Vec<bool>,
    failure_code: Vec<String>,
) -> Result<RecordBatch, MultiModArchiveErrorV1> {
    let length = algorithm.len();
    if length == 0
        || [
            k.len(),
            start_index.len(),
            iteration.len(),
            objective.len(),
            converged.len(),
            failure_code.len(),
        ]
        .into_iter()
        .any(|candidate| candidate != length)
        || algorithm.iter().any(|value| value.trim().is_empty())
        || k.iter().any(|value| !(2..=5).contains(value))
        || objective.iter().any(|value| !value.is_finite())
        || converged
            .iter()
            .zip(&failure_code)
            .any(|(converged, code)| *converged && !code.is_empty())
    {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            "multi-start trace columns or values".into(),
        ));
    }
    let schema = Arc::new(Schema::new(vec![
        Field::new("algorithm", DataType::Utf8, false),
        Field::new("k", DataType::UInt8, false),
        Field::new("start_index", DataType::UInt32, false),
        Field::new("iteration", DataType::UInt32, false),
        Field::new("objective", DataType::Float64, false),
        Field::new("converged", DataType::Boolean, false),
        Field::new("failure_code", DataType::Utf8, false),
    ]));
    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(algorithm)),
            Arc::new(UInt8Array::from(k)),
            Arc::new(UInt32Array::from(start_index)),
            Arc::new(UInt32Array::from(iteration)),
            Arc::new(Float64Array::from(objective)),
            Arc::new(BooleanArray::from(converged)),
            Arc::new(StringArray::from(failure_code)),
        ],
    )?)
}

/// Complete summary needed to audit each full delete-one BCa acceleration.
/// The delete-one target vector itself is stored with
/// `multimod_target_ledger_batch_v1`.
pub fn multimod_bca_jackknife_summary_batch_v1(
    target_id: Vec<String>,
    delete_one_count: Vec<u32>,
    jackknife_mean: Vec<f64>,
    centered_cube_sum: Vec<f64>,
    centered_square_sum: Vec<f64>,
    acceleration: Vec<f64>,
    complete: Vec<bool>,
) -> Result<RecordBatch, MultiModArchiveErrorV1> {
    let length = target_id.len();
    if length == 0
        || [
            delete_one_count.len(),
            jackknife_mean.len(),
            centered_cube_sum.len(),
            centered_square_sum.len(),
            acceleration.len(),
            complete.len(),
        ]
        .into_iter()
        .any(|candidate| candidate != length)
        || target_id.iter().any(|value| value.trim().is_empty())
        || target_id.iter().collect::<BTreeSet<_>>().len() != length
        || delete_one_count.iter().any(|value| *value < 2)
        || jackknife_mean
            .iter()
            .chain(&centered_cube_sum)
            .chain(&centered_square_sum)
            .chain(&acceleration)
            .any(|value| !value.is_finite())
        || centered_square_sum.iter().any(|value| *value < 0.0)
    {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            "BCa jackknife summary columns or values".into(),
        ));
    }
    let schema = Arc::new(Schema::new(vec![
        Field::new("target_id", DataType::Utf8, false),
        Field::new("delete_one_count", DataType::UInt32, false),
        Field::new("jackknife_mean", DataType::Float64, false),
        Field::new("centered_cube_sum", DataType::Float64, false),
        Field::new("centered_square_sum", DataType::Float64, false),
        Field::new("acceleration", DataType::Float64, false),
        Field::new("complete", DataType::Boolean, false),
    ]));
    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(target_id)),
            Arc::new(UInt32Array::from(delete_one_count)),
            Arc::new(Float64Array::from(jackknife_mean)),
            Arc::new(Float64Array::from(centered_cube_sum)),
            Arc::new(Float64Array::from(centered_square_sum)),
            Arc::new(Float64Array::from(acceleration)),
            Arc::new(BooleanArray::from(complete)),
        ],
    )?)
}

/// Convenience schema for N×K posterior probabilities or segment membership.
pub fn multimod_membership_batch_v1(
    row_index: Vec<u32>,
    class_id: Vec<u8>,
    posterior_probability: Vec<f64>,
) -> Result<RecordBatch, MultiModArchiveErrorV1> {
    let length = row_index.len();
    if class_id.len() != length || posterior_probability.len() != length {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            "membership columns".into(),
        ));
    }
    let schema = Arc::new(Schema::new(vec![
        Field::new("row_index", DataType::UInt32, false),
        Field::new("class_id", DataType::UInt8, false),
        Field::new("posterior_probability", DataType::Float64, false),
    ]));
    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(UInt32Array::from(row_index)),
            Arc::new(UInt8Array::from(class_id)),
            Arc::new(Float64Array::from(posterior_probability)),
        ],
    )?)
}

/// Posterior/membership table with an immutable source-row token in addition
/// to the display-order row index. Every row must contain the same complete
/// class inventory and posterior probabilities must sum to one.
pub fn multimod_membership_with_row_tokens_batch_v1(
    row_index: Vec<u32>,
    stable_row_token: Vec<String>,
    class_id: Vec<u8>,
    posterior_probability: Vec<f64>,
) -> Result<RecordBatch, MultiModArchiveErrorV1> {
    let length = row_index.len();
    if length == 0
        || stable_row_token.len() != length
        || class_id.len() != length
        || posterior_probability.len() != length
        || stable_row_token.iter().any(|value| value.trim().is_empty())
        || posterior_probability
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            "membership row-token columns or probabilities".into(),
        ));
    }
    let class_count = class_id
        .iter()
        .copied()
        .max()
        .map_or(0usize, |value| value as usize + 1);
    if !(2..=5).contains(&class_count) {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            "membership class inventory".into(),
        ));
    }
    let mut rows = BTreeMap::<u32, (String, BTreeSet<u8>, f64)>::new();
    let mut token_rows = BTreeMap::<&str, u32>::new();
    for (((row, token), class), probability) in row_index
        .iter()
        .zip(&stable_row_token)
        .zip(&class_id)
        .zip(&posterior_probability)
    {
        if token_rows
            .insert(token.as_str(), *row)
            .is_some_and(|previous| previous != *row)
        {
            return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
                "membership row token identity".into(),
            ));
        }
        let entry = rows
            .entry(*row)
            .or_insert_with(|| (token.clone(), BTreeSet::new(), 0.0));
        if entry.0 != *token || !entry.1.insert(*class) {
            return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
                "membership row/class identity".into(),
            ));
        }
        entry.2 += *probability;
    }
    if rows.values().any(|(_, classes, probability_sum)| {
        classes.len() != class_count
            || classes.iter().copied().ne(0..class_count as u8)
            || (probability_sum - 1.0).abs() > 1.0e-10
    }) {
        return Err(MultiModArchiveErrorV1::SidecarShapeMismatch(
            "membership posterior matrix completeness".into(),
        ));
    }
    let schema = Arc::new(Schema::new(vec![
        Field::new("row_index", DataType::UInt32, false),
        Field::new("stable_row_token", DataType::Utf8, false),
        Field::new("class_id", DataType::UInt8, false),
        Field::new("posterior_probability", DataType::Float64, false),
    ]));
    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(UInt32Array::from(row_index)),
            Arc::new(StringArray::from(stable_row_token)),
            Arc::new(UInt8Array::from(class_id)),
            Arc::new(Float64Array::from(posterior_probability)),
        ],
    )?)
}

/// Atomically attaches a completed MultiMod result, its sidecars, and its
/// canonical projection to an archive that already contains the exact recipe.
pub fn append_multimod_result_and_canonical_to_archive_v6<Cancelled>(
    archive_path: &Path,
    expected_source_sha256: &str,
    attachment: MultiModResultAttachmentV1,
    payloads: Vec<MultiModSidecarPayloadV1>,
    canonical_document: CanonicalResultDocumentV2,
    cancelled: Cancelled,
) -> Result<MultiModArchiveAppendReceiptV1, MultiModArchiveErrorV1>
where
    Cancelled: Fn() -> bool,
{
    append_multimod_recipe_and_result_inner_v1(
        archive_path,
        expected_source_sha256,
        None,
        attachment,
        payloads,
        canonical_document,
        cancelled,
    )
}

/// Atomically stages a Recipe V4, its completed MultiMod attachment, every
/// required Arrow sidecar, and the canonical result projection used by reopen
/// and semantic export. No member of this set can become resident alone.
pub fn append_multimod_recipe_result_and_canonical_to_archive_v6<Cancelled>(
    archive_path: &Path,
    expected_source_sha256: &str,
    staged_recipe: AnalysisRecipeV4,
    attachment: MultiModResultAttachmentV1,
    payloads: Vec<MultiModSidecarPayloadV1>,
    canonical_document: CanonicalResultDocumentV2,
    cancelled: Cancelled,
) -> Result<MultiModArchiveAppendReceiptV1, MultiModArchiveErrorV1>
where
    Cancelled: Fn() -> bool,
{
    append_multimod_recipe_and_result_inner_v1(
        archive_path,
        expected_source_sha256,
        Some(staged_recipe),
        attachment,
        payloads,
        canonical_document,
        cancelled,
    )
}

fn append_multimod_recipe_and_result_inner_v1<Cancelled>(
    archive_path: &Path,
    expected_source_sha256: &str,
    staged_recipe: Option<AnalysisRecipeV4>,
    attachment: MultiModResultAttachmentV1,
    payloads: Vec<MultiModSidecarPayloadV1>,
    canonical_document: CanonicalResultDocumentV2,
    cancelled: Cancelled,
) -> Result<MultiModArchiveAppendReceiptV1, MultiModArchiveErrorV1>
where
    Cancelled: Fn() -> bool,
{
    validate_sha256("expected_source_sha256", expected_source_sha256)?;
    attachment.ensure_valid()?;
    let descriptor_names = attachment
        .sidecars
        .iter()
        .map(|descriptor| descriptor.entry_name.as_str())
        .collect::<BTreeSet<_>>();
    let payload_names = payloads
        .iter()
        .map(|payload| payload.descriptor.entry_name.as_str())
        .collect::<BTreeSet<_>>();
    if descriptor_names != payload_names || payloads.len() != payload_names.len() {
        return Err(MultiModArchiveErrorV1::SidecarSetMismatch);
    }
    validate_multimod_sidecar_total_bytes_v1(&attachment.sidecars)?;
    for payload in &payloads {
        if attachment
            .sidecars
            .iter()
            .find(|descriptor| descriptor.entry_name == payload.descriptor.entry_name)
            != Some(&payload.descriptor)
        {
            return Err(MultiModArchiveErrorV1::SidecarSetMismatch);
        }
        validate_multimod_sidecar_payload_v1(&attachment.result_id, payload)?;
    }
    let mut source_file = open_pinned_multimod_source_v1(archive_path)?;
    if sha256_file_handle_v1(&mut source_file)? != expected_source_sha256 {
        return Err(MultiModArchiveErrorV1::SourceChanged);
    }
    if cancelled() {
        return Err(MultiModArchiveErrorV1::Cancelled);
    }
    let loaded = crate::load_project_archive_v6_from_file(source_file.try_clone()?)?;
    let source_manifest = loaded.manifest;
    let mut source_document = loaded.document;
    if let Some(recipe) = staged_recipe {
        recipe
            .ensure_valid()
            .map_err(|error| MultiModArchiveErrorV1::InvalidStagedRecipe(error.to_string()))?;
        if recipe.id != attachment.recipe_id {
            return Err(MultiModArchiveErrorV1::StagedRecipeIdentityMismatch);
        }
        if source_document
            .recipes
            .iter()
            .any(|resident| resident.id == recipe.id)
            || source_document
                .historical_recipes
                .iter()
                .any(|resident| resident.recipe_id() == recipe.id)
        {
            return Err(MultiModArchiveErrorV1::DuplicateRecipe(recipe.id));
        }
        source_document.recipes.push(recipe);
        source_document.recipes.sort_by_key(|resident| resident.id);
        source_document.ensure_valid()?;
    }
    let mut updated =
        attach_multimod_result_for_atomic_pair_v1(&source_document, attachment.clone())?;
    if canonical_document.provenance.run_id != attachment.result_id
        || canonical_document.provenance.recipe_id != attachment.recipe_id.to_string()
        || canonical_document.provenance.project_id != updated.project_id.to_string()
    {
        return Err(MultiModArchiveErrorV1::CanonicalResultBinding(
            "run, recipe, or project identity differs from the scientific attachment".into(),
        ));
    }
    let canonical_attachment =
        CanonicalResultDocumentAttachmentV2::from_document(canonical_document.clone())
            .map_err(ProjectArchiveV6Error::from)?;
    canonical_attachment
        .ensure_valid(&updated.project_id.to_string())
        .map_err(ProjectArchiveV6Error::from)?;
    if updated.canonical_result_documents.iter().any(|candidate| {
        candidate.document_id() == canonical_attachment.document_id()
            || candidate.run_id() == canonical_attachment.run_id()
    }) {
        return Err(MultiModArchiveErrorV1::CanonicalResultBinding(
            "canonical document or run identity is already resident".into(),
        ));
    }
    updated
        .canonical_result_documents
        .push(canonical_attachment);
    updated
        .canonical_result_documents
        .sort_by(|left, right| left.document_id().cmp(right.document_id()));
    updated.ensure_valid()?;
    let project_bytes = serialize_project_document_v6(&updated)?;
    let temporary = sibling_temporary_path(archive_path, "multimod.tmp")?;
    let rollback = sibling_temporary_path(archive_path, "multimod.rollback")?;
    let temporary_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> Result<(), MultiModArchiveErrorV1> {
        source_file.seek(SeekFrom::Start(0))?;
        let mut source_archive = ZipArchive::new(source_file.try_clone()?)?;
        let mut output = ZipWriter::new(temporary_file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut checksums = BTreeMap::new();
        output.start_file("project.json", options)?;
        output.write_all(&project_bytes)?;
        checksums.insert("project.json".to_owned(), sha256_bytes(&project_bytes));

        for (entry_name, verified_sha256) in &source_manifest.checksums {
            if entry_name == "project.json" {
                continue;
            }
            if payload_names.contains(entry_name.as_str()) {
                return Err(MultiModArchiveErrorV1::DuplicateSidecarEntry(
                    entry_name.clone(),
                ));
            }
            let mut entry = source_archive.by_name(entry_name)?;
            output.start_file(entry_name, options)?;
            std::io::copy(&mut entry, &mut output)?;
            checksums.insert(entry_name.clone(), verified_sha256.clone());
        }
        for payload in &payloads {
            output.start_file(&payload.descriptor.entry_name, options)?;
            output.write_all(&payload.bytes)?;
            checksums.insert(
                payload.descriptor.entry_name.clone(),
                payload.descriptor.sha256.clone(),
            );
        }
        let manifest = ProjectManifest {
            schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
            project_id: updated.project_id,
            name: updated.name.clone(),
            created_at: updated.created_at,
            modified_at: updated.modified_at,
            engine_version: source_manifest.engine_version.clone(),
            checksum_algorithm: "sha256".into(),
            checksums,
        };
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
        output.start_file("manifest.json", options)?;
        output.write_all(&manifest_bytes)?;
        let finished = output.finish()?;
        finished.sync_all()?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if cancelled() {
        let _ = fs::remove_file(&temporary);
        return Err(MultiModArchiveErrorV1::Cancelled);
    }
    if sha256_file_handle_v1(&mut source_file)? != expected_source_sha256 {
        let _ = fs::remove_file(&temporary);
        return Err(MultiModArchiveErrorV1::SourceChanged);
    }
    let updated_archive_sha256 = sha256_file(&temporary)?;
    drop(source_file);
    if let Err(error) = atomic_replace_with_rollback_v6(&temporary, archive_path, &rollback) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    let validation = load_project_archive_v6(archive_path)
        .map_err(MultiModArchiveErrorV1::from)
        .and_then(|loaded| {
            let result_present = loaded
                .document
                .multimod_results
                .iter()
                .any(|result| result == &attachment);
            let canonical_present =
                loaded
                    .document
                    .canonical_result_documents
                    .iter()
                    .any(|candidate| {
                        candidate.run_id() == attachment.result_id
                            && candidate.canonical_document() == &canonical_document
                    });
            if !result_present
                || !payload_names
                    .iter()
                    .all(|entry| loaded.multimod_sidecars.contains(*entry))
            {
                Err(MultiModArchiveErrorV1::SidecarSetMismatch)
            } else if !canonical_present {
                Err(MultiModArchiveErrorV1::CanonicalResultBinding(
                    "canonical projection was absent or changed after strict reopen".into(),
                ))
            } else {
                Ok(())
            }
        });
    if let Err(error) = validation {
        restore_rollback_v6(&rollback, archive_path).map_err(|rollback_error| {
            MultiModArchiveErrorV1::RollbackFailed(format!(
                "{error}; rollback error: {rollback_error}"
            ))
        })?;
        return Err(error);
    }
    let rollback_removed = fs::remove_file(&rollback).is_ok();
    Ok(MultiModArchiveAppendReceiptV1 {
        schema_version: 1,
        project_id: updated.project_id,
        result_id: attachment.result_id,
        source_archive_sha256: expected_source_sha256.into(),
        updated_archive_sha256,
        sidecar_count: payloads.len(),
        source_verified_at_commit: true,
        post_write_validated: true,
        rollback_removed,
    })
}

pub(crate) fn validate_sidecar_descriptor_v1(
    result_id: &str,
    descriptor: &MultimodResultSidecarDescriptorV1,
) -> Result<(), MultiModArchiveErrorV1> {
    if descriptor.schema_version != MULTIMOD_RESULT_SIDECAR_DESCRIPTOR_V1_SCHEMA_VERSION
        || descriptor.media_type != MULTIMOD_ARROW_SIDECAR_MEDIA_TYPE_V1
        || descriptor.compression != MULTIMOD_ARROW_SIDECAR_COMPRESSION_V1
        || descriptor.arrow_schema_contract_version != MULTIMOD_ARROW_EVIDENCE_CONTRACT_VERSION_V1
        || descriptor.uncompressed_bytes == 0
        || descriptor.uncompressed_bytes > MULTIMOD_SIDECAR_MAX_BYTES_V1
        || descriptor.row_count == 0
        || descriptor.column_count == 0
        || !descriptor.required_for_scientific_reopen
    {
        return Err(MultiModArchiveErrorV1::InvalidSidecarEntry(
            descriptor.entry_name.clone(),
        ));
    }
    let trusted_schema = trusted_multimod_arrow_schema_for_role_v1(&descriptor.evidence_role)?;
    let trusted_schema_sha256 = multimod_arrow_schema_sha256_v1(&trusted_schema)?;
    let contract_prefix = format!("qpls.multimod.arrow.{}.v1.", descriptor.evidence_role);
    if !descriptor
        .arrow_schema_contract_id
        .starts_with(&contract_prefix)
        || !descriptor
            .arrow_schema_contract_id
            .strip_prefix(&contract_prefix)
            .is_some_and(|value| value == trusted_schema_sha256.as_str())
    {
        return Err(MultiModArchiveErrorV1::InvalidSidecarEntry(
            descriptor.entry_name.clone(),
        ));
    }
    validate_sha256(
        "sidecar.arrow_schema_sha256",
        &descriptor.arrow_schema_sha256,
    )?;
    let prefix = format!("results/{result_id}/");
    if !descriptor.entry_name.starts_with(&prefix)
        || !descriptor.entry_name.ends_with(".arrow")
        || descriptor.entry_name.contains("..")
        || descriptor.entry_name.contains('\\')
        || descriptor.entry_name.starts_with('/')
    {
        return Err(MultiModArchiveErrorV1::InvalidSidecarEntry(
            descriptor.entry_name.clone(),
        ));
    }
    let leaf = descriptor
        .entry_name
        .strip_prefix(&prefix)
        .and_then(|value| value.strip_suffix(".arrow"))
        .ok_or_else(|| {
            MultiModArchiveErrorV1::InvalidSidecarEntry(descriptor.entry_name.clone())
        })?;
    let (evidence_kind, table_role) = descriptor
        .evidence_role
        .split_once(':')
        .expect("validated evidence role");
    if !(leaf.starts_with(&format!("{evidence_kind}-"))
        || leaf.contains(&format!("-{evidence_kind}-")))
        || !leaf.ends_with(&format!("-{table_role}"))
    {
        return Err(MultiModArchiveErrorV1::InvalidSidecarEntry(
            descriptor.entry_name.clone(),
        ));
    }
    validate_sha256("sidecar.sha256", &descriptor.sha256)?;
    validate_sha256("sidecar.identity_sha256", &descriptor.identity_sha256)?;
    Ok(())
}

fn validate_stable_result_id(result_id: &str) -> Result<(), MultiModArchiveErrorV1> {
    if result_id.is_empty()
        || result_id.trim() != result_id
        || result_id.contains('/')
        || result_id.contains('\\')
        || result_id.contains("..")
        || result_id.chars().any(char::is_control)
    {
        Err(MultiModArchiveErrorV1::InvalidResultId(result_id.into()))
    } else {
        Ok(())
    }
}

fn validate_sha256(field: &'static str, value: &str) -> Result<(), MultiModArchiveErrorV1> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(MultiModArchiveErrorV1::InvalidSha256(field))
    }
}

fn sha256_json(value: &impl Serialize) -> Result<String, serde_json::Error> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn sha256_file_handle_v1(file: &mut File) -> Result<String, std::io::Error> {
    file.seek(SeekFrom::Start(0))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(windows)]
fn open_pinned_multimod_source_v1(path: &Path) -> Result<File, std::io::Error> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
    };

    let file = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let attributes = file.metadata()?.file_attributes();
    if attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        return Err(std::io::Error::other(
            "MultiMod source must be a regular non-reparse file",
        ));
    }
    Ok(file)
}

#[cfg(not(windows))]
fn open_pinned_multimod_source_v1(path: &Path) -> Result<File, std::io::Error> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(std::io::Error::other(
            "MultiMod source must be a regular non-symlink file",
        ));
    }
    File::open(path)
}

fn sibling_temporary_path(path: &Path, suffix: &str) -> Result<PathBuf, std::io::Error> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("archive path has no parent"))?;
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("archive path has no filename"))?
        .to_string_lossy();
    Ok(parent.join(format!(".{name}.{}.{}", Uuid::new_v4(), suffix)))
}
