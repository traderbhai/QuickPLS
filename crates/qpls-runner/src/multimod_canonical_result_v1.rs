//! Canonical export projection for the versioned MultiMod result families.
//!
//! This adapter deliberately keeps the scientific result as the source of
//! truth. It validates that result first, verifies its immutable identities
//! against an explicit run context, projects every retained field into
//! capability-attributed canonical tables, and validates the final document.
//! `CanonicalResultDocumentV2` has no first-class MultiMod payload, so this
//! module does not use `general_sem_results` or imply release qualification.

use qpls_core::{
    CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION, CanonicalChartDisplayOptions, CanonicalChartKind,
    CanonicalChartPoint, CanonicalChartSeries, CanonicalChartX, CanonicalColumnRole,
    CanonicalColumnType, CanonicalMissingReason, CanonicalResultCell, CanonicalResultChart,
    CanonicalResultColumn, CanonicalResultDocumentV2, CanonicalResultExclusion,
    CanonicalResultFootnote, CanonicalResultNotice, CanonicalResultPresentationV2,
    CanonicalResultProvenanceV2, CanonicalResultRow, CanonicalResultSection, CanonicalResultTable,
    CapabilityCellReferenceV2, GeneralSemConditionalProcessResultV2,
    HeterogeneityCandidateMethodV2, InterventionalMediationResultV1, MultiModAnalysisResultV1,
    MultiModResultValidationErrorV1, MultimodIntervalV1, MultimodProvenanceV1,
    MultimodQualificationStateV1, MultimodReplicateLedgerSummaryV1,
    MultimodResultSidecarDescriptorV1, PlsHeterogeneityAnalysisV2, PlsMultigroupAnalysisV1,
    canonical_result_use_eligibility_v2, sha256_serialized, validate_canonical_result_document_v2,
};
use serde::Serialize;

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiModCanonicalRunContextV1 {
    pub run_id: String,
    pub project_id: String,
    pub recipe_id: String,
    pub recipe_analytical_sha256: String,
    pub model_id: String,
    pub model_scientific_sha256: String,
    pub dataset_id: String,
    pub dataset_fingerprint: String,
    pub engine_version: String,
    pub workers: u32,
    pub started_at: String,
    pub completed_at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MultiModCanonicalResultErrorV1 {
    #[error("invalid MultiMod source result: {0}")]
    Source(#[from] MultiModResultValidationErrorV1),
    #[error("invalid MultiMod canonical run context: {0}")]
    Context(String),
    #[error("failed to serialize a typed MultiMod identity: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("canonical MultiMod projection is invalid: {0}")]
    Canonical(String),
}

struct FamilyProjection {
    sections: Vec<CanonicalResultSection>,
    tables: Vec<CanonicalResultTable>,
    charts: Vec<CanonicalResultChart>,
    notices: Vec<CanonicalResultNotice>,
    exclusions: Vec<CanonicalResultExclusion>,
    default_section_id: String,
    default_table_id: String,
}

pub fn build_multimod_canonical_result_v2(
    context: &MultiModCanonicalRunContextV1,
    result: &MultiModAnalysisResultV1,
) -> Result<CanonicalResultDocumentV2, MultiModCanonicalResultErrorV1> {
    result.ensure_valid()?;
    let provenance = result_provenance(result);
    validate_context(context, provenance)?;

    let capability = provenance.capability_cell.clone();
    let capability_cells = vec![capability.clone()];
    let (family_id, title, schema_version, profile_id, policy_id) = family_identity(result)?;
    let sidecars = result.sidecars();

    let provenance_table = build_provenance_table(
        context,
        provenance,
        family_id,
        schema_version,
        &profile_id,
        &policy_id,
        sidecars.len(),
        &capability_cells,
    )?;
    let sidecar_table = build_sidecar_table(sidecars, &capability_cells)?;
    let mut sections = vec![section(
        "multimod_provenance",
        "Provenance and retained evidence",
        "Exact run, model, dataset, method, capability, qualification, and sidecar identities.",
        vec![
            "multimod_run_provenance",
            "multimod_sidecar_inventory",
            "multimod_scientific_boundaries",
        ],
        Vec::new(),
        &capability_cells,
    )];
    let mut tables = vec![provenance_table, sidecar_table];

    let mut projection = match result {
        MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(value) => {
            project_mga(value, &capability_cells)?
        }
        MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(value) => {
            project_heterogeneity(value, &capability_cells)?
        }
        MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(value) => {
            project_conditional_process(value, &capability_cells)?
        }
        MultiModAnalysisResultV1::InterventionalMediationResultV1(value) => {
            project_interventional_mediation(value, &capability_cells)?
        }
    };
    sections.append(&mut projection.sections);
    tables.append(&mut projection.tables);

    let mut notices = qualification_notices(provenance, &projection.default_section_id);
    notices.append(&mut projection.notices);
    let mut exclusions = vec![CanonicalResultExclusion {
        id: "release_qualification_not_implied".into(),
        capability_cell: Some(capability.clone()),
        title: "Release qualification is not implied".into(),
        reason: "Canonical validation proves schema integrity only. Labs/candidate qualification remains bound to live source, reference, simulation, persistence, export, and installed-application evidence for the exact commit."
            .into(),
    }];
    exclusions.append(&mut projection.exclusions);
    tables.push(build_exclusions_table(&exclusions, &capability_cells));

    let document = CanonicalResultDocumentV2 {
        schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
        document_id: stable_id(
            "multimod_result",
            &(
                context.run_id.as_str(),
                family_id,
                provenance.recipe_analytical_sha256.as_str(),
            ),
        ),
        title: title.into(),
        provenance: CanonicalResultProvenanceV2 {
            run_id: context.run_id.clone(),
            project_id: context.project_id.clone(),
            model_id: context.model_id.clone(),
            model_digest: context.model_scientific_sha256.clone(),
            dataset_id: context.dataset_id.clone(),
            dataset_fingerprint: context.dataset_fingerprint.clone(),
            recipe_id: provenance.recipe_id.clone(),
            recipe_digest: provenance.recipe_analytical_sha256.clone(),
            capability_cell: capability,
            method_version: provenance.method_version.clone(),
            engine_version: provenance.engine_version.clone(),
            seed: Some(i64::try_from(provenance.seed).map_err(|_| {
                MultiModCanonicalResultErrorV1::Context(
                    "result seed cannot be represented by the canonical safe-integer contract"
                        .into(),
                )
            })?),
            workers: i64::from(context.workers),
            started_at: context.started_at.clone(),
            completed_at: context.completed_at.clone(),
        },
        capability_cells: Some(capability_cells.clone()),
        general_sem_results: None,
        sections,
        tables,
        charts: projection.charts,
        notices,
        exclusions,
        footnotes: common_footnotes(),
        presentation: CanonicalResultPresentationV2 {
            default_section_id: Some(projection.default_section_id),
            default_table_id: Some(projection.default_table_id),
            precision: 6,
            missing_value_label: "not available".into(),
            chart_defaults: CanonicalChartDisplayOptions {
                palette: Some("quickpls_accessible".into()),
                show_legend: Some(true),
                show_values: Some(false),
                x_axis_label: None,
                y_axis_label: None,
            },
        },
    };

    let validation = validate_canonical_result_document_v2(&document);
    if !validation.passed {
        return Err(MultiModCanonicalResultErrorV1::Canonical(
            validation.errors.join("; "),
        ));
    }
    // This is intentionally evaluated as an additional guard. V2 currently
    // treats capability attribution as qualification-export eligibility; the
    // explicit Labs notice/exclusion above preserves the stricter boundary.
    if !canonical_result_use_eligibility_v2(&document).readable {
        return Err(MultiModCanonicalResultErrorV1::Canonical(
            "canonical projection is not readable under the V2 use-eligibility contract".into(),
        ));
    }
    Ok(document)
}

fn result_provenance(result: &MultiModAnalysisResultV1) -> &MultimodProvenanceV1 {
    match result {
        MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(value) => &value.provenance,
        MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(value) => &value.provenance,
        MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(value) => &value.provenance,
        MultiModAnalysisResultV1::InterventionalMediationResultV1(value) => &value.provenance,
    }
}

fn validate_context(
    context: &MultiModCanonicalRunContextV1,
    provenance: &MultimodProvenanceV1,
) -> Result<(), MultiModCanonicalResultErrorV1> {
    if context.run_id.trim().is_empty()
        || context.project_id.trim().is_empty()
        || context.recipe_id.trim().is_empty()
        || context.model_id.trim().is_empty()
        || context.dataset_id.trim().is_empty()
        || context.engine_version.trim().is_empty()
    {
        return Err(MultiModCanonicalResultErrorV1::Context(
            "run, project, model, and dataset identities must be nonempty".into(),
        ));
    }
    if context.workers == 0 {
        return Err(MultiModCanonicalResultErrorV1::Context(
            "workers must be positive".into(),
        ));
    }
    if context.recipe_id != provenance.recipe_id {
        return Err(MultiModCanonicalResultErrorV1::Context(
            "recipe identifier differs from result provenance".into(),
        ));
    }
    if context.recipe_analytical_sha256 != provenance.recipe_analytical_sha256 {
        return Err(MultiModCanonicalResultErrorV1::Context(
            "recipe analytical SHA-256 differs from result provenance".into(),
        ));
    }
    if context.model_id != provenance.model_id {
        return Err(MultiModCanonicalResultErrorV1::Context(
            "model identifier differs from result provenance".into(),
        ));
    }
    if context.model_scientific_sha256 != provenance.model_scientific_sha256 {
        return Err(MultiModCanonicalResultErrorV1::Context(
            "model scientific SHA-256 differs from result provenance".into(),
        ));
    }
    if context.dataset_id != provenance.dataset_id {
        return Err(MultiModCanonicalResultErrorV1::Context(
            "dataset identifier differs from result provenance".into(),
        ));
    }
    if context.dataset_fingerprint != provenance.dataset_fingerprint {
        return Err(MultiModCanonicalResultErrorV1::Context(
            "dataset fingerprint differs from result provenance".into(),
        ));
    }
    if context.engine_version != provenance.engine_version {
        return Err(MultiModCanonicalResultErrorV1::Context(
            "engine version differs from result provenance".into(),
        ));
    }
    if provenance.seed > MAX_SAFE_INTEGER {
        return Err(MultiModCanonicalResultErrorV1::Context(
            "result seed exceeds the canonical safe-integer limit".into(),
        ));
    }
    Ok(())
}

fn family_identity(
    result: &MultiModAnalysisResultV1,
) -> Result<(&'static str, &'static str, u32, String, String), MultiModCanonicalResultErrorV1> {
    Ok(match result {
        MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(value) => (
            "pls_multigroup_analysis_v1",
            "PLS multigroup analysis",
            value.schema_version,
            enum_name(&value.profile)?,
            enum_name(&value.multiplicity)?,
        ),
        MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(value) => (
            "pls_heterogeneity_analysis_v2",
            "PLS unobserved heterogeneity analysis",
            value.schema_version,
            enum_name(&value.profile)?,
            match (value.locked_algorithm, value.locked_k) {
                (Some(algorithm), Some(k)) => format!("locked:{}:k{k}", enum_name(&algorithm)?),
                _ => "discovery_not_locked".into(),
            },
        ),
        MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(value) => (
            "general_sem_conditional_process_result_v2",
            "General SEM conditional-process analysis",
            value.schema_version,
            value.profile_id.clone(),
            "shared_resample_ledger".into(),
        ),
        MultiModAnalysisResultV1::InterventionalMediationResultV1(value) => (
            "interventional_mediation_result_v1",
            "Interventional causal-mediation analysis",
            value.schema_version,
            "observed_data_parametric_g_computation".into(),
            value.interpretation_label.clone(),
        ),
    })
}

fn build_provenance_table(
    context: &MultiModCanonicalRunContextV1,
    provenance: &MultimodProvenanceV1,
    family_id: &str,
    schema_version: u32,
    profile_id: &str,
    policy_id: &str,
    sidecar_count: usize,
    capabilities: &[CapabilityCellReferenceV2],
) -> Result<CanonicalResultTable, MultiModCanonicalResultErrorV1> {
    let candidate_receipt_cell = match &provenance.candidate_qualification_receipt {
        Some(receipt) => text(serde_json::to_string(receipt)?),
        None => missing(CanonicalMissingReason::NotApplicable),
    };
    Ok(table(
        "multimod_run_provenance",
        "MultiMod run provenance",
        "Immutable run context, typed family/profile, scientific digests, capability ownership, and qualification state.",
        vec![
            text_column(
                "family",
                "Result family",
                "Versioned MultiMod result family.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "family_schema_version",
                "Family schema",
                "Result-family schema version.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "profile",
                "Profile",
                "Exact admitted profile identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "policy",
                "Policy",
                "Multiplicity, lock, ledger, or interpretation policy identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "qualification",
                "Qualification state",
                "Typed result qualification state.",
                CanonicalColumnRole::Decision,
            ),
            text_column(
                "candidate_qualification_receipt_json",
                "Candidate authority receipt",
                "Exact build-embedded candidate authority receipt; absent for Labs and failed-closed results.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "run_id",
                "Run ID",
                "Execution run identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "project_id",
                "Project ID",
                "Project identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "model_id",
                "Model ID",
                "Scientific model identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "model_sha256",
                "Model SHA-256",
                "Scientific model digest.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "dataset_id",
                "Dataset ID",
                "Resident dataset identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "dataset_fingerprint",
                "Dataset fingerprint",
                "Exact dataset fingerprint.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "recipe_id",
                "Recipe ID",
                "Recipe identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "recipe_sha256",
                "Recipe SHA-256",
                "Analytical recipe digest.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "config_sha256",
                "Config SHA-256",
                "MultiMod configuration digest.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "method_version",
                "Method version",
                "Frozen statistical method identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "engine_version",
                "Engine version",
                "Executing engine version.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "seed",
                "Seed",
                "Deterministic analysis seed.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "workers",
                "Workers",
                "Worker count used by execution.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "started_at",
                "Started",
                "ISO-8601 start time.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "completed_at",
                "Completed",
                "ISO-8601 completion time.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "capability_id",
                "Capability ID",
                "Owning capability identity.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "capability_registry_schema",
                "Capability registry schema",
                "Capability-registry schema version.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "capability_cell_id",
                "Capability cell",
                "Owning capability cell identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "capability_version",
                "Capability version",
                "Owning capability version.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "sidecar_count",
                "Sidecars",
                "Required scientific sidecars retained in the archive.",
                CanonicalColumnRole::Diagnostic,
            ),
        ],
        vec![CanonicalResultRow {
            id: "run".into(),
            cells: vec![
                text(family_id),
                number(f64::from(schema_version)),
                text(profile_id),
                text(policy_id),
                text(enum_name(&provenance.qualification)?),
                candidate_receipt_cell,
                text(&context.run_id),
                text(&context.project_id),
                text(&context.model_id),
                text(&context.model_scientific_sha256),
                text(&context.dataset_id),
                text(&context.dataset_fingerprint),
                text(&provenance.recipe_id),
                text(&provenance.recipe_analytical_sha256),
                text(&provenance.config_sha256),
                text(&provenance.method_version),
                text(&provenance.engine_version),
                number(provenance.seed as f64),
                number(f64::from(context.workers)),
                text(&context.started_at),
                text(&context.completed_at),
                text(&provenance.capability_cell.capability_id),
                number(f64::from(
                    provenance.capability_cell.registry_schema_version,
                )),
                text(&provenance.capability_cell.cell_id),
                text(&provenance.capability_cell.capability_version),
                number(usize_number(sidecar_count)?),
            ],
        }],
        vec!["multimod_labs_boundary"],
        capabilities,
    ))
}

fn build_sidecar_table(
    sidecars: &[MultimodResultSidecarDescriptorV1],
    capabilities: &[CapabilityCellReferenceV2],
) -> Result<CanonicalResultTable, MultiModCanonicalResultErrorV1> {
    let mut rows = Vec::with_capacity(sidecars.len());
    for sidecar in sidecars {
        rows.push(CanonicalResultRow {
            id: stable_id("sidecar", &sidecar.entry_name),
            cells: vec![
                number(f64::from(sidecar.schema_version)),
                text(&sidecar.entry_name),
                text(&sidecar.media_type),
                text(&sidecar.compression),
                text(&sidecar.arrow_schema_sha256),
                number(u64_number(sidecar.row_count)?),
                number(f64::from(sidecar.column_count)),
                number(u64_number(sidecar.uncompressed_bytes)?),
                text(&sidecar.sha256),
                text(&sidecar.identity_sha256),
                boolean(sidecar.required_for_scientific_reopen),
            ],
        });
    }
    Ok(table(
        "multimod_sidecar_inventory",
        "Scientific sidecar inventory",
        "Complete Arrow sidecar identity, shape, compression, digest, and strict-reopen requirement inventory.",
        vec![
            number_column(
                "descriptor_schema",
                "Descriptor schema",
                "MultiMod sidecar-descriptor schema version.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "entry_name",
                "Archive entry",
                "Exact .qpls archive entry name.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "media_type",
                "Media type",
                "Sidecar media type.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "compression",
                "Compression",
                "Sidecar compression identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "arrow_schema_sha256",
                "Arrow schema SHA-256",
                "Canonical Arrow schema digest.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "rows",
                "Rows",
                "Arrow row count.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "columns",
                "Columns",
                "Arrow column count.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "uncompressed_bytes",
                "Uncompressed bytes",
                "Uncompressed sidecar size.",
                CanonicalColumnRole::Diagnostic,
            ),
            text_column(
                "content_sha256",
                "Content SHA-256",
                "Stored content digest.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "identity_sha256",
                "Identity SHA-256",
                "Scientific sidecar identity digest.",
                CanonicalColumnRole::Provenance,
            ),
            boolean_column(
                "required_for_reopen",
                "Required for reopen",
                "Whether strict scientific reopening requires this sidecar.",
                CanonicalColumnRole::Decision,
            ),
        ],
        rows,
        Vec::new(),
        capabilities,
    ))
}

fn build_exclusions_table(
    exclusions: &[CanonicalResultExclusion],
    capabilities: &[CapabilityCellReferenceV2],
) -> CanonicalResultTable {
    table(
        "multimod_scientific_boundaries",
        "Scientific exclusions and boundaries",
        "Complete machine-exportable mirror of the canonical exclusions that prevent overclaiming qualification, methods, or interpretation.",
        vec![
            text_column(
                "exclusion_id",
                "Exclusion ID",
                "Stable scientific-boundary identity.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "title",
                "Title",
                "Concise boundary title.",
                CanonicalColumnRole::Decision,
            ),
            text_column(
                "reason",
                "Reason",
                "Complete boundary rationale.",
                CanonicalColumnRole::Diagnostic,
            ),
            text_column(
                "capability_id",
                "Capability ID",
                "Owning capability when present.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "capability_cell_id",
                "Capability cell",
                "Owning capability cell when present.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "capability_version",
                "Capability version",
                "Owning capability version when present.",
                CanonicalColumnRole::Provenance,
            ),
        ],
        exclusions
            .iter()
            .map(|exclusion| CanonicalResultRow {
                id: stable_id("boundary", &exclusion.id),
                cells: vec![
                    text(&exclusion.id),
                    text(&exclusion.title),
                    text(&exclusion.reason),
                    optional_text(
                        exclusion
                            .capability_cell
                            .as_ref()
                            .map(|cell| cell.capability_id.as_str()),
                        CanonicalMissingReason::NotApplicable,
                    ),
                    optional_text(
                        exclusion
                            .capability_cell
                            .as_ref()
                            .map(|cell| cell.cell_id.as_str()),
                        CanonicalMissingReason::NotApplicable,
                    ),
                    optional_text(
                        exclusion
                            .capability_cell
                            .as_ref()
                            .map(|cell| cell.capability_version.as_str()),
                        CanonicalMissingReason::NotApplicable,
                    ),
                ],
            })
            .collect(),
        Vec::new(),
        capabilities,
    )
}

fn project_mga(
    result: &PlsMultigroupAnalysisV1,
    capabilities: &[CapabilityCellReferenceV2],
) -> Result<FamilyProjection, MultiModCanonicalResultErrorV1> {
    let eligibility = table(
        "mga_group_eligibility",
        "Group eligibility",
        "Selected and complete-case counts plus the exact eligibility decision for every typed group.",
        vec![
            text_column(
                "group_id",
                "Group ID",
                "Stable selected-group identity.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "label",
                "Label",
                "Displayed group label.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "selected_rows",
                "Selected rows",
                "Rows matching the typed group value before complete-case exclusion.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "complete_cases",
                "Complete cases",
                "Complete model cases retained in the group.",
                CanonicalColumnRole::Diagnostic,
            ),
            boolean_column(
                "eligible",
                "Eligible",
                "Whether the group passed all frozen admission gates.",
                CanonicalColumnRole::Decision,
            ),
            number_column(
                "warnings",
                "Warnings",
                "Number of eligibility warnings.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "blockers",
                "Blockers",
                "Number of eligibility blockers.",
                CanonicalColumnRole::Diagnostic,
            ),
        ],
        result
            .group_eligibility
            .iter()
            .map(|group| {
                Ok(CanonicalResultRow {
                    id: stable_id("group", &group.group_id),
                    cells: vec![
                        text(&group.group_id),
                        text(&group.label),
                        number(u64_number(group.selected_rows)?),
                        number(u64_number(group.complete_cases)?),
                        boolean(group.eligible),
                        number(usize_number(group.warnings.len())?),
                        number(usize_number(group.blockers.len())?),
                    ],
                })
            })
            .collect::<Result<Vec<_>, MultiModCanonicalResultErrorV1>>()?,
        Vec::new(),
        capabilities,
    );

    let mut eligibility_messages = Vec::new();
    for group in &result.group_eligibility {
        for (index, message) in group.warnings.iter().enumerate() {
            eligibility_messages.push(CanonicalResultRow {
                id: stable_id(
                    "eligibility_message",
                    &(group.group_id.as_str(), "warning", index, message.as_str()),
                ),
                cells: vec![text(&group.group_id), text("warning"), text(message)],
            });
        }
        for (index, message) in group.blockers.iter().enumerate() {
            eligibility_messages.push(CanonicalResultRow {
                id: stable_id(
                    "eligibility_message",
                    &(group.group_id.as_str(), "blocker", index, message.as_str()),
                ),
                cells: vec![text(&group.group_id), text("blocker"), text(message)],
            });
        }
    }
    let eligibility_messages = table(
        "mga_group_eligibility_messages",
        "Group eligibility messages",
        "Complete warning and blocker inventory retained without parsing display prose.",
        vec![
            text_column(
                "group_id",
                "Group ID",
                "Selected-group identity.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "severity",
                "Kind",
                "Warning or blocker.",
                CanonicalColumnRole::Decision,
            ),
            text_column(
                "message",
                "Message",
                "Exact eligibility message.",
                CanonicalColumnRole::Diagnostic,
            ),
        ],
        eligibility_messages,
        Vec::new(),
        capabilities,
    );

    let mut parameter_rows = Vec::with_capacity(result.group_parameters.len());
    for row in &result.group_parameters {
        let mut cells = vec![
            text(&row.group_id),
            text(&row.parameter.target_id),
            text(&row.parameter.target_kind),
            number(row.parameter.estimate),
            optional_number(
                row.parameter.standard_error,
                CanonicalMissingReason::NotEstimated,
            ),
            optional_number(row.parameter.p_value, CanonicalMissingReason::NotEstimated),
        ];
        cells.extend(interval_cells(row.parameter.interval.as_ref())?);
        parameter_rows.push(CanonicalResultRow {
            id: stable_id(
                "group_parameter",
                &(row.group_id.as_str(), row.parameter.target_id.as_str()),
            ),
            cells,
        });
    }
    let group_parameters = table(
        "mga_group_parameters",
        "Group-specific parameters",
        "Group-specific path, loading, weight, interaction, or other explicitly selected parameter estimates and inference.",
        parameter_columns("Group ID"),
        parameter_rows,
        vec!["one_sided_interval_bounds"],
        capabilities,
    );

    let micom = table(
        "mga_micom_pairs",
        "Pairwise MICOM results",
        "Pairwise configural, compositional, mean, and variance invariance results; no omnibus MICOM claim is made.",
        vec![
            text_column(
                "left_group",
                "Left group",
                "Left member of the ordered pair.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "right_group",
                "Right group",
                "Right member of the ordered pair.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "construct_id",
                "Construct",
                "Construct evaluated by MICOM.",
                CanonicalColumnRole::Label,
            ),
            boolean_column(
                "configural_confirmed",
                "Configural confirmed",
                "Whether the structured Step 1 checklist was confirmed.",
                CanonicalColumnRole::Decision,
            ),
            number_column(
                "compositional_correlation",
                "Compositional correlation",
                "Observed Step 2 compositional correlation.",
                CanonicalColumnRole::Estimate,
            ),
            number_column(
                "compositional_probability",
                "Compositional probability",
                "Directional permutation probability retained by the MICOM contract.",
                CanonicalColumnRole::Uncertainty,
            ),
            boolean_column(
                "partial_invariance",
                "Partial invariance",
                "Whether Steps 1 and 2 establish partial measurement invariance.",
                CanonicalColumnRole::Decision,
            ),
            number_column(
                "equal_mean_probability",
                "Equal-mean probability",
                "Step 3 mean-equality permutation probability.",
                CanonicalColumnRole::Uncertainty,
            ),
            number_column(
                "equal_variance_probability",
                "Equal-variance probability",
                "Step 3 variance-equality permutation probability.",
                CanonicalColumnRole::Uncertainty,
            ),
        ],
        result
            .micom_pairs
            .iter()
            .enumerate()
            .map(|(index, row)| CanonicalResultRow {
                id: stable_id(
                    "micom",
                    &(
                        index,
                        row.left_group_id.as_str(),
                        row.right_group_id.as_str(),
                        row.construct_id.as_str(),
                    ),
                ),
                cells: vec![
                    text(&row.left_group_id),
                    text(&row.right_group_id),
                    text(&row.construct_id),
                    boolean(row.configural_invariance_confirmed),
                    number(row.compositional_correlation),
                    number(row.compositional_p_value),
                    boolean(row.partial_invariance),
                    number(row.equal_mean_p_value),
                    number(row.equal_variance_p_value),
                ],
            })
            .collect(),
        vec!["micom_interpretation"],
        capabilities,
    );

    let omnibus = table(
        "mga_omnibus_comparisons",
        "K-group omnibus comparisons",
        "Explicit max-spread permutation or inverse-variance Wald omnibus tests, separate from pairwise follow-up.",
        vec![
            text_column(
                "procedure",
                "Procedure",
                "Frozen omnibus procedure identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "target_id",
                "Target",
                "Scientific parameter target.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "statistic",
                "Statistic",
                "Observed omnibus statistic.",
                CanonicalColumnRole::Estimate,
            ),
            number_column(
                "degrees_of_freedom",
                "Degrees of freedom",
                "Procedure degrees of freedom.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "p_value",
                "p value",
                "Omnibus probability.",
                CanonicalColumnRole::Uncertainty,
            ),
        ],
        result
            .omnibus
            .iter()
            .enumerate()
            .map(|(index, row)| CanonicalResultRow {
                id: stable_id(
                    "omnibus",
                    &(index, row.procedure.as_str(), row.target_id.as_str()),
                ),
                cells: vec![
                    text(&row.procedure),
                    text(&row.target_id),
                    number(row.statistic),
                    number(f64::from(row.degrees_of_freedom)),
                    number(row.p_value),
                ],
            })
            .collect(),
        Vec::new(),
        capabilities,
    );

    let mut pairwise_rows = Vec::with_capacity(result.pairwise.len());
    for (index, row) in result.pairwise.iter().enumerate() {
        let mut cells = vec![
            text(&row.procedure),
            text(&row.left_group_id),
            text(&row.right_group_id),
            text(&row.target_id),
            number(row.difference_left_minus_right),
            optional_number(row.raw_p_value, CanonicalMissingReason::NotEstimated),
            optional_number(row.adjusted_p_value, CanonicalMissingReason::NotApplicable),
            optional_number(
                row.directional_probability,
                CanonicalMissingReason::NotApplicable,
            ),
        ];
        cells.extend(interval_cells(row.interval.as_ref())?);
        cells.extend([
            boolean(row.measurement_comparability_satisfied),
            boolean(row.interpretation_blocked),
        ]);
        pairwise_rows.push(CanonicalResultRow {
            id: stable_id(
                "pairwise",
                &(
                    index,
                    row.procedure.as_str(),
                    row.left_group_id.as_str(),
                    row.right_group_id.as_str(),
                    row.target_id.as_str(),
                ),
            ),
            cells,
        });
    }
    let pairwise = table(
        "mga_pairwise_comparisons",
        "Pairwise group comparisons",
        "Signed left-minus-right differences with procedure-specific probability, multiplicity, interval, and measurement-comparability state.",
        vec![
            text_column("procedure", "Procedure", "Frozen pairwise procedure identity.", CanonicalColumnRole::Provenance),
            text_column("left_group", "Left group", "Minuend group in the signed contrast.", CanonicalColumnRole::Label),
            text_column("right_group", "Right group", "Subtrahend group in the signed contrast.", CanonicalColumnRole::Label),
            text_column("target_id", "Target", "Scientific parameter target.", CanonicalColumnRole::Label),
            number_column("difference", "Left minus right", "Signed parameter difference.", CanonicalColumnRole::Estimate),
            number_column("raw_p_value", "Raw p value", "Unadjusted procedure probability when defined.", CanonicalColumnRole::Uncertainty),
            number_column("adjusted_p_value", "Adjusted p value", "Multiplicity-adjusted probability when defined.", CanonicalColumnRole::Uncertainty),
            number_column("directional_probability", "Directional probability", "Henseler PLS-MGA directional probability; not relabelled as an ordinary two-sided p value.", CanonicalColumnRole::Uncertainty),
        ].into_iter().chain(interval_columns()).chain([
            boolean_column("measurement_comparable", "Measurement comparable", "Whether the required partial-invariance gate is satisfied.", CanonicalColumnRole::Decision),
            boolean_column("interpretation_blocked", "Interpretation blocked", "Whether structural interpretation is suppressed.", CanonicalColumnRole::Decision),
        ]).collect(),
        pairwise_rows,
        vec!["one_sided_interval_bounds", "directional_probability", "micom_interpretation"],
        capabilities,
    );

    let excluded_rows = table(
        "mga_excluded_rows",
        "Excluded source rows",
        "Stable row-token inventory for unselected levels and unusable rows, including the exact typed group value and reason.",
        vec![
            text_column(
                "row_token",
                "Stable row token",
                "Non-positional source-row identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "typed_group_value",
                "Typed group value",
                "Type-preserving group value receipt.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "reason",
                "Reason",
                "Typed exclusion reason.",
                CanonicalColumnRole::Decision,
            ),
        ],
        result
            .excluded_rows
            .iter()
            .map(|row| {
                Ok(CanonicalResultRow {
                    id: stable_id("excluded", &row.stable_row_token),
                    cells: vec![
                        text(&row.stable_row_token),
                        text(&row.typed_group_value),
                        text(enum_name(&row.reason)?),
                    ],
                })
            })
            .collect::<Result<Vec<_>, MultiModCanonicalResultErrorV1>>()?,
        Vec::new(),
        capabilities,
    );

    let (ledger_tables, ledger_ids) =
        build_ledger_tables(result.replicate_ledgers.iter().collect(), capabilities)?;
    let pairwise_chart = interval_chart(
        "mga_pairwise_intervals",
        "Pairwise differences and confidence bounds",
        "Signed left-minus-right pairwise estimates with only the bounds actually defined by each alternative.",
        "mga_pairwise_comparisons",
        result
            .pairwise
            .iter()
            .filter_map(|row| {
                row.interval.as_ref().map(|interval| {
                    (
                        format!(
                            "{}: {} - {}: {}",
                            row.procedure, row.left_group_id, row.right_group_id, row.target_id
                        ),
                        row.difference_left_minus_right,
                        interval.lower,
                        interval.upper,
                    )
                })
            })
            .collect(),
    );
    let mut charts = Vec::new();
    let mut comparison_chart_ids = Vec::new();
    if let Some(chart) = pairwise_chart {
        comparison_chart_ids.push(chart.id.clone());
        charts.push(chart);
    }

    let mut tables = vec![
        eligibility,
        eligibility_messages,
        group_parameters,
        micom,
        omnibus,
        pairwise,
        excluded_rows,
    ];
    tables.extend(ledger_tables);
    let mut inference_table_ids = vec![
        "mga_group_parameters",
        "mga_micom_pairs",
        "mga_omnibus_comparisons",
        "mga_pairwise_comparisons",
    ];
    inference_table_ids.extend(ledger_ids.iter().map(String::as_str));
    let notices = result
        .group_eligibility
        .iter()
        .enumerate()
        .filter(|(_, group)| !group.eligible)
        .map(|(index, group)| CanonicalResultNotice {
            id: stable_id("mga_ineligible_group", &(index, group.group_id.as_str())),
            code: "multimod_mga_group_ineligible".into(),
            severity: qpls_core::CanonicalNoticeSeverity::Error,
            message: format!(
                "Group {} is ineligible; MGA interpretation must remain fail-closed.",
                group.label
            ),
            section_ids: vec!["mga_groups".into()],
            table_ids: vec!["mga_group_eligibility".into()],
        })
        .collect();
    Ok(FamilyProjection {
        sections: vec![
            section("mga_groups", "Groups and exclusions", "Group admission evidence and complete row-level exclusion receipts.", vec!["mga_group_eligibility", "mga_group_eligibility_messages", "mga_excluded_rows"], Vec::new(), capabilities),
            section("mga_inference", "MICOM and group comparisons", "Group-specific estimates, pairwise MICOM, omnibus tests, pairwise procedures, multiplicity, and complete resampling ledgers.", inference_table_ids, comparison_chart_ids, capabilities),
        ],
        tables,
        charts,
        notices,
        exclusions: vec![CanonicalResultExclusion {
            id: "mga_no_omnibus_micom".into(), capability_cell: Some(result.provenance.capability_cell.clone()),
            title: "No omnibus MICOM claim".into(), reason: "MICOM Steps 2 and 3 are pairwise. K-group omnibus inference applies to selected parameter differences, not to an invented omnibus MICOM procedure.".into(),
        }],
        default_section_id: "mga_inference".into(),
        default_table_id: "mga_pairwise_comparisons".into(),
    })
}

fn project_heterogeneity(
    result: &PlsHeterogeneityAnalysisV2,
    capabilities: &[CapabilityCellReferenceV2],
) -> Result<FamilyProjection, MultiModCanonicalResultErrorV1> {
    let candidates = table(
        "heterogeneity_candidates",
        "Candidate segmentation diagnostics",
        "Algorithm/K candidate state and multi-start diagnostics. Criteria are reported separately and K is never auto-selected.",
        vec![
            text_column(
                "algorithm",
                "Algorithm",
                "Frozen FIMIX-PLS or PLS-POS algorithm identity.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "k",
                "K",
                "Candidate class or segment count; K=1 is the pooled baseline.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "state",
                "State",
                "Typed candidate eligibility, convergence, stability, or failure state.",
                CanonicalColumnRole::Decision,
            ),
            number_column(
                "converged_starts",
                "Converged starts",
                "Number of starts satisfying convergence rules.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "stable_starts",
                "Stable starts",
                "Number of aligned starts reproducing the optimum/partition.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "log_likelihood",
                "Log likelihood",
                "Actual optimized FIMIX log likelihood when applicable.",
                CanonicalColumnRole::Estimate,
            ),
            number_column(
                "objective",
                "Objective",
                "Full-refit PLS-POS objective when applicable.",
                CanonicalColumnRole::Estimate,
            ),
            number_column(
                "blockers",
                "Blockers",
                "Number of retained blocker messages.",
                CanonicalColumnRole::Diagnostic,
            ),
        ],
        result
            .candidates
            .iter()
            .map(|candidate| {
                Ok(CanonicalResultRow {
                    id: stable_id(
                        "candidate",
                        &(
                            heterogeneity_candidate_method_name(&candidate.method)?,
                            candidate.k,
                        ),
                    ),
                    cells: vec![
                        text(heterogeneity_candidate_method_name(&candidate.method)?),
                        number(f64::from(candidate.k)),
                        text(enum_name(&candidate.state)?),
                        number(f64::from(candidate.converged_starts)),
                        number(f64::from(candidate.stable_starts)),
                        optional_number(
                            candidate.log_likelihood,
                            CanonicalMissingReason::NotApplicable,
                        ),
                        optional_number(candidate.objective, CanonicalMissingReason::NotApplicable),
                        number(usize_number(candidate.blockers.len())?),
                    ],
                })
            })
            .collect::<Result<Vec<_>, MultiModCanonicalResultErrorV1>>()?,
        vec!["heterogeneity_exploratory"],
        capabilities,
    );

    let mut criteria_rows = Vec::new();
    let mut share_rows = Vec::new();
    let mut blocker_rows = Vec::new();
    for candidate in &result.candidates {
        let algorithm = heterogeneity_candidate_method_name(&candidate.method)?;
        for (criterion, value) in &candidate.criteria {
            criteria_rows.push(CanonicalResultRow {
                id: stable_id(
                    "criterion",
                    &(algorithm.as_str(), candidate.k, criterion.as_str()),
                ),
                cells: vec![
                    text(&algorithm),
                    number(f64::from(candidate.k)),
                    text(criterion),
                    number(*value),
                ],
            });
        }
        for (index, share) in candidate.class_or_segment_shares.iter().enumerate() {
            share_rows.push(CanonicalResultRow {
                id: stable_id("candidate_share", &(algorithm.as_str(), candidate.k, index)),
                cells: vec![
                    text(&algorithm),
                    number(f64::from(candidate.k)),
                    number((index + 1) as f64),
                    number(*share),
                ],
            });
        }
        for (index, blocker) in candidate.blockers.iter().enumerate() {
            blocker_rows.push(CanonicalResultRow {
                id: stable_id(
                    "candidate_blocker",
                    &(algorithm.as_str(), candidate.k, index, blocker.as_str()),
                ),
                cells: vec![
                    text(&algorithm),
                    number(f64::from(candidate.k)),
                    text(blocker),
                ],
            });
        }
    }
    let criteria = table(
        "heterogeneity_candidate_criteria",
        "Candidate information criteria",
        "Long-form AIC, AIC3, AIC4, BIC, CAIC, HQ, entropy, or other explicitly retained criterion values.",
        vec![
            text_column(
                "algorithm",
                "Algorithm",
                "Candidate algorithm.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "k",
                "K",
                "Candidate class count.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "criterion",
                "Criterion",
                "Exact criterion identity.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "value",
                "Value",
                "Criterion value calculated from the actual optimized likelihood or objective contract.",
                CanonicalColumnRole::Estimate,
            ),
        ],
        criteria_rows,
        Vec::new(),
        capabilities,
    );
    let shares = table(
        "heterogeneity_candidate_shares",
        "Candidate class or segment shares",
        "Aligned share vector for every candidate that retained a partition.",
        vec![
            text_column(
                "algorithm",
                "Algorithm",
                "Candidate algorithm.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "k",
                "K",
                "Candidate class count.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "class_id",
                "Class or segment",
                "One-based aligned class/segment identity.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "share",
                "Share",
                "Estimated class or segment proportion.",
                CanonicalColumnRole::Estimate,
            ),
        ],
        share_rows,
        Vec::new(),
        capabilities,
    );
    let blockers = table(
        "heterogeneity_candidate_blockers",
        "Candidate blockers",
        "Complete typed-candidate blocker prose retained by algorithm and K.",
        vec![
            text_column(
                "algorithm",
                "Algorithm",
                "Candidate algorithm.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "k",
                "K",
                "Candidate class count.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "blocker",
                "Blocker",
                "Exact failure or ineligibility detail.",
                CanonicalColumnRole::Diagnostic,
            ),
        ],
        blocker_rows,
        Vec::new(),
        capabilities,
    );

    let mut parameter_rows = Vec::with_capacity(result.parameters.len());
    for row in &result.parameters {
        let mut cells = vec![
            number(f64::from(row.class_id)),
            text(&row.metric),
            text(&row.parameter.target_id),
            text(&row.parameter.target_kind),
            number(row.parameter.estimate),
            optional_number(
                row.parameter.standard_error,
                CanonicalMissingReason::NotEstimated,
            ),
            optional_number(row.parameter.p_value, CanonicalMissingReason::NotEstimated),
        ];
        cells.extend(interval_cells(row.parameter.interval.as_ref())?);
        parameter_rows.push(CanonicalResultRow {
            id: stable_id(
                "class_parameter",
                &(row.class_id, row.parameter.target_id.as_str()),
            ),
            cells,
        });
    }
    let parameters = table(
        "heterogeneity_class_parameters",
        "Class or segment parameters",
        "Class-specific paths, interaction coefficients, scientific gamma/delta, slopes, residual variances, and other retained parameters on their declared metric.",
        vec![
            number_column(
                "class_id",
                "Class or segment",
                "One-based aligned identity.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "metric",
                "Metric",
                "Pooled common metric or segment-local metric identity.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "target_id",
                "Target",
                "Scientific parameter target.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "target_kind",
                "Target kind",
                "Typed parameter family.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "estimate",
                "Estimate",
                "Point estimate.",
                CanonicalColumnRole::Estimate,
            ),
            number_column(
                "standard_error",
                "Standard error",
                "Standard error when estimated.",
                CanonicalColumnRole::Uncertainty,
            ),
            number_column(
                "p_value",
                "p value",
                "Probability when estimated.",
                CanonicalColumnRole::Uncertainty,
            ),
        ]
        .into_iter()
        .chain(interval_columns())
        .collect(),
        parameter_rows,
        vec!["one_sided_interval_bounds"],
        capabilities,
    );

    let mut contrast_rows = Vec::with_capacity(result.contrasts.len());
    for (index, row) in result.contrasts.iter().enumerate() {
        let mut cells = vec![
            number(f64::from(row.left_class_id)),
            number(f64::from(row.right_class_id)),
            text(&row.target_id),
            number(row.difference),
            optional_number(row.p_value, CanonicalMissingReason::Withheld),
        ];
        cells.extend(interval_cells(row.interval.as_ref())?);
        cells.extend([
            boolean(row.common_metric_comparability_satisfied),
            boolean(row.inferential_interpretation_blocked),
        ]);
        contrast_rows.push(CanonicalResultRow {
            id: stable_id(
                "class_contrast",
                &(
                    index,
                    row.left_class_id,
                    row.right_class_id,
                    row.target_id.as_str(),
                ),
            ),
            cells,
        });
    }
    let contrasts = table(
        "heterogeneity_class_contrasts",
        "Class or segment contrasts",
        "Aligned segment contrasts with the pooled common-metric comparability gate and withheld inference represented explicitly.",
        vec![
            number_column("left_class", "Left class", "Minuend class/segment.", CanonicalColumnRole::Label),
            number_column("right_class", "Right class", "Subtrahend class/segment.", CanonicalColumnRole::Label),
            text_column("target_id", "Target", "Scientific contrast target.", CanonicalColumnRole::Label),
            number_column("difference", "Difference", "Signed left-minus-right difference.", CanonicalColumnRole::Estimate),
            number_column("p_value", "p value", "Probability, withheld when the common-metric gate fails.", CanonicalColumnRole::Uncertainty),
        ].into_iter().chain(interval_columns()).chain([
            boolean_column("common_metric_comparable", "Common metric comparable", "Whether pooled scoring and pairwise compositional-invariance evidence support comparison.", CanonicalColumnRole::Decision),
            boolean_column("interpretation_blocked", "Inference blocked", "Whether segment-to-segment inferential interpretation is suppressed.", CanonicalColumnRole::Decision),
        ]).collect(),
        contrast_rows,
        vec!["one_sided_interval_bounds", "heterogeneity_common_metric"],
        capabilities,
    );

    let ledgers = result.bootstrap_ledger.iter().collect::<Vec<_>>();
    let (ledger_tables, ledger_ids) = build_ledger_tables(ledgers, capabilities)?;
    let mut tables = vec![
        candidates, criteria, shares, blockers, parameters, contrasts,
    ];
    tables.extend(ledger_tables);

    let share_chart = segmentation_share_chart(result)?;
    let contrast_chart = interval_chart(
        "heterogeneity_contrast_intervals",
        "Common-metric contrast intervals",
        "Only contrasts that passed the common-metric gate and retained an interval are shown.",
        "heterogeneity_class_contrasts",
        result
            .contrasts
            .iter()
            .filter(|row| row.common_metric_comparability_satisfied)
            .filter_map(|row| {
                row.interval.as_ref().map(|interval| {
                    (
                        format!(
                            "class {} - {}: {}",
                            row.left_class_id, row.right_class_id, row.target_id
                        ),
                        row.difference,
                        interval.lower,
                        interval.upper,
                    )
                })
            })
            .collect(),
    );
    let mut charts = Vec::new();
    let mut diagnostic_chart_ids = Vec::new();
    if let Some(chart) = share_chart {
        diagnostic_chart_ids.push(chart.id.clone());
        charts.push(chart);
    }
    let mut estimate_chart_ids = Vec::new();
    if let Some(chart) = contrast_chart {
        estimate_chart_ids.push(chart.id.clone());
        charts.push(chart);
    }

    let mut estimate_tables = vec![
        "heterogeneity_class_parameters",
        "heterogeneity_class_contrasts",
    ];
    estimate_tables.extend(ledger_ids.iter().map(String::as_str));
    let mut notices = Vec::new();
    if result.descriptive_only {
        notices.push(CanonicalResultNotice {
            id: "heterogeneity_descriptive_only".into(),
            code: "multimod_heterogeneity_common_metric_gate_failed".into(),
            severity: qpls_core::CanonicalNoticeSeverity::Warning,
            message: "Segment-local estimates are descriptive only; segment-to-segment gamma, delta, slope, and effect inference is suppressed because common-metric comparability was not established.".into(),
            section_ids: vec!["heterogeneity_estimates".into()],
            table_ids: vec!["heterogeneity_class_contrasts".into()],
        });
    }
    if result.locked_algorithm.is_none() {
        notices.push(CanonicalResultNotice {
            id: "heterogeneity_not_locked".into(),
            code: "multimod_heterogeneity_discovery_not_locked".into(),
            severity: qpls_core::CanonicalNoticeSeverity::Information,
            message: "No algorithm/K is locked; candidate diagnostics are exploratory and bootstrap discovery/model selection was not performed.".into(),
            section_ids: vec!["heterogeneity_diagnostics".into()],
            table_ids: vec!["heterogeneity_candidates".into()],
        });
    }

    Ok(FamilyProjection {
        sections: vec![
            section("heterogeneity_diagnostics", "Candidate diagnostics", "FIMIX-PLS and PLS-POS candidate convergence, stability, criteria, shares, and blockers.", vec!["heterogeneity_candidates", "heterogeneity_candidate_criteria", "heterogeneity_candidate_shares", "heterogeneity_candidate_blockers"], diagnostic_chart_ids, capabilities),
            section("heterogeneity_estimates", "Locked-segmentation estimates", "Class/segment parameters, common-metric contrasts, and fixed-algorithm/fixed-K bootstrap evidence.", estimate_tables, estimate_chart_ids, capabilities),
        ],
        tables,
        charts,
        notices,
        exclusions: vec![CanonicalResultExclusion {
            id: "heterogeneity_no_automatic_k_selection".into(),
            capability_cell: Some(result.provenance.capability_cell.clone()),
            title: "No automatic K selection".into(),
            reason: "Candidate criteria and diagnostics are reported for analyst inspection; QuickPLS does not silently select the number of classes or segments.".into(),
        }],
        default_section_id: "heterogeneity_diagnostics".into(),
        default_table_id: "heterogeneity_candidates".into(),
    })
}

fn segmentation_share_chart(
    result: &PlsHeterogeneityAnalysisV2,
) -> Result<Option<CanonicalResultChart>, MultiModCanonicalResultErrorV1> {
    let mut series = Vec::new();
    for candidate in &result.candidates {
        if candidate.class_or_segment_shares.is_empty() {
            continue;
        }
        let algorithm = heterogeneity_candidate_method_name(&candidate.method)?;
        series.push(CanonicalChartSeries {
            id: stable_id("candidate", &(algorithm.as_str(), candidate.k)),
            label: format!("{algorithm}, K={}", candidate.k),
            group: Some(algorithm),
            points: candidate
                .class_or_segment_shares
                .iter()
                .enumerate()
                .map(|(index, share)| CanonicalChartPoint {
                    x: CanonicalChartX::Number((index + 1) as f64),
                    y: *share,
                    lower: None,
                    upper: None,
                    label: Some(format!("class or segment {}", index + 1)),
                })
                .collect(),
        });
    }
    if series.is_empty() {
        Ok(None)
    } else {
        Ok(Some(CanonicalResultChart {
            id: "heterogeneity_segment_shares".into(),
            title: "Candidate class or segment shares".into(),
            description: "Estimated aligned class or segment proportions by algorithm and candidate K; this chart does not select K.".into(),
            kind: CanonicalChartKind::Bar,
            series,
            source_table_id: Some("heterogeneity_candidate_shares".into()),
            display: CanonicalChartDisplayOptions {
                palette: Some("quickpls_accessible".into()),
                show_legend: Some(true),
                show_values: Some(true),
                x_axis_label: Some("Aligned class or segment".into()),
                y_axis_label: Some("Estimated share".into()),
            },
        }))
    }
}

fn project_conditional_process(
    result: &GeneralSemConditionalProcessResultV2,
    capabilities: &[CapabilityCellReferenceV2],
) -> Result<FamilyProjection, MultiModCanonicalResultErrorV1> {
    let mut target_rows = Vec::with_capacity(result.targets.len());
    let mut probe_rows = Vec::new();
    let mut derivative_rows = Vec::new();
    for target in &result.targets {
        let mut cells = vec![
            text(&target.target_id),
            text(enum_name(&target.kind)?),
            text(&target.path_id),
            optional_text(
                target.group_id.as_deref(),
                CanonicalMissingReason::NotApplicable,
            ),
            number(target.estimate),
            optional_number(target.p_value, CanonicalMissingReason::NotEstimated),
        ];
        cells.extend(interval_cells(target.interval.as_ref())?);
        cells.push(number(f64::from(target.usable_replicates)));
        target_rows.push(CanonicalResultRow {
            id: stable_id("conditional_target", &target.target_id),
            cells,
        });
        for (moderator_id, value) in &target.probe_values {
            probe_rows.push(CanonicalResultRow {
                id: stable_id(
                    "target_probe",
                    &(target.target_id.as_str(), moderator_id.as_str()),
                ),
                cells: vec![text(&target.target_id), text(moderator_id), number(*value)],
            });
        }
        for (position, variable_id) in target.derivative_variables.iter().enumerate() {
            derivative_rows.push(CanonicalResultRow {
                id: stable_id(
                    "target_derivative",
                    &(target.target_id.as_str(), position, variable_id.as_str()),
                ),
                cells: vec![
                    text(&target.target_id),
                    number(position as f64),
                    text(variable_id),
                ],
            });
        }
    }
    let targets = table(
        "conditional_process_targets",
        "Conditional-process targets",
        "Every explicitly selected indirect, total, derivative, probe, or group-contrast estimand on one shared resample ledger.",
        vec![
            text_column(
                "target_id",
                "Target",
                "Stable inferential target identity.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "target_kind",
                "Target kind",
                "Typed conditional-process estimand.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "path_id",
                "Path",
                "Explicitly selected indirect-path identity.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "group_id",
                "Group",
                "Selected group for grouped profiles, otherwise not applicable.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "estimate",
                "Estimate",
                "Conditional effect, derivative, index, or contrast estimate.",
                CanonicalColumnRole::Estimate,
            ),
            number_column(
                "p_value",
                "p value",
                "Alternative-specific probability when estimated.",
                CanonicalColumnRole::Uncertainty,
            ),
        ]
        .into_iter()
        .chain(interval_columns())
        .chain([number_column(
            "usable_replicates",
            "Usable replicates",
            "Usable rows from the shared target ledger.",
            CanonicalColumnRole::Diagnostic,
        )])
        .collect(),
        target_rows,
        vec![
            "one_sided_interval_bounds",
            "conditional_process_interpretation",
        ],
        capabilities,
    );
    let probes = table(
        "conditional_process_probe_values",
        "Frozen probe values",
        "Long-form original-sample probe anchors by target and moderator; absent rows mean that target is not evaluated at a finite probe tuple.",
        vec![
            text_column(
                "target_id",
                "Target",
                "Inferential target identity.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "moderator_id",
                "Moderator",
                "Moderator identity.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "value",
                "Probe value",
                "Frozen standardized or receipted raw-unit value.",
                CanonicalColumnRole::Provenance,
            ),
        ],
        probe_rows,
        Vec::new(),
        capabilities,
    );
    let derivatives = table(
        "conditional_process_derivative_variables",
        "Derivative variables",
        "Ordered derivative-variable identity for local first, second, and cross derivatives.",
        vec![
            text_column(
                "target_id",
                "Target",
                "Derivative target identity.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "position",
                "Position",
                "Zero-based derivative order position.",
                CanonicalColumnRole::Provenance,
            ),
            text_column(
                "variable_id",
                "Variable",
                "Moderator differentiated with respect to.",
                CanonicalColumnRole::Label,
            ),
        ],
        derivative_rows,
        Vec::new(),
        capabilities,
    );
    let warnings = table(
        "conditional_process_warnings",
        "Conditional-process warnings",
        "Complete result-level warning inventory.",
        vec![
            number_column(
                "position",
                "Position",
                "Stable warning order.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "warning",
                "Warning",
                "Exact warning text.",
                CanonicalColumnRole::Diagnostic,
            ),
        ],
        result
            .warnings
            .iter()
            .enumerate()
            .map(|(index, warning)| CanonicalResultRow {
                id: stable_id("conditional_warning", &(index, warning.as_str())),
                cells: vec![number(index as f64), text(warning)],
            })
            .collect(),
        Vec::new(),
        capabilities,
    );

    let (ledger_tables, ledger_ids) =
        build_ledger_tables(vec![&result.replicate_ledger], capabilities)?;
    let mut tables = vec![targets, probes, derivatives, warnings];
    tables.extend(ledger_tables);
    let interval_chart = interval_chart(
        "conditional_process_intervals",
        "Conditional-process estimates and confidence bounds",
        "Explicitly selected conditional-process targets with only the confidence bounds defined by their declared alternative.",
        "conditional_process_targets",
        result
            .targets
            .iter()
            .filter_map(|target| {
                target.interval.as_ref().map(|interval| {
                    (
                        format!("{}: {}", target.path_id, target.target_id),
                        target.estimate,
                        interval.lower,
                        interval.upper,
                    )
                })
            })
            .collect(),
    );
    let mut charts = Vec::new();
    let mut chart_ids = Vec::new();
    if let Some(chart) = interval_chart {
        chart_ids.push(chart.id.clone());
        charts.push(chart);
    }
    let mut table_ids = vec![
        "conditional_process_targets",
        "conditional_process_probe_values",
        "conditional_process_derivative_variables",
        "conditional_process_warnings",
    ];
    table_ids.extend(ledger_ids.iter().map(String::as_str));
    let notices = result
        .warnings
        .iter()
        .enumerate()
        .map(|(index, warning)| CanonicalResultNotice {
            id: stable_id("conditional_warning", &(index, warning.as_str())),
            code: "multimod_conditional_process_warning".into(),
            severity: qpls_core::CanonicalNoticeSeverity::Warning,
            message: warning.clone(),
            section_ids: vec!["conditional_process_results".into()],
            table_ids: vec!["conditional_process_warnings".into()],
        })
        .collect();
    Ok(FamilyProjection {
        sections: vec![section(
            "conditional_process_results",
            "Conditional effects and inference",
            "Explicit path targets, joint probes, derivative identities, warnings, and their single shared resampling ledger.",
            table_ids,
            chart_ids,
            capabilities,
        )],
        tables,
        charts,
        notices,
        exclusions: vec![CanonicalResultExclusion {
            id: "conditional_process_no_causal_claim".into(),
            capability_cell: Some(result.provenance.capability_cell.clone()),
            title: "No causal interpretation".into(),
            reason: "Ordinary PLS conditional indirect effects and moderated mediation are associational. They are not relabelled as causal, natural, cross-world, or counterfactual effects.".into(),
        }],
        default_section_id: "conditional_process_results".into(),
        default_table_id: "conditional_process_targets".into(),
    })
}

fn project_interventional_mediation(
    result: &InterventionalMediationResultV1,
    capabilities: &[CapabilityCellReferenceV2],
) -> Result<FamilyProjection, MultiModCanonicalResultErrorV1> {
    let assumptions = table(
        "interventional_identification_assumptions",
        "Identification assumptions",
        "Analyst-declared identification assumptions required for interpreting the g-computation estimates.",
        vec![
            number_column(
                "position",
                "Position",
                "Declared assumption order.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "assumption",
                "Assumption",
                "Exact analyst-declared assumption.",
                CanonicalColumnRole::Decision,
            ),
        ],
        result
            .identification_assumptions
            .iter()
            .enumerate()
            .map(|(index, assumption)| CanonicalResultRow {
                id: stable_id("assumption", &(index, assumption.as_str())),
                cells: vec![number(index as f64), text(assumption)],
            })
            .collect(),
        vec!["causal_interpretation"],
        capabilities,
    );
    let positivity = table(
        "interventional_positivity_diagnostics",
        "Positivity diagnostics",
        "Requested treatment/moderator values checked against observed support; unsupported requests fail interpretation visibly.",
        vec![
            text_column(
                "variable_id",
                "Variable",
                "Observed treatment or moderator identity.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "observed_minimum",
                "Observed minimum",
                "Minimum observed analysis value.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "observed_maximum",
                "Observed maximum",
                "Maximum observed analysis value.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "requested_value",
                "Requested value",
                "Value used by the interventional contrast or probe.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "support_count",
                "Support count",
                "Rows supporting the requested intervention value under the declared screen.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "minimum_required_count",
                "Minimum required",
                "Predeclared minimum usable support count.",
                CanonicalColumnRole::Diagnostic,
            ),
            text_column(
                "support_rule",
                "Support rule",
                "Predeclared positivity rule used for this diagnostic.",
                CanonicalColumnRole::Provenance,
            ),
            boolean_column(
                "supported",
                "Supported",
                "Whether the requested value lies inside observed support.",
                CanonicalColumnRole::Decision,
            ),
        ],
        result
            .positivity
            .iter()
            .enumerate()
            .map(|(index, row)| CanonicalResultRow {
                id: stable_id(
                    "positivity",
                    &(
                        index,
                        row.variable_id.as_str(),
                        row.requested_value.to_bits(),
                    ),
                ),
                cells: vec![
                    text(&row.variable_id),
                    number(row.observed_minimum),
                    number(row.observed_maximum),
                    number(row.requested_value),
                    number(row.support_count as f64),
                    number(row.minimum_required_count as f64),
                    text(&row.support_rule),
                    boolean(row.supported),
                ],
            })
            .collect(),
        vec!["causal_interpretation"],
        capabilities,
    );
    let mut effect_rows = Vec::with_capacity(result.effects.len());
    for effect in &result.effects {
        let mut cells = vec![
            text(&effect.target_id),
            text(&effect.path_id),
            text(&effect.estimand),
            text(&result.interpretation_label),
            number(effect.estimate),
            optional_number(effect.p_value, CanonicalMissingReason::NotEstimated),
        ];
        cells.extend(interval_cells(effect.interval.as_ref())?);
        effect_rows.push(CanonicalResultRow {
            id: stable_id("interventional_effect", &effect.target_id),
            cells,
        });
    }
    let effects = table(
        "interventional_effects",
        "Interventional mediation effects",
        "Parametric g-computation effects for explicitly selected observed-variable paths, always carrying the required cautious interpretation label.",
        vec![
            text_column(
                "target_id",
                "Target",
                "Stable effect target identity.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "path_id",
                "Path",
                "Explicitly selected recursive path identity.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "estimand",
                "Estimand",
                "Declared interventional estimand identity.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "interpretation",
                "Interpretation",
                "Mandatory assumption-dependent output wording.",
                CanonicalColumnRole::Decision,
            ),
            number_column(
                "estimate",
                "Estimate",
                "Parametric g-computation point estimate.",
                CanonicalColumnRole::Estimate,
            ),
            number_column(
                "p_value",
                "p value",
                "Probability when estimated.",
                CanonicalColumnRole::Uncertainty,
            ),
        ]
        .into_iter()
        .chain(interval_columns())
        .collect(),
        effect_rows,
        vec!["one_sided_interval_bounds", "causal_interpretation"],
        capabilities,
    );

    let (ledger_tables, ledger_ids) =
        build_ledger_tables(vec![&result.replicate_ledger], capabilities)?;
    let mut tables = vec![assumptions, positivity, effects];
    tables.extend(ledger_tables);
    let effect_chart = interval_chart(
        "interventional_effect_intervals",
        "Interventional effect estimates and confidence bounds",
        "Assumption-dependent interventional estimates with only the confidence bounds defined by their declared alternative.",
        "interventional_effects",
        result
            .effects
            .iter()
            .filter_map(|effect| {
                effect.interval.as_ref().map(|interval| {
                    (
                        format!("{}: {}", effect.path_id, effect.target_id),
                        effect.estimate,
                        interval.lower,
                        interval.upper,
                    )
                })
            })
            .collect(),
    );
    let mut charts = Vec::new();
    let mut chart_ids = Vec::new();
    if let Some(chart) = effect_chart {
        chart_ids.push(chart.id.clone());
        charts.push(chart);
    }
    let mut table_ids = vec![
        "interventional_identification_assumptions",
        "interventional_positivity_diagnostics",
        "interventional_effects",
    ];
    table_ids.extend(ledger_ids.iter().map(String::as_str));
    let unsupported = result
        .positivity
        .iter()
        .filter(|row| !row.supported)
        .count();
    let notices = if unsupported == 0 {
        Vec::new()
    } else {
        vec![CanonicalResultNotice {
            id: "interventional_positivity_failure".into(),
            code: "multimod_interventional_positivity_unsupported".into(),
            severity: qpls_core::CanonicalNoticeSeverity::Error,
            message: format!(
                "{unsupported} requested value(s) fall outside observed support; causal interpretation must fail closed."
            ),
            section_ids: vec!["interventional_results".into()],
            table_ids: vec!["interventional_positivity_diagnostics".into()],
        }]
    };
    Ok(FamilyProjection {
        sections: vec![section(
            "interventional_results",
            "Identification, positivity, and effects",
            "Assumptions, observed-support diagnostics, explicit interventional effects, and complete resampling evidence.",
            table_ids,
            chart_ids,
            capabilities,
        )],
        tables,
        charts,
        notices,
        exclusions: vec![CanonicalResultExclusion {
            id: "interventional_natural_effects_excluded".into(),
            capability_cell: Some(result.provenance.capability_cell.clone()),
            title: "Natural and cross-world effects are excluded".into(),
            reason: "This first observed-data profile reports assumption-dependent interventional estimates only; it does not identify natural effects, cross-world effects, recanting-witness settings, or exposure-induced mediator-outcome confounding.".into(),
        }],
        default_section_id: "interventional_results".into(),
        default_table_id: "interventional_effects".into(),
    })
}

fn build_ledger_tables(
    ledgers: Vec<&MultimodReplicateLedgerSummaryV1>,
    capabilities: &[CapabilityCellReferenceV2],
) -> Result<(Vec<CanonicalResultTable>, Vec<String>), MultiModCanonicalResultErrorV1> {
    let mut summary_rows = Vec::with_capacity(ledgers.len());
    let mut count_rows = Vec::new();
    let mut failure_rows = Vec::new();
    for (ledger_index, ledger) in ledgers.iter().enumerate() {
        let ledger_id = format!("ledger_{}", &ledger.ledger_sha256[..16]);
        summary_rows.push(CanonicalResultRow {
            id: stable_id("ledger", &(ledger_index, ledger.ledger_sha256.as_str())),
            cells: vec![
                text(&ledger_id),
                number(f64::from(ledger.requested)),
                number(f64::from(ledger.usable)),
                number(f64::from(ledger.minimum_required)),
                number(ledger.usable_fraction),
                boolean(ledger.complete),
                text(&ledger.ledger_sha256),
                number(usize_number(ledger.failure_counts.len())?),
                number(usize_number(ledger.failures.len())?),
            ],
        });
        for (stable_code, count) in &ledger.failure_counts {
            count_rows.push(CanonicalResultRow {
                id: stable_id("failure_count", &(ledger_index, stable_code.as_str())),
                cells: vec![
                    text(&ledger_id),
                    text(stable_code),
                    number(f64::from(*count)),
                ],
            });
        }
        for (failure_index, failure) in ledger.failures.iter().enumerate() {
            failure_rows.push(CanonicalResultRow {
                id: stable_id(
                    "replicate_failure",
                    &(
                        ledger_index,
                        failure_index,
                        failure.replicate_index,
                        failure.stable_code.as_str(),
                    ),
                ),
                cells: vec![
                    text(&ledger_id),
                    number(f64::from(failure.replicate_index)),
                    text(enum_name(&failure.kind)?),
                    text(&failure.stable_code),
                    text(&failure.detail),
                ],
            });
        }
    }
    let summary = table(
        "multimod_resampling_ledgers",
        "Resampling ledgers",
        "Complete requested/usable/minimum counts and immutable ledger digests; failed draws are never replaced silently.",
        vec![
            text_column(
                "ledger_id",
                "Ledger ID",
                "Digest-derived ledger identity used by the related failure tables.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "requested",
                "Requested",
                "Requested draws.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "usable",
                "Usable",
                "Usable draws.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "minimum_required",
                "Minimum required",
                "Frozen minimum usable-draw gate.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "usable_fraction",
                "Usable fraction",
                "Usable divided by requested draws.",
                CanonicalColumnRole::Diagnostic,
            ),
            boolean_column(
                "complete",
                "Complete",
                "Whether the minimum usable-draw gate passed.",
                CanonicalColumnRole::Decision,
            ),
            text_column(
                "ledger_sha256",
                "Ledger SHA-256",
                "Immutable complete-ledger digest.",
                CanonicalColumnRole::Provenance,
            ),
            number_column(
                "failure_code_count",
                "Failure codes",
                "Number of distinct failure codes.",
                CanonicalColumnRole::Diagnostic,
            ),
            number_column(
                "retained_failure_rows",
                "Retained failures",
                "Number of explicit per-replicate failure rows retained in the result.",
                CanonicalColumnRole::Diagnostic,
            ),
        ],
        summary_rows,
        Vec::new(),
        capabilities,
    );
    let counts = table(
        "multimod_resampling_failure_counts",
        "Resampling failure counts",
        "Complete aggregate failure inventory by ledger and stable code.",
        vec![
            text_column(
                "ledger_id",
                "Ledger ID",
                "Related resampling ledger.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "stable_code",
                "Stable code",
                "Machine-readable failure code.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "count",
                "Count",
                "Failed replicate count.",
                CanonicalColumnRole::Diagnostic,
            ),
        ],
        count_rows,
        Vec::new(),
        capabilities,
    );
    let failures = table(
        "multimod_resampling_failures",
        "Retained replicate failures",
        "Per-replicate typed failures retained by the source result. Aggregate counts remain authoritative when a result intentionally truncates detail.",
        vec![
            text_column(
                "ledger_id",
                "Ledger ID",
                "Related resampling ledger.",
                CanonicalColumnRole::Label,
            ),
            number_column(
                "replicate_index",
                "Replicate",
                "Zero-based deterministic replicate index.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "failure_kind",
                "Failure kind",
                "Typed failure family.",
                CanonicalColumnRole::Decision,
            ),
            text_column(
                "stable_code",
                "Stable code",
                "Machine-readable failure code.",
                CanonicalColumnRole::Label,
            ),
            text_column(
                "detail",
                "Detail",
                "Exact retained diagnostic detail.",
                CanonicalColumnRole::Diagnostic,
            ),
        ],
        failure_rows,
        Vec::new(),
        capabilities,
    );
    Ok((
        vec![summary, counts, failures],
        vec![
            "multimod_resampling_ledgers".into(),
            "multimod_resampling_failure_counts".into(),
            "multimod_resampling_failures".into(),
        ],
    ))
}

fn qualification_notices(
    provenance: &MultimodProvenanceV1,
    section_id: &str,
) -> Vec<CanonicalResultNotice> {
    let (severity, code, message) = match provenance.qualification {
        MultimodQualificationStateV1::UnqualifiedLabs => (
            qpls_core::CanonicalNoticeSeverity::Warning,
            "multimod_unqualified_labs",
            "This result is a Labs/unqualified result. Canonical schema validation does not make it release-qualified or Standard-eligible.",
        ),
        MultimodQualificationStateV1::ReleaseQualifiedCandidate => (
            qpls_core::CanonicalNoticeSeverity::Information,
            "multimod_release_qualified_candidate",
            "This result is a release-qualification candidate only. Exact-commit live manifests and every dependent evidence gate remain authoritative.",
        ),
        MultimodQualificationStateV1::FailedClosed => (
            qpls_core::CanonicalNoticeSeverity::Error,
            "multimod_failed_closed",
            "This result failed closed and must not be interpreted as completed scientific inference.",
        ),
    };
    vec![CanonicalResultNotice {
        id: "multimod_qualification_boundary".into(),
        code: code.into(),
        severity,
        message: message.into(),
        section_ids: vec!["multimod_provenance".into(), section_id.into()],
        table_ids: vec!["multimod_run_provenance".into()],
    }]
}

fn common_footnotes() -> Vec<CanonicalResultFootnote> {
    vec![
        CanonicalResultFootnote {
            id: "causal_interpretation".into(),
            text: "Causal-module estimates remain assumption-dependent interventional estimates; the output does not establish causality.".into(),
            reference: None,
        },
        CanonicalResultFootnote {
            id: "conditional_process_interpretation".into(),
            text: "A scalar index of moderated mediation is reported only for an indirect effect affine in one moderator; other models retain local derivatives and finite contrasts under their exact names.".into(),
            reference: None,
        },
        CanonicalResultFootnote {
            id: "directional_probability".into(),
            text: "A Henseler PLS-MGA directional probability is not an ordinary two-sided p value.".into(),
            reference: None,
        },
        CanonicalResultFootnote {
            id: "heterogeneity_common_metric".into(),
            text: "Segment-to-segment interaction and effect inference requires pooled common-metric refitting plus configural and pairwise compositional-invariance evidence; otherwise inference is withheld.".into(),
            reference: None,
        },
        CanonicalResultFootnote {
            id: "heterogeneity_exploratory".into(),
            text: "Latent classes and PLS-POS segments are exploratory heterogeneity summaries. Candidate criteria do not automatically select K.".into(),
            reference: None,
        },
        CanonicalResultFootnote {
            id: "micom_interpretation".into(),
            text: "Partial measurement invariance requires the reviewed configural checklist and pairwise compositional invariance before affected structural differences are interpreted.".into(),
            reference: None,
        },
        CanonicalResultFootnote {
            id: "multimod_labs_boundary".into(),
            text: "MultiMod qualification state is retained explicitly. Generic canonical validation and export are not release-qualification evidence.".into(),
            reference: None,
        },
        CanonicalResultFootnote {
            id: "one_sided_interval_bounds".into(),
            text: "Two-sided intervals have lower and upper bounds. A less alternative has only an upper bound; a greater alternative has only a lower bound. The other endpoint is not applicable, not zero.".into(),
            reference: None,
        },
    ]
}

fn interval_chart(
    id: &str,
    title: &str,
    description: &str,
    source_table_id: &str,
    values: Vec<(String, f64, Option<f64>, Option<f64>)>,
) -> Option<CanonicalResultChart> {
    if values.is_empty() {
        return None;
    }
    Some(CanonicalResultChart {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        kind: CanonicalChartKind::Interval,
        series: vec![CanonicalChartSeries {
            id: "estimates".into(),
            label: "Estimate and confidence bounds".into(),
            group: None,
            points: values
                .into_iter()
                .map(|(label, estimate, lower, upper)| CanonicalChartPoint {
                    x: CanonicalChartX::Text(label.clone()),
                    y: estimate,
                    lower,
                    upper,
                    label: Some(label),
                })
                .collect(),
        }],
        source_table_id: Some(source_table_id.into()),
        display: CanonicalChartDisplayOptions {
            palette: Some("quickpls_accessible".into()),
            show_legend: Some(false),
            show_values: Some(true),
            x_axis_label: Some("Scientific target".into()),
            y_axis_label: Some("Estimate".into()),
        },
    })
}

fn interval_columns() -> impl Iterator<Item = CanonicalResultColumn> {
    [
        number_column(
            "confidence_level",
            "Confidence",
            "Confidence level for the declared interval family.",
            CanonicalColumnRole::Provenance,
        ),
        text_column(
            "interval_family",
            "Interval family",
            "Percentile, BC, BCa, studentized, or other exact family identity.",
            CanonicalColumnRole::Provenance,
        ),
        text_column(
            "alternative",
            "Alternative",
            "Two-sided, less, or greater inference alternative.",
            CanonicalColumnRole::Provenance,
        ),
        number_column(
            "lower",
            "Lower bound",
            "Lower confidence bound; not applicable for a less alternative.",
            CanonicalColumnRole::Uncertainty,
        ),
        number_column(
            "upper",
            "Upper bound",
            "Upper confidence bound; not applicable for a greater alternative.",
            CanonicalColumnRole::Uncertainty,
        ),
    ]
    .into_iter()
}

fn interval_cells(
    interval: Option<&MultimodIntervalV1>,
) -> Result<Vec<CanonicalResultCell>, MultiModCanonicalResultErrorV1> {
    Ok(match interval {
        Some(interval) => vec![
            number(interval.confidence_level),
            text(&interval.family),
            text(enum_name(&interval.alternative)?),
            optional_number(interval.lower, CanonicalMissingReason::NotApplicable),
            optional_number(interval.upper, CanonicalMissingReason::NotApplicable),
        ],
        None => vec![
            missing(CanonicalMissingReason::NotEstimated),
            missing(CanonicalMissingReason::NotEstimated),
            missing(CanonicalMissingReason::NotEstimated),
            missing(CanonicalMissingReason::NotEstimated),
            missing(CanonicalMissingReason::NotEstimated),
        ],
    })
}

fn parameter_columns(first_label: &str) -> Vec<CanonicalResultColumn> {
    vec![
        text_column(
            "group_id",
            first_label,
            "Group identity.",
            CanonicalColumnRole::Label,
        ),
        text_column(
            "target_id",
            "Target",
            "Scientific parameter target.",
            CanonicalColumnRole::Label,
        ),
        text_column(
            "target_kind",
            "Target kind",
            "Typed parameter family.",
            CanonicalColumnRole::Label,
        ),
        number_column(
            "estimate",
            "Estimate",
            "Point estimate.",
            CanonicalColumnRole::Estimate,
        ),
        number_column(
            "standard_error",
            "Standard error",
            "Standard error when estimated.",
            CanonicalColumnRole::Uncertainty,
        ),
        number_column(
            "p_value",
            "p value",
            "Probability when estimated.",
            CanonicalColumnRole::Uncertainty,
        ),
    ]
    .into_iter()
    .chain(interval_columns())
    .collect()
}

fn section(
    id: &str,
    title: &str,
    description: &str,
    table_ids: Vec<&str>,
    chart_ids: Vec<String>,
    capabilities: &[CapabilityCellReferenceV2],
) -> CanonicalResultSection {
    CanonicalResultSection {
        id: id.into(),
        title: title.into(),
        description: Some(description.into()),
        table_ids: table_ids.into_iter().map(str::to_owned).collect(),
        chart_ids,
        capability_cells: Some(capabilities.to_vec()),
    }
}

fn table(
    id: &str,
    title: &str,
    description: &str,
    columns: Vec<CanonicalResultColumn>,
    rows: Vec<CanonicalResultRow>,
    footnote_ids: Vec<&str>,
    capabilities: &[CapabilityCellReferenceV2],
) -> CanonicalResultTable {
    CanonicalResultTable {
        id: id.into(),
        title: title.into(),
        description: Some(description.into()),
        columns,
        rows,
        footnote_ids: footnote_ids.into_iter().map(str::to_owned).collect(),
        capability_cells: Some(capabilities.to_vec()),
    }
}

fn text_column(
    id: &str,
    label: &str,
    description: &str,
    role: CanonicalColumnRole,
) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Text,
        description: description.into(),
        role: Some(role),
        unit: None,
        default_precision: None,
    }
}

fn number_column(
    id: &str,
    label: &str,
    description: &str,
    role: CanonicalColumnRole,
) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Number,
        description: description.into(),
        role: Some(role),
        unit: None,
        default_precision: Some(6),
    }
}

fn boolean_column(
    id: &str,
    label: &str,
    description: &str,
    role: CanonicalColumnRole,
) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Boolean,
        description: description.into(),
        role: Some(role),
        unit: None,
        default_precision: None,
    }
}

fn text(value: impl Into<String>) -> CanonicalResultCell {
    CanonicalResultCell::Text {
        value: value.into(),
    }
}

fn number(value: f64) -> CanonicalResultCell {
    CanonicalResultCell::Number {
        value,
        display: None,
    }
}

fn boolean(value: bool) -> CanonicalResultCell {
    CanonicalResultCell::Boolean { value }
}

fn missing(reason: CanonicalMissingReason) -> CanonicalResultCell {
    CanonicalResultCell::Missing {
        reason,
        display: None,
    }
}

fn optional_number(value: Option<f64>, reason: CanonicalMissingReason) -> CanonicalResultCell {
    value.map_or_else(|| missing(reason), number)
}

fn optional_text(value: Option<&str>, reason: CanonicalMissingReason) -> CanonicalResultCell {
    value.map_or_else(|| missing(reason), text)
}

fn enum_name<T: Serialize>(value: &T) -> Result<String, MultiModCanonicalResultErrorV1> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(value) => Ok(value),
        _ => Err(MultiModCanonicalResultErrorV1::Canonical(
            "typed identity did not serialize as a string".into(),
        )),
    }
}

fn heterogeneity_candidate_method_name(
    method: &HeterogeneityCandidateMethodV2,
) -> Result<String, MultiModCanonicalResultErrorV1> {
    match method {
        HeterogeneityCandidateMethodV2::PooledBaselineV1 => Ok("pooled_baseline_v1".into()),
        HeterogeneityCandidateMethodV2::Segmentation { algorithm } => enum_name(algorithm),
    }
}

fn stable_id<T: Serialize>(prefix: &str, identity: &T) -> String {
    let digest = sha256_serialized(identity);
    format!("{prefix}_{}", &digest[..20])
}

fn u64_number(value: u64) -> Result<f64, MultiModCanonicalResultErrorV1> {
    if value > MAX_SAFE_INTEGER {
        Err(MultiModCanonicalResultErrorV1::Canonical(
            "an integer table value exceeds CanonicalResultCell's exact safe-integer range".into(),
        ))
    } else {
        Ok(value as f64)
    }
}

fn usize_number(value: usize) -> Result<f64, MultiModCanonicalResultErrorV1> {
    u64_number(u64::try_from(value).map_err(|_| {
        MultiModCanonicalResultErrorV1::Canonical(
            "a collection length cannot be represented by the canonical table contract".into(),
        )
    })?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{
        CausalPositivityDiagnosticV1, ConditionalProcessTargetKindV2,
        GENERAL_SEM_CONDITIONAL_PROCESS_RESULT_V2_SCHEMA_VERSION,
        GeneralSemConditionalProcessResultV2, HeterogeneityAlgorithmV2,
        HeterogeneityCandidateStateV2, HeterogeneityCandidateV2, HeterogeneityInteractionProfileV2,
        INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION, InferenceAlternativeV1,
        InterventionalEffectResultV1, InterventionalMediationResultV1, MgaGroupEligibilityV1,
        MgaModelProfileV1, MgaPairwiseComparisonV1, MultiplicityAdjustmentV1,
        PLS_HETEROGENEITY_ANALYSIS_V2_SCHEMA_VERSION, PLS_MULTIGROUP_ANALYSIS_V1_SCHEMA_VERSION,
        PlsHeterogeneityAnalysisV2, PlsMultigroupAnalysisV1,
    };
    use std::collections::BTreeMap;

    fn provenance(method: &str) -> MultimodProvenanceV1 {
        MultimodProvenanceV1 {
            method_version: method.into(),
            recipe_id: "00000000-0000-0000-0000-000000000101".into(),
            recipe_analytical_sha256: "a".repeat(64),
            config_sha256: "b".repeat(64),
            model_id: "00000000-0000-0000-0000-000000000203".into(),
            model_scientific_sha256: "c".repeat(64),
            dataset_id: "00000000-0000-0000-0000-000000000204".into(),
            dataset_fingerprint: "d".repeat(64),
            engine_version: "2.56.0".into(),
            seed: 42,
            capability_cell: CapabilityCellReferenceV2 {
                registry_schema_version: 2,
                capability_id: "quickpls.multimod".into(),
                cell_id: "qpls.multimod.labs.v1".into(),
                capability_version: "multimod_labs_v1".into(),
            },
            qualification: MultimodQualificationStateV1::UnqualifiedLabs,
            candidate_qualification_receipt: None,
        }
    }

    fn context() -> MultiModCanonicalRunContextV1 {
        MultiModCanonicalRunContextV1 {
            run_id: "00000000-0000-0000-0000-000000000201".into(),
            project_id: "00000000-0000-0000-0000-000000000202".into(),
            recipe_id: "00000000-0000-0000-0000-000000000101".into(),
            recipe_analytical_sha256: "a".repeat(64),
            model_id: "00000000-0000-0000-0000-000000000203".into(),
            model_scientific_sha256: "c".repeat(64),
            dataset_id: "00000000-0000-0000-0000-000000000204".into(),
            dataset_fingerprint: "d".repeat(64),
            engine_version: "2.56.0".into(),
            workers: 2,
            started_at: "2026-08-24T10:00:00+05:30".into(),
            completed_at: "2026-08-24T10:01:00+05:30".into(),
        }
    }

    fn ledger() -> MultimodReplicateLedgerSummaryV1 {
        MultimodReplicateLedgerSummaryV1 {
            requested: 2,
            usable: 2,
            minimum_required: 2,
            usable_fraction: 1.0,
            complete: true,
            ledger_sha256: "e".repeat(64),
            failure_counts: BTreeMap::new(),
            failures: Vec::new(),
        }
    }

    fn greater_interval() -> MultimodIntervalV1 {
        MultimodIntervalV1 {
            confidence_level: 0.95,
            lower: Some(0.1),
            upper: None,
            family: "percentile_type7".into(),
            alternative: InferenceAlternativeV1::Greater,
        }
    }

    fn assert_canonical(document: &CanonicalResultDocumentV2) {
        let validation = validate_canonical_result_document_v2(document);
        assert!(validation.passed, "{}", validation.errors.join("; "));
        assert!(
            document
                .tables
                .iter()
                .all(|table| table.capability_cells.is_some())
        );
        assert!(
            document
                .sections
                .iter()
                .all(|section| section.capability_cells.is_some())
        );
        assert!(
            document
                .notices
                .iter()
                .any(|notice| notice.code == "multimod_unqualified_labs")
        );
        assert!(
            document
                .tables
                .iter()
                .any(|table| table.id == "multimod_sidecar_inventory")
        );
    }

    #[test]
    fn mga_projection_preserves_directional_probability_and_one_sided_bound() {
        let result = MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(PlsMultigroupAnalysisV1 {
            schema_version: PLS_MULTIGROUP_ANALYSIS_V1_SCHEMA_VERSION,
            provenance: provenance("qpls.mga.multigroup.v1"),
            profile: MgaModelProfileV1::GeneralSemPls,
            group_eligibility: vec![
                MgaGroupEligibilityV1 {
                    group_id: "a".into(),
                    label: "A".into(),
                    complete_cases: 20,
                    selected_rows: 20,
                    eligible: true,
                    warnings: Vec::new(),
                    blockers: Vec::new(),
                },
                MgaGroupEligibilityV1 {
                    group_id: "b".into(),
                    label: "B".into(),
                    complete_cases: 20,
                    selected_rows: 20,
                    eligible: true,
                    warnings: Vec::new(),
                    blockers: Vec::new(),
                },
            ],
            group_parameters: Vec::new(),
            micom_pairs: Vec::new(),
            omnibus: Vec::new(),
            pairwise: vec![MgaPairwiseComparisonV1 {
                procedure: "henseler_pls_mga".into(),
                left_group_id: "a".into(),
                right_group_id: "b".into(),
                target_id: "x_to_y".into(),
                difference_left_minus_right: 0.25,
                raw_p_value: None,
                adjusted_p_value: None,
                directional_probability: Some(0.97),
                interval: Some(greater_interval()),
                measurement_comparability_satisfied: true,
                interpretation_blocked: false,
            }],
            multiplicity: MultiplicityAdjustmentV1::Holm,
            replicate_ledgers: vec![ledger()],
            excluded_rows: Vec::new(),
            sidecars: Vec::new(),
        });
        let document = build_multimod_canonical_result_v2(&context(), &result).unwrap();
        assert_canonical(&document);
        let table = document
            .tables
            .iter()
            .find(|table| table.id == "mga_pairwise_comparisons")
            .unwrap();
        let lower = table
            .columns
            .iter()
            .position(|column| column.id == "lower")
            .unwrap();
        let upper = table
            .columns
            .iter()
            .position(|column| column.id == "upper")
            .unwrap();
        assert!(matches!(
            table.rows[0].cells[lower],
            CanonicalResultCell::Number { value: 0.1, .. }
        ));
        assert!(matches!(
            table.rows[0].cells[upper],
            CanonicalResultCell::Missing {
                reason: CanonicalMissingReason::NotApplicable,
                ..
            }
        ));
    }

    #[test]
    fn heterogeneity_projection_retains_candidates_criteria_and_share_chart() {
        let mut criteria = BTreeMap::new();
        criteria.insert("bic".into(), 123.0);
        let result =
            MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(PlsHeterogeneityAnalysisV2 {
                schema_version: PLS_HETEROGENEITY_ANALYSIS_V2_SCHEMA_VERSION,
                provenance: provenance("qpls.fimix_pls.v2"),
                profile: HeterogeneityInteractionProfileV2::P0Structural,
                candidates: vec![
                    HeterogeneityCandidateV2 {
                        method: HeterogeneityCandidateMethodV2::PooledBaselineV1,
                        k: 1,
                        state: HeterogeneityCandidateStateV2::Eligible,
                        converged_starts: 0,
                        stable_starts: 0,
                        log_likelihood: None,
                        objective: None,
                        criteria: BTreeMap::from([("observation_count".into(), 100.0)]),
                        class_or_segment_shares: Vec::new(),
                        pooled_parameters: vec![qpls_core::MultimodParameterEstimateV1 {
                            target_id: "path:x:y".into(),
                            target_kind: "pooled_structural_path".into(),
                            estimate: 0.2,
                            standard_error: None,
                            p_value: None,
                            interval: None,
                        }],
                        blockers: Vec::new(),
                    },
                    HeterogeneityCandidateV2 {
                        method: HeterogeneityCandidateMethodV2::Segmentation {
                            algorithm: HeterogeneityAlgorithmV2::FimixPlsV2,
                        },
                        k: 2,
                        state: HeterogeneityCandidateStateV2::ConvergedStable,
                        converged_starts: 30,
                        stable_starts: 3,
                        log_likelihood: Some(-55.0),
                        objective: None,
                        criteria,
                        class_or_segment_shares: vec![0.4, 0.6],
                        pooled_parameters: Vec::new(),
                        blockers: Vec::new(),
                    },
                ],
                discovery_result_identity_sha256: "e".repeat(64),
                inference_lock: None,
                locked_algorithm: None,
                locked_k: None,
                parameters: Vec::new(),
                contrasts: Vec::new(),
                bootstrap_ledger: None,
                sidecars: Vec::new(),
                descriptive_only: false,
            });
        let document = build_multimod_canonical_result_v2(&context(), &result).unwrap();
        assert_canonical(&document);
        assert!(
            document
                .charts
                .iter()
                .any(|chart| chart.id == "heterogeneity_segment_shares")
        );
        assert_eq!(
            document
                .tables
                .iter()
                .find(|table| table.id == "heterogeneity_candidate_criteria")
                .unwrap()
                .rows
                .len(),
            2
        );
    }

    #[test]
    fn conditional_projection_uses_one_shared_ledger_and_explicit_probe_table() {
        let result = MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(
            GeneralSemConditionalProcessResultV2 {
                schema_version: GENERAL_SEM_CONDITIONAL_PROCESS_RESULT_V2_SCHEMA_VERSION,
                provenance: provenance("qpls.conditional_process.v2"),
                profile_id: "multi_two_way".into(),
                targets: vec![qpls_core::ConditionalProcessTargetResultV2 {
                    target_id: "path_1_at_z_1".into(),
                    kind: ConditionalProcessTargetKindV2::ConditionalSpecificIndirect,
                    path_id: "path_1".into(),
                    group_id: None,
                    probe_values: BTreeMap::from([("z".into(), 1.0)]),
                    derivative_variables: Vec::new(),
                    estimate: 0.3,
                    p_value: Some(0.02),
                    interval: Some(greater_interval()),
                    usable_replicates: 2,
                }],
                replicate_ledger: ledger(),
                sidecars: Vec::new(),
                warnings: vec!["fixture warning".into()],
            },
        );
        let document = build_multimod_canonical_result_v2(&context(), &result).unwrap();
        assert_canonical(&document);
        assert_eq!(
            document
                .tables
                .iter()
                .find(|table| table.id == "conditional_process_probe_values")
                .unwrap()
                .rows
                .len(),
            1
        );
        assert_eq!(
            document
                .tables
                .iter()
                .find(|table| table.id == "multimod_resampling_ledgers")
                .unwrap()
                .rows
                .len(),
            1
        );
    }

    #[test]
    fn causal_projection_keeps_cautious_label_assumptions_and_positivity() {
        let result = MultiModAnalysisResultV1::InterventionalMediationResultV1(
            InterventionalMediationResultV1 {
                schema_version: INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION,
                provenance: provenance("qpls.interventional_mediation.v1"),
                interpretation_label: "assumption-dependent interventional estimate".into(),
                identification_assumptions: vec![
                    "No unmeasured treatment-outcome confounding".into(),
                ],
                positivity: vec![CausalPositivityDiagnosticV1 {
                    variable_id: "treatment".into(),
                    observed_minimum: 0.0,
                    observed_maximum: 1.0,
                    requested_value: 1.0,
                    support_count: 50,
                    minimum_required_count: 10,
                    support_rule: "binary_arm_count".into(),
                    supported: true,
                }],
                effects: vec![InterventionalEffectResultV1 {
                    target_id: "iie_path_1".into(),
                    path_id: "path_1".into(),
                    estimand: "interventional_indirect_effect".into(),
                    estimate: 0.2,
                    p_value: Some(0.03),
                    interval: Some(greater_interval()),
                }],
                replicate_ledger: ledger(),
                sidecars: vec![MultimodResultSidecarDescriptorV1 {
                    schema_version: 1,
                    entry_name: "results/00000000-0000-0000-0000-000000000201/interventional-bootstrap-target-vectors.arrow".into(),
                    evidence_role: "interventional-bootstrap:target-vectors".into(),
                    arrow_schema_contract_id: format!(
                        "qpls.multimod.arrow.interventional-bootstrap:target-vectors.v1.{}",
                        "4".repeat(64)
                    ),
                    arrow_schema_contract_version: 1,
                    media_type: "application/vnd.apache.arrow.stream".into(),
                    compression: "zip_deflate".into(),
                    arrow_schema_sha256: "1".repeat(64),
                    row_count: 2,
                    column_count: 3,
                    uncompressed_bytes: 256,
                    sha256: "2".repeat(64),
                    identity_sha256: "3".repeat(64),
                    required_for_scientific_reopen: true,
                }],
            },
        );
        let document = build_multimod_canonical_result_v2(&context(), &result).unwrap();
        assert_canonical(&document);
        let effects = document
            .tables
            .iter()
            .find(|table| table.id == "interventional_effects")
            .unwrap();
        let interpretation = effects
            .columns
            .iter()
            .position(|column| column.id == "interpretation")
            .unwrap();
        assert!(
            matches!(&effects.rows[0].cells[interpretation], CanonicalResultCell::Text { value } if value == "assumption-dependent interventional estimate")
        );
        assert!(
            document
                .exclusions
                .iter()
                .any(|exclusion| exclusion.id == "interventional_natural_effects_excluded")
        );
        assert_eq!(
            document
                .tables
                .iter()
                .find(|table| table.id == "multimod_sidecar_inventory")
                .unwrap()
                .rows
                .len(),
            1
        );
    }

    #[test]
    fn every_scientific_context_identity_mismatch_fails_before_projection() {
        let result = MultiModAnalysisResultV1::InterventionalMediationResultV1(
            InterventionalMediationResultV1 {
                schema_version: INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION,
                provenance: provenance("qpls.interventional_mediation.v1"),
                interpretation_label: "assumption-dependent interventional estimate".into(),
                identification_assumptions: vec!["Temporal ordering declared".into()],
                positivity: Vec::new(),
                effects: vec![InterventionalEffectResultV1 {
                    target_id: "effect".into(),
                    path_id: "path".into(),
                    estimand: "iie".into(),
                    estimate: 0.1,
                    p_value: None,
                    interval: None,
                }],
                replicate_ledger: ledger(),
                sidecars: Vec::new(),
            },
        );
        let mut cases = Vec::new();
        let mut mismatched = context();
        mismatched.recipe_id = "different-recipe".into();
        cases.push(mismatched);
        let mut mismatched = context();
        mismatched.recipe_analytical_sha256 = "f".repeat(64);
        cases.push(mismatched);
        let mut mismatched = context();
        mismatched.model_id = "different-model".into();
        cases.push(mismatched);
        let mut mismatched = context();
        mismatched.model_scientific_sha256 = "f".repeat(64);
        cases.push(mismatched);
        let mut mismatched = context();
        mismatched.dataset_id = "different-dataset".into();
        cases.push(mismatched);
        let mut mismatched = context();
        mismatched.dataset_fingerprint = "f".repeat(64);
        cases.push(mismatched);
        let mut mismatched = context();
        mismatched.engine_version = "different-engine".into();
        cases.push(mismatched);

        for mismatched in cases {
            assert!(matches!(
                build_multimod_canonical_result_v2(&mismatched, &result),
                Err(MultiModCanonicalResultErrorV1::Context(_))
            ));
        }
    }
}
