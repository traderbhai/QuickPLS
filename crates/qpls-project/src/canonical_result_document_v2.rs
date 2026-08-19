use chrono::DateTime;
pub use qpls_core::{
    CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION, CanonicalAggregateEffectKindV1,
    CanonicalAggregateEffectResultV1, CanonicalCbsemFitResultV1,
    CanonicalConditionalEffectProbeResultV1, CanonicalConditionalEffectResultV1,
    CanonicalConditionalProbeValuesResultV1, CanonicalGeneralSemBootstrapIntervalV1,
    CanonicalGeneralSemEstimateV1, CanonicalGeneralSemFailedReplicateV1,
    CanonicalGeneralSemInferenceKindV1, CanonicalGeneralSemInferenceReceiptV1,
    CanonicalGeneralSemInferenceTailV1, CanonicalGeneralSemIntervalV1,
    CanonicalGeneralSemResultTraceV1, CanonicalGeneralSemResultsV1, CanonicalHocRelationEstimateV1,
    CanonicalHocStageKindV1, CanonicalHocStageResultV1, CanonicalIdentificationDiagnosticV1,
    CanonicalIdentificationScopeV1, CanonicalIdentificationStatusV1,
    CanonicalInteractionConstructionMethodV1, CanonicalInteractionEffectResultV1,
    CanonicalInteractionHierarchyPolicyV1, CanonicalInteractionPlotPointV1,
    CanonicalInteractionPlotResultV1, CanonicalInteractionPlotSeriesV1,
    CanonicalJointStageStructuralCoefficientResultV1, CanonicalSpecificIndirectEffectResultV1,
    CanonicalStructuralEstimateStageV1, CanonicalStructuralRelationRoleV1,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION: u32 = 2;
const JAVASCRIPT_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCellReferenceV2 {
    pub registry_schema_version: u32,
    pub capability_id: String,
    pub cell_id: String,
    pub capability_version: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalColumnTypeV2 {
    Number,
    Text,
    Boolean,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalMissingReasonV2 {
    NotApplicable,
    NotEstimated,
    Undefined,
    Withheld,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalResultCellV2 {
    Number {
        value: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display: Option<String>,
    },
    Text {
        value: String,
    },
    Boolean {
        value: bool,
    },
    Missing {
        reason: CanonicalMissingReasonV2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalColumnRoleV2 {
    Label,
    Estimate,
    Uncertainty,
    Decision,
    Diagnostic,
    Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultColumnV2 {
    pub id: String,
    pub label: String,
    pub data_type: CanonicalColumnTypeV2,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<CanonicalColumnRoleV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_precision: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultRowV2 {
    pub id: String,
    pub cells: Vec<CanonicalResultCellV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultTableV2 {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub columns: Vec<CanonicalResultColumnV2>,
    pub rows: Vec<CanonicalResultRowV2>,
    pub footnote_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cells: Option<Vec<CapabilityCellReferenceV2>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum CanonicalChartXValueV2 {
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalChartPointV2 {
    pub x: CanonicalChartXValueV2,
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalChartSeriesV2 {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub points: Vec<CanonicalChartPointV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct CanonicalChartDisplayOptionsV2 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_legend: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_values: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_axis_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_axis_label: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalChartKindV2 {
    Line,
    Bar,
    Scatter,
    Interval,
    Heatmap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultChartV2 {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: CanonicalChartKindV2,
    pub series: Vec<CanonicalChartSeriesV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_table_id: Option<String>,
    pub display: CanonicalChartDisplayOptionsV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultSectionV2 {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub table_ids: Vec<String>,
    pub chart_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cells: Option<Vec<CapabilityCellReferenceV2>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalNoticeSeverityV2 {
    Information,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultNoticeV2 {
    pub id: String,
    pub code: String,
    pub severity: CanonicalNoticeSeverityV2,
    pub message: String,
    pub section_ids: Vec<String>,
    pub table_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultExclusionV2 {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cell: Option<CapabilityCellReferenceV2>,
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultFootnoteV2 {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultProvenanceV2 {
    pub run_id: String,
    pub project_id: String,
    pub model_id: String,
    pub model_digest: String,
    pub dataset_id: String,
    pub dataset_fingerprint: String,
    pub recipe_id: String,
    pub recipe_digest: String,
    pub capability_cell: CapabilityCellReferenceV2,
    pub method_version: String,
    pub engine_version: String,
    pub seed: Option<u64>,
    pub workers: u32,
    pub started_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultPresentationV2 {
    pub default_section_id: Option<String>,
    pub default_table_id: Option<String>,
    pub precision: u8,
    pub missing_value_label: String,
    pub chart_defaults: CanonicalChartDisplayOptionsV2,
}

/// Rust wire mirror of `src/domain/canonicalResultDocumentV2.ts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultDocumentV2 {
    pub schema_version: u32,
    pub document_id: String,
    pub title: String,
    pub provenance: CanonicalResultProvenanceV2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cells: Option<Vec<CapabilityCellReferenceV2>>,
    /// Exact qpls-core General SEM V1 wire type. Reuse keeps archive persistence
    /// aligned with the analytical contract while this module validates the
    /// document-level model and capability cross-references.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general_sem_results: Option<CanonicalGeneralSemResultsV1>,
    pub sections: Vec<CanonicalResultSectionV2>,
    pub tables: Vec<CanonicalResultTableV2>,
    pub charts: Vec<CanonicalResultChartV2>,
    pub notices: Vec<CanonicalResultNoticeV2>,
    pub exclusions: Vec<CanonicalResultExclusionV2>,
    pub footnotes: Vec<CanonicalResultFootnoteV2>,
    pub presentation: CanonicalResultPresentationV2,
}

impl CanonicalResultDocumentV2 {
    pub fn ensure_valid(&self) -> Result<(), CanonicalResultDocumentV2Error> {
        if self.schema_version != CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION {
            return invalid("schema_version must equal 2");
        }
        require_stable_id(&self.document_id, "document_id")?;
        require_nonempty(&self.title, "title")?;

        let section_ids = unique_stable_ids(
            self.sections.iter().map(|item| item.id.as_str()),
            "sections",
        )?;
        let table_ids =
            unique_stable_ids(self.tables.iter().map(|item| item.id.as_str()), "tables")?;
        let chart_ids =
            unique_stable_ids(self.charts.iter().map(|item| item.id.as_str()), "charts")?;
        unique_stable_ids(self.notices.iter().map(|item| item.id.as_str()), "notices")?;
        unique_stable_ids(
            self.exclusions.iter().map(|item| item.id.as_str()),
            "exclusions",
        )?;
        let footnote_ids = unique_stable_ids(
            self.footnotes.iter().map(|item| item.id.as_str()),
            "footnotes",
        )?;

        let document_capabilities = match &self.capability_cells {
            Some(cells) => {
                let identities = validate_capability_set(cells, "capability_cells")?;
                let primary = capability_identity(&self.provenance.capability_cell)?;
                if !identities.contains(&primary) {
                    return invalid("capability_cells must include provenance.capability_cell");
                }
                Some(identities)
            }
            None => None,
        };

        let mut table_capabilities = BTreeMap::<&str, BTreeSet<String>>::new();
        for table in &self.tables {
            require_nonempty(&table.title, &format!("table {} title", table.id))?;
            let column_ids = unique_stable_ids(
                table.columns.iter().map(|item| item.id.as_str()),
                &format!("table {} columns", table.id),
            )?;
            unique_stable_ids(
                table.rows.iter().map(|item| item.id.as_str()),
                &format!("table {} rows", table.id),
            )?;
            for column in &table.columns {
                require_nonempty(
                    &column.label,
                    &format!("table {} column {} label", table.id, column.id),
                )?;
                require_nonempty(
                    &column.description,
                    &format!("table {} column {} description", table.id, column.id),
                )?;
                if column
                    .default_precision
                    .is_some_and(|precision| precision > 12)
                {
                    return invalid(format!(
                        "table {} column {} default_precision exceeds 12",
                        table.id, column.id
                    ));
                }
            }
            for row in &table.rows {
                if row.cells.len() != column_ids.len() {
                    return invalid(format!(
                        "table {} row {} has {} cells; expected {}",
                        table.id,
                        row.id,
                        row.cells.len(),
                        column_ids.len()
                    ));
                }
                for (column, cell) in table.columns.iter().zip(&row.cells) {
                    if let CanonicalResultCellV2::Number { value, .. } = cell
                        && !value.is_finite()
                    {
                        return invalid(format!(
                            "table {} row {} cell {} must be finite",
                            table.id, row.id, column.id
                        ));
                    }
                    if !cell_matches_column(cell, column.data_type) {
                        return invalid(format!(
                            "table {} row {} cell {} has the wrong type",
                            table.id, row.id, column.id
                        ));
                    }
                }
            }
            for footnote_id in &table.footnote_ids {
                if !footnote_ids.contains(footnote_id) {
                    return invalid(format!(
                        "table {} references missing footnote {}",
                        table.id, footnote_id
                    ));
                }
            }
            let capabilities = validate_child_capabilities(
                table.capability_cells.as_deref(),
                document_capabilities.as_ref(),
                &format!("table {}", table.id),
            )?;
            table_capabilities.insert(&table.id, capabilities);
        }

        for section in &self.sections {
            require_nonempty(&section.title, &format!("section {} title", section.id))?;
            for table_id in &section.table_ids {
                if !table_ids.contains(table_id) {
                    return invalid(format!(
                        "section {} references missing table {}",
                        section.id, table_id
                    ));
                }
            }
            for chart_id in &section.chart_ids {
                if !chart_ids.contains(chart_id) {
                    return invalid(format!(
                        "section {} references missing chart {}",
                        section.id, chart_id
                    ));
                }
            }
            let capabilities = validate_child_capabilities(
                section.capability_cells.as_deref(),
                document_capabilities.as_ref(),
                &format!("section {}", section.id),
            )?;
            for table_id in &section.table_ids {
                for identity in table_capabilities
                    .get(table_id.as_str())
                    .into_iter()
                    .flatten()
                {
                    if !capabilities.contains(identity) {
                        return invalid(format!(
                            "section {} is missing table option cell {}",
                            section.id, identity
                        ));
                    }
                }
            }
        }

        for chart in &self.charts {
            require_nonempty(&chart.title, &format!("chart {} title", chart.id))?;
            require_nonempty(
                &chart.description,
                &format!("chart {} description", chart.id),
            )?;
            if let Some(source_table_id) = &chart.source_table_id
                && !table_ids.contains(source_table_id)
            {
                return invalid(format!(
                    "chart {} references missing table {}",
                    chart.id, source_table_id
                ));
            }
            unique_stable_ids(
                chart.series.iter().map(|item| item.id.as_str()),
                &format!("chart {} series", chart.id),
            )?;
            for series in &chart.series {
                for point in &series.points {
                    let x_finite = match point.x {
                        CanonicalChartXValueV2::Number(value) => value.is_finite(),
                        CanonicalChartXValueV2::Text(_) => true,
                    };
                    if !x_finite
                        || !point.y.is_finite()
                        || point.lower.is_some_and(|value| !value.is_finite())
                        || point.upper.is_some_and(|value| !value.is_finite())
                    {
                        return invalid(format!(
                            "chart {} series {} contains a non-finite point",
                            chart.id, series.id
                        ));
                    }
                    if point
                        .lower
                        .zip(point.upper)
                        .is_some_and(|(lower, upper)| lower > upper)
                    {
                        return invalid(format!(
                            "chart {} series {} has lower greater than upper",
                            chart.id, series.id
                        ));
                    }
                }
            }
        }

        for notice in &self.notices {
            require_nonempty(&notice.code, &format!("notice {} code", notice.id))?;
            require_nonempty(&notice.message, &format!("notice {} message", notice.id))?;
            for section_id in &notice.section_ids {
                if !section_ids.contains(section_id) {
                    return invalid(format!(
                        "notice {} references missing section {}",
                        notice.id, section_id
                    ));
                }
            }
            for table_id in &notice.table_ids {
                if !table_ids.contains(table_id) {
                    return invalid(format!(
                        "notice {} references missing table {}",
                        notice.id, table_id
                    ));
                }
            }
        }

        for exclusion in &self.exclusions {
            require_nonempty(
                &exclusion.title,
                &format!("exclusion {} title", exclusion.id),
            )?;
            require_nonempty(
                &exclusion.reason,
                &format!("exclusion {} reason", exclusion.id),
            )?;
            if let Some(reference) = &exclusion.capability_cell {
                capability_identity(reference)?;
            }
        }
        for footnote in &self.footnotes {
            require_nonempty(&footnote.text, &format!("footnote {} text", footnote.id))?;
        }

        self.provenance.ensure_valid()?;
        if self.presentation.precision > 12 {
            return invalid("presentation.precision exceeds 12");
        }
        require_nonempty(
            &self.presentation.missing_value_label,
            "presentation.missing_value_label",
        )?;
        if let Some(section_id) = &self.presentation.default_section_id
            && !section_ids.contains(section_id)
        {
            return invalid("presentation.default_section_id is missing");
        }
        if let Some(table_id) = &self.presentation.default_table_id
            && !table_ids.contains(table_id)
        {
            return invalid("presentation.default_table_id is missing");
        }
        if let Some(results) = &self.general_sem_results {
            ensure_general_sem_results_finite(results)?;
            validate_general_sem_results_with_core(self)?;
        }
        Ok(())
    }
}

impl CanonicalResultProvenanceV2 {
    fn ensure_valid(&self) -> Result<(), CanonicalResultDocumentV2Error> {
        for (field, value) in [
            ("run_id", self.run_id.as_str()),
            ("project_id", self.project_id.as_str()),
            ("model_id", self.model_id.as_str()),
            ("dataset_id", self.dataset_id.as_str()),
            ("recipe_id", self.recipe_id.as_str()),
            ("method_version", self.method_version.as_str()),
            ("engine_version", self.engine_version.as_str()),
        ] {
            require_nonempty(value, &format!("provenance.{field}"))?;
        }
        for (field, digest) in [
            ("model_digest", self.model_digest.as_str()),
            ("recipe_digest", self.recipe_digest.as_str()),
        ] {
            require_sha256(digest, &format!("provenance.{field}"))?;
        }
        require_dataset_fingerprint(&self.dataset_fingerprint, "provenance.dataset_fingerprint")?;
        capability_identity(&self.capability_cell)?;
        if self
            .seed
            .is_some_and(|seed| seed > JAVASCRIPT_MAX_SAFE_INTEGER)
        {
            return invalid("provenance.seed exceeds the JavaScript safe-integer range");
        }
        if self.workers == 0 {
            return invalid("provenance.workers must be positive");
        }
        let started = DateTime::parse_from_rfc3339(&self.started_at)
            .map_err(|_| invalid_error("provenance.started_at must be an ISO timestamp"))?;
        let completed = DateTime::parse_from_rfc3339(&self.completed_at)
            .map_err(|_| invalid_error("provenance.completed_at must be an ISO timestamp"))?;
        if completed < started {
            return invalid("provenance.completed_at precedes started_at");
        }
        Ok(())
    }
}

fn validate_general_sem_results_with_core(
    document: &CanonicalResultDocumentV2,
) -> Result<(), CanonicalResultDocumentV2Error> {
    // qpls-project already depends on qpls-core. Converting the complete wire
    // document lets the authoritative validator enforce the same stable-ID
    // ordering, scientific uniqueness, probe/plot references, HOC stages,
    // fit constraints, model identity, and declared capability-cell rules.
    let wire = serde_json::to_value(document)?;
    let core_document = serde_json::from_value::<qpls_core::CanonicalResultDocumentV2>(wire)
        .map_err(|error| {
            invalid_error(format!(
                "general_sem_results does not match the qpls-core V1 wire: {error}"
            ))
        })?;
    let validation = qpls_core::validate_canonical_result_document_v2(&core_document);
    if !validation.passed {
        return invalid(format!(
            "qpls-core General SEM validation failed: {}",
            validation.errors.join("; ")
        ));
    }
    Ok(())
}

fn ensure_general_sem_results_finite(
    results: &CanonicalGeneralSemResultsV1,
) -> Result<(), CanonicalResultDocumentV2Error> {
    if let Some(receipt) = &results.inference_receipt {
        require_finite(
            receipt.confidence_level,
            "general_sem_results.inference_receipt.confidence_level",
        )?;
    }
    for (index, effect) in results.specific_indirect_effects.iter().enumerate() {
        ensure_general_sem_estimate_finite(
            &effect.value,
            &format!("general_sem_results.specific_indirect_effects[{index}].value"),
        )?;
    }
    for (index, effect) in results.aggregate_effects.iter().enumerate() {
        ensure_general_sem_estimate_finite(
            &effect.value,
            &format!("general_sem_results.aggregate_effects[{index}].value"),
        )?;
    }
    for (index, coefficient) in results
        .joint_stage_structural_coefficients
        .iter()
        .enumerate()
    {
        ensure_general_sem_estimate_finite(
            &coefficient.estimate,
            &format!("general_sem_results.joint_stage_structural_coefficients[{index}].estimate"),
        )?;
    }
    for (index, effect) in results.interaction_effects.iter().enumerate() {
        let context = format!("general_sem_results.interaction_effects[{index}]");
        require_finite(
            effect.unstandardized_product_mean,
            &format!("{context}.unstandardized_product_mean"),
        )?;
        require_finite(
            effect.unstandardized_product_sample_standard_deviation,
            &format!("{context}.unstandardized_product_sample_standard_deviation"),
        )?;
        ensure_general_sem_estimate_finite(
            &effect.standardized_product_coefficient,
            &format!("{context}.standardized_product_coefficient"),
        )?;
        ensure_general_sem_estimate_finite(
            &effect.scientific_rescaled_gamma,
            &format!("{context}.scientific_rescaled_gamma"),
        )?;
    }
    for (index, probe) in results.conditional_effect_probes.iter().enumerate() {
        let context = format!("general_sem_results.conditional_effect_probes[{index}].values");
        match &probe.values {
            CanonicalConditionalProbeValuesResultV1::DataDerivedMeanPlusMinusOneSd {
                mean,
                standard_deviation,
            } => {
                require_finite(*mean, &format!("{context}.mean"))?;
                require_finite(
                    *standard_deviation,
                    &format!("{context}.standard_deviation"),
                )?;
            }
            CanonicalConditionalProbeValuesResultV1::Explicit { values } => {
                for (value_index, value) in values.iter().enumerate() {
                    require_finite(*value, &format!("{context}.values[{value_index}]"))?;
                }
            }
        }
    }
    for (index, effect) in results.conditional_effects.iter().enumerate() {
        let context = format!("general_sem_results.conditional_effects[{index}]");
        require_finite(
            effect.moderator_value,
            &format!("{context}.moderator_value"),
        )?;
        ensure_general_sem_estimate_finite(&effect.value, &format!("{context}.value"))?;
    }
    for (plot_index, plot) in results.interaction_plots.iter().enumerate() {
        for (series_index, series) in plot.series.iter().enumerate() {
            let series_context = format!(
                "general_sem_results.interaction_plots[{plot_index}].series[{series_index}]"
            );
            require_finite(
                series.moderator_value,
                &format!("{series_context}.moderator_value"),
            )?;
            for (point_index, point) in series.points.iter().enumerate() {
                let point_context = format!("{series_context}.points[{point_index}]");
                require_finite(point.focal_value, &format!("{point_context}.focal_value"))?;
                require_finite(
                    point.predicted_value,
                    &format!("{point_context}.predicted_value"),
                )?;
                ensure_optional_finite(point.lower, &format!("{point_context}.lower"))?;
                ensure_optional_finite(point.upper, &format!("{point_context}.upper"))?;
            }
        }
    }
    for (stage_index, stage) in results.higher_order_stages.iter().enumerate() {
        for (relation_index, relation) in stage.relation_estimates.iter().enumerate() {
            ensure_general_sem_estimate_finite(
                &relation.value,
                &format!(
                    "general_sem_results.higher_order_stages[{stage_index}].relation_estimates[{relation_index}].value"
                ),
            )?;
        }
    }
    for (index, fit) in results.cbsem_fit.iter().enumerate() {
        let context = format!("general_sem_results.cbsem_fit[{index}]");
        require_finite(fit.chi_square, &format!("{context}.chi_square"))?;
        for (name, value) in [
            ("chi_square_p_value", fit.chi_square_p_value),
            ("rmsea", fit.rmsea),
            ("cfi", fit.cfi),
            ("tli", fit.tli),
            ("srmr", fit.srmr),
            ("aic", fit.aic),
            ("bic", fit.bic),
        ] {
            ensure_optional_finite(value, &format!("{context}.{name}"))?;
        }
        if let Some(interval) = &fit.rmsea_interval {
            let interval_context = format!("{context}.rmsea_interval");
            require_finite(
                interval.confidence_level,
                &format!("{interval_context}.confidence_level"),
            )?;
            require_finite(interval.lower, &format!("{interval_context}.lower"))?;
            require_finite(interval.upper, &format!("{interval_context}.upper"))?;
        }
    }
    Ok(())
}

fn ensure_general_sem_estimate_finite(
    value: &CanonicalGeneralSemEstimateV1,
    context: &str,
) -> Result<(), CanonicalResultDocumentV2Error> {
    require_finite(value.estimate, &format!("{context}.estimate"))?;
    ensure_optional_finite(value.bootstrap_mean, &format!("{context}.bootstrap_mean"))?;
    ensure_optional_finite(value.bootstrap_bias, &format!("{context}.bootstrap_bias"))?;
    ensure_optional_finite(value.standard_error, &format!("{context}.standard_error"))?;
    ensure_optional_finite(value.lower, &format!("{context}.lower"))?;
    ensure_optional_finite(value.upper, &format!("{context}.upper"))?;
    ensure_optional_finite(value.p_value, &format!("{context}.p_value"))?;
    Ok(())
}

fn ensure_optional_finite(
    value: Option<f64>,
    context: &str,
) -> Result<(), CanonicalResultDocumentV2Error> {
    if let Some(value) = value {
        require_finite(value, context)?;
    }
    Ok(())
}

fn require_finite(value: f64, context: &str) -> Result<(), CanonicalResultDocumentV2Error> {
    if !value.is_finite() {
        return invalid(format!("{context} must be finite"));
    }
    Ok(())
}

/// Immutable schema-6 attachment. Private fields prevent in-place edits by
/// normal Rust callers; replacement requires constructing and revalidating a
/// complete archive document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultDocumentAttachmentV2 {
    document_id: String,
    run_id: String,
    document_schema_version: u32,
    canonical_document: CanonicalResultDocumentV2,
    canonical_document_sha256: String,
    immutable: bool,
}

impl CanonicalResultDocumentAttachmentV2 {
    pub fn from_document(
        document: CanonicalResultDocumentV2,
    ) -> Result<Self, CanonicalResultDocumentV2Error> {
        document.ensure_valid()?;
        let canonical_document_sha256 = canonical_result_document_v2_sha256(&document)?;
        Ok(Self {
            document_id: document.document_id.clone(),
            run_id: document.provenance.run_id.clone(),
            document_schema_version: document.schema_version,
            canonical_document: document,
            canonical_document_sha256,
            immutable: true,
        })
    }

    pub fn document_id(&self) -> &str {
        &self.document_id
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn canonical_document(&self) -> &CanonicalResultDocumentV2 {
        &self.canonical_document
    }

    pub fn canonical_document_sha256(&self) -> &str {
        &self.canonical_document_sha256
    }

    pub fn immutable(&self) -> bool {
        self.immutable
    }

    pub fn ensure_valid(
        &self,
        expected_project_id: &str,
    ) -> Result<(), CanonicalResultDocumentV2Error> {
        self.canonical_document.ensure_valid()?;
        if self.document_schema_version != CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION
            || self.document_schema_version != self.canonical_document.schema_version
        {
            return invalid(
                "canonical result attachment schema identity differs from its document",
            );
        }
        if self.document_id != self.canonical_document.document_id
            || self.run_id != self.canonical_document.provenance.run_id
        {
            return invalid("canonical result attachment identity differs from its document");
        }
        if self.canonical_document.provenance.project_id != expected_project_id {
            return invalid("canonical result document belongs to a different project");
        }
        if !self.immutable {
            return invalid("canonical result attachment must be immutable");
        }
        require_sha256(&self.canonical_document_sha256, "canonical_document_sha256")?;
        if canonical_result_document_v2_sha256(&self.canonical_document)?
            != self.canonical_document_sha256
        {
            return Err(CanonicalResultDocumentV2Error::DigestMismatch {
                document_id: self.document_id.clone(),
            });
        }
        Ok(())
    }
}

pub fn canonical_result_document_v2_json(
    document: &CanonicalResultDocumentV2,
) -> Result<Vec<u8>, CanonicalResultDocumentV2Error> {
    document.ensure_valid()?;
    let canonical = canonicalize_json_value(serde_json::to_value(document)?);
    Ok(serde_json::to_vec(&canonical)?)
}

pub fn canonical_result_document_v2_sha256(
    document: &CanonicalResultDocumentV2,
) -> Result<String, CanonicalResultDocumentV2Error> {
    Ok(format!(
        "{:x}",
        Sha256::digest(canonical_result_document_v2_json(document)?)
    ))
}

fn canonicalize_json_value(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, canonicalize_json_value(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(canonicalize_json_value).collect())
        }
        scalar => scalar,
    }
}

fn validate_child_capabilities(
    child: Option<&[CapabilityCellReferenceV2]>,
    document: Option<&BTreeSet<String>>,
    context: &str,
) -> Result<BTreeSet<String>, CanonicalResultDocumentV2Error> {
    match (child, document) {
        (None, None) => Ok(BTreeSet::new()),
        (Some(_), None) => invalid(format!(
            "{context} cannot declare capability_cells without a document capability_cells set"
        )),
        (None, Some(_)) => invalid(format!("{context} must declare capability_cells")),
        (Some(cells), Some(document)) => {
            let identities =
                validate_capability_set(cells, &format!("{context}.capability_cells"))?;
            if let Some(identity) = identities
                .iter()
                .find(|identity| !document.contains(*identity))
            {
                return invalid(format!(
                    "{context} references an undeclared option cell {identity}"
                ));
            }
            Ok(identities)
        }
    }
}

fn validate_capability_set(
    references: &[CapabilityCellReferenceV2],
    context: &str,
) -> Result<BTreeSet<String>, CanonicalResultDocumentV2Error> {
    if references.is_empty() {
        return invalid(format!("{context} must not be empty"));
    }
    let identities = references
        .iter()
        .map(capability_identity)
        .collect::<Result<Vec<_>, _>>()?;
    let distinct = identities.iter().cloned().collect::<BTreeSet<_>>();
    if distinct.len() != identities.len() {
        return invalid(format!("{context} contains duplicate references"));
    }
    if identities.iter().ne(distinct.iter()) {
        return invalid(format!(
            "{context} must be ordered by exact option-cell identity"
        ));
    }
    Ok(distinct)
}

fn capability_identity(
    reference: &CapabilityCellReferenceV2,
) -> Result<String, CanonicalResultDocumentV2Error> {
    if reference.registry_schema_version != 2 {
        return invalid("capability reference registry_schema_version must equal 2");
    }
    require_stable_id(&reference.capability_id, "capability_id")?;
    require_stable_id(&reference.cell_id, "cell_id")?;
    require_stable_id(&reference.capability_version, "capability_version")?;
    Ok(format!(
        "{}:{}:{}:{}",
        reference.registry_schema_version,
        reference.capability_id,
        reference.cell_id,
        reference.capability_version
    ))
}

fn cell_matches_column(cell: &CanonicalResultCellV2, column: CanonicalColumnTypeV2) -> bool {
    matches!(cell, CanonicalResultCellV2::Missing { .. })
        || matches!(
            (cell, column),
            (
                CanonicalResultCellV2::Number { .. },
                CanonicalColumnTypeV2::Number
            ) | (
                CanonicalResultCellV2::Text { .. },
                CanonicalColumnTypeV2::Text
            ) | (
                CanonicalResultCellV2::Boolean { .. },
                CanonicalColumnTypeV2::Boolean
            )
        )
}

fn unique_stable_ids<'a>(
    values: impl Iterator<Item = &'a str>,
    context: &str,
) -> Result<BTreeSet<String>, CanonicalResultDocumentV2Error> {
    let mut output = BTreeSet::new();
    for value in values {
        require_stable_id(value, context)?;
        if !output.insert(value.to_owned()) {
            return invalid(format!("{context} contains duplicate ID {value}"));
        }
    }
    Ok(output)
}

fn require_stable_id(value: &str, context: &str) -> Result<(), CanonicalResultDocumentV2Error> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return invalid(format!("{context} must be a stable lowercase identifier"));
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return invalid(format!("{context} must be a stable lowercase identifier"));
    }
    if chars.any(|character| {
        !character.is_ascii_lowercase()
            && !character.is_ascii_digit()
            && !matches!(character, '.' | '_' | ':' | '-')
    }) {
        return invalid(format!("{context} must be a stable lowercase identifier"));
    }
    Ok(())
}

fn require_nonempty(value: &str, context: &str) -> Result<(), CanonicalResultDocumentV2Error> {
    if value.trim().is_empty() {
        return invalid(format!("{context} must be nonempty"));
    }
    Ok(())
}

fn require_sha256(value: &str, context: &str) -> Result<(), CanonicalResultDocumentV2Error> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return invalid(format!("{context} must be lowercase SHA-256"));
    }
    Ok(())
}

fn require_dataset_fingerprint(
    value: &str,
    context: &str,
) -> Result<(), CanonicalResultDocumentV2Error> {
    let digest = value.strip_prefix("v2:").unwrap_or(value);
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return invalid(format!(
            "{context} must be bare lowercase SHA-256 or v2:<lowercase SHA-256>"
        ));
    }
    Ok(())
}

fn invalid<T>(message: impl Into<String>) -> Result<T, CanonicalResultDocumentV2Error> {
    Err(invalid_error(message))
}

fn invalid_error(message: impl Into<String>) -> CanonicalResultDocumentV2Error {
    CanonicalResultDocumentV2Error::Invalid(message.into())
}

#[derive(Debug, thiserror::Error)]
pub enum CanonicalResultDocumentV2Error {
    #[error("invalid CanonicalResultDocumentV2: {0}")]
    Invalid(String),
    #[error("canonical result document {document_id} digest mismatch")]
    DigestMismatch { document_id: String },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability_cell() -> CapabilityCellReferenceV2 {
        CapabilityCellReferenceV2 {
            registry_schema_version: 2,
            capability_id: "smartpls.mediation".into(),
            cell_id: "qpls3.pls.mediation".into(),
            capability_version: "pls_mediation_v1".into(),
        }
    }

    fn bootstrap_capability_cell() -> CapabilityCellReferenceV2 {
        let cell = qpls_core::general_sem_pls_bootstrap_capability_cell_v1();
        CapabilityCellReferenceV2 {
            registry_schema_version: cell.registry_schema_version,
            capability_id: cell.capability_id,
            cell_id: cell.cell_id,
            capability_version: cell.capability_version,
        }
    }

    fn core_capability_cell() -> qpls_core::CapabilityCellReferenceV2 {
        qpls_core::CapabilityCellReferenceV2 {
            registry_schema_version: 2,
            capability_id: "smartpls.mediation".into(),
            cell_id: "qpls3.pls.mediation".into(),
            capability_version: "pls_mediation_v1".into(),
        }
    }

    fn trace() -> CanonicalGeneralSemResultTraceV1 {
        CanonicalGeneralSemResultTraceV1 {
            model_id: "model_1".into(),
            capability_cell: core_capability_cell(),
        }
    }

    fn estimate(value: f64) -> CanonicalGeneralSemEstimateV1 {
        CanonicalGeneralSemEstimateV1 {
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

    fn complete_general_sem_results() -> CanonicalGeneralSemResultsV1 {
        CanonicalGeneralSemResultsV1 {
            schema_version: CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION,
            inference_receipt: None,
            specific_indirect_effects: vec![CanonicalSpecificIndirectEffectResultV1 {
                effect_id: qpls_core::specific_directed_path_identity_v1(&[
                    "relation_x_m".to_string(),
                    "relation_m_y".to_string(),
                ]),
                estimand_id: "estimand_specific_1".into(),
                trace: trace(),
                source_id: "construct:x".into(),
                target_id: "construct:y".into(),
                ordered_relation_ids: vec!["relation_x_m".into(), "relation_m_y".into()],
                value: estimate(0.2),
            }],
            aggregate_effects: vec![
                CanonicalAggregateEffectResultV1 {
                    effect_id: "estimand_total_effect_1".into(),
                    estimand_id: "estimand_total_effect_1".into(),
                    trace: trace(),
                    kind: CanonicalAggregateEffectKindV1::TotalEffect,
                    source_id: "construct:x".into(),
                    target_id: "construct:y".into(),
                    direct_relation_ids: vec!["relation_x_y".into()],
                    contributing_path_identities: vec![
                        qpls_core::specific_directed_path_identity_v1(&[
                            "relation_x_m".to_string(),
                            "relation_m_y".to_string(),
                        ]),
                    ],
                    value: estimate(0.5),
                },
                CanonicalAggregateEffectResultV1 {
                    effect_id: "estimand_total_indirect_1".into(),
                    estimand_id: "estimand_total_indirect_1".into(),
                    trace: trace(),
                    kind: CanonicalAggregateEffectKindV1::TotalIndirect,
                    source_id: "construct:x".into(),
                    target_id: "construct:y".into(),
                    direct_relation_ids: Vec::new(),
                    contributing_path_identities: vec![
                        qpls_core::specific_directed_path_identity_v1(&[
                            "relation_x_m".to_string(),
                            "relation_m_y".to_string(),
                        ]),
                    ],
                    value: estimate(0.2),
                },
            ],
            joint_stage_structural_coefficients: Vec::new(),
            interaction_effects: Vec::new(),
            conditional_effect_probes: vec![
                CanonicalConditionalEffectProbeResultV1 {
                    probe_id: "probe_explicit".into(),
                    trace: trace(),
                    moderator_id: "construct:w".into(),
                    values: CanonicalConditionalProbeValuesResultV1::Explicit {
                        values: vec![-1.0, 0.0, 1.0],
                    },
                },
                CanonicalConditionalEffectProbeResultV1 {
                    probe_id: "probe_sd".into(),
                    trace: trace(),
                    moderator_id: "construct:w".into(),
                    values:
                        CanonicalConditionalProbeValuesResultV1::DataDerivedMeanPlusMinusOneSd {
                            mean: 0.0,
                            standard_deviation: 1.0,
                        },
                },
            ],
            conditional_effects: vec![CanonicalConditionalEffectResultV1 {
                effect_id: "effect_conditional_1".into(),
                estimand_id: "estimand_conditional_1".into(),
                trace: trace(),
                interaction_id: "interaction_1".into(),
                interaction_effect_id: None,
                focal_relation_id: "relation_x_y".into(),
                probe_id: "probe_explicit".into(),
                moderator_id: "construct:w".into(),
                probe_value_index: 1,
                moderator_value: 0.0,
                value: estimate(0.4),
            }],
            interaction_plots: vec![CanonicalInteractionPlotResultV1 {
                plot_id: "plot_1".into(),
                trace: trace(),
                interaction_id: "interaction_1".into(),
                interaction_effect_id: None,
                focal_relation_id: "relation_x_y".into(),
                focal_predictor_id: "construct:x".into(),
                moderator_id: "construct:w".into(),
                outcome_id: "construct:y".into(),
                series: vec![
                    CanonicalInteractionPlotSeriesV1 {
                        series_id: "series_high".into(),
                        probe_id: "probe_explicit".into(),
                        probe_value_index: 2,
                        moderator_value: 1.0,
                        points: vec![
                            CanonicalInteractionPlotPointV1 {
                                focal_value: -1.0,
                                predicted_value: 0.1,
                                lower: Some(0.0),
                                upper: Some(0.2),
                            },
                            CanonicalInteractionPlotPointV1 {
                                focal_value: 1.0,
                                predicted_value: 0.7,
                                lower: Some(0.6),
                                upper: Some(0.8),
                            },
                        ],
                    },
                    CanonicalInteractionPlotSeriesV1 {
                        series_id: "series_low".into(),
                        probe_id: "probe_explicit".into(),
                        probe_value_index: 0,
                        moderator_value: -1.0,
                        points: vec![
                            CanonicalInteractionPlotPointV1 {
                                focal_value: -1.0,
                                predicted_value: 0.2,
                                lower: Some(0.1),
                                upper: Some(0.3),
                            },
                            CanonicalInteractionPlotPointV1 {
                                focal_value: 1.0,
                                predicted_value: 0.4,
                                lower: Some(0.3),
                                upper: Some(0.5),
                            },
                        ],
                    },
                ],
            }],
            higher_order_stages: vec![
                CanonicalHocStageResultV1 {
                    stage_id: "stage_1".into(),
                    trace: trace(),
                    higher_order_construct_id: "construct:hoc".into(),
                    stage_number: 1,
                    kind: CanonicalHocStageKindV1::LowerOrderScoreEstimation,
                    input_construct_ids: vec!["construct:m1".into(), "construct:m2".into()],
                    output_variable_ids: vec!["score:hoc".into()],
                    relation_estimates: vec![CanonicalHocRelationEstimateV1 {
                        relation_id: "relation_m1_hoc".into(),
                        source_id: "construct:m1".into(),
                        target_id: "construct:hoc".into(),
                        value: estimate(0.8),
                    }],
                },
                CanonicalHocStageResultV1 {
                    stage_id: "stage_2".into(),
                    trace: trace(),
                    higher_order_construct_id: "construct:hoc".into(),
                    stage_number: 2,
                    kind: CanonicalHocStageKindV1::HigherOrderEstimation,
                    input_construct_ids: vec!["score:hoc".into()],
                    output_variable_ids: vec!["construct:y".into()],
                    relation_estimates: vec![CanonicalHocRelationEstimateV1 {
                        relation_id: "relation_hoc_y".into(),
                        source_id: "construct:hoc".into(),
                        target_id: "construct:y".into(),
                        value: estimate(0.6),
                    }],
                },
            ],
            cbsem_fit: vec![CanonicalCbsemFitResultV1 {
                fit_id: "fit_1".into(),
                trace: trace(),
                chi_square: 12.0,
                degrees_of_freedom: 10,
                chi_square_p_value: Some(0.28),
                rmsea: Some(0.05),
                rmsea_interval: Some(CanonicalGeneralSemIntervalV1 {
                    confidence_level: 0.9,
                    lower: 0.02,
                    upper: 0.08,
                }),
                cfi: Some(0.97),
                tli: Some(0.96),
                srmr: Some(0.04),
                aic: Some(120.0),
                bic: Some(130.0),
            }],
            identification_diagnostics: vec![CanonicalIdentificationDiagnosticV1 {
                diagnostic_id: "diagnostic_1".into(),
                trace: trace(),
                scope: CanonicalIdentificationScopeV1::Model,
                subject_id: "model_1".into(),
                status: CanonicalIdentificationStatusV1::Identified,
                code: "identified".into(),
                message: "The model is identified.".into(),
                degrees_of_freedom: Some(10),
            }],
        }
    }

    fn complete_general_sem_inference_results() -> CanonicalGeneralSemResultsV1 {
        let mut results = complete_general_sem_results();
        for effect in &mut results.specific_indirect_effects {
            effect.value = inferred_estimate(effect.value.estimate);
        }
        for effect in &mut results.aggregate_effects {
            effect.value = inferred_estimate(effect.value.estimate);
        }
        let mut effect_ids = results
            .specific_indirect_effects
            .iter()
            .map(|effect| effect.effect_id.clone())
            .chain(
                results
                    .aggregate_effects
                    .iter()
                    .map(|effect| effect.effect_id.clone()),
            )
            .collect::<Vec<_>>();
        effect_ids.sort();
        let usable_indices = (0..10_u32)
            .filter(|replicate_index| *replicate_index != 7)
            .collect::<Vec<_>>();
        results.inference_receipt = Some(CanonicalGeneralSemInferenceReceiptV1 {
            kind: CanonicalGeneralSemInferenceKindV1::CaseBootstrap,
            capability_cell: qpls_core::general_sem_pls_bootstrap_capability_cell_v1(),
            method_version: qpls_core::GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1.into(),
            resampling_operation_version:
                qpls_core::GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1.into(),
            resampling_stream_version:
                qpls_core::GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1.into(),
            quantile_method_version: qpls_core::GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1.into(),
            standard_error_method_version:
                qpls_core::GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1.into(),
            summation_method_version: qpls_core::GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1
                .into(),
            p_value_method_version:
                qpls_core::GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1.into(),
            failure_policy_version:
                qpls_core::GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1.into(),
            compilation_artifact_identity_sha256: "d".repeat(64),
            compiled_plan_sha256: "9".repeat(64),
            general_sem_config_sha256: "e".repeat(64),
            recipe_analytical_sha256: "c".repeat(64),
            model_scientific_sha256: "a".repeat(64),
            source_dataset_fingerprint: "b".repeat(64),
            complete_case_frame_sha256: "f".repeat(64),
            usable_replicate_indices_sha256: qpls_core::sha256_serialized(&usable_indices),
            effect_identity_set_sha256: qpls_core::general_sem_effect_identity_set_sha256_v1(
                &qpls_core::canonical_general_sem_effect_identities_v1(&results),
            ),
            effect_ids,
            interval: CanonicalGeneralSemBootstrapIntervalV1::PercentileType7,
            tail: CanonicalGeneralSemInferenceTailV1::TwoSided,
            confidence_level: 0.95,
            resamples_requested: 10,
            resamples_usable: 9,
            minimum_usable_resamples: 9,
            seed: "1".to_string(),
            workers: 1,
            complete_model_reestimated_per_replicate: true,
            failed_replicates: vec![CanonicalGeneralSemFailedReplicateV1 {
                replicate_index: 7,
                reason_code:
                    qpls_core::CanonicalGeneralSemFailedReplicateReasonV1::EstimationNonconvergence,
                message: "The complete model did not converge for this draw.".into(),
            }],
        });
        results
    }

    fn inferred_estimate(value: f64) -> CanonicalGeneralSemEstimateV1 {
        CanonicalGeneralSemEstimateV1 {
            estimate: value,
            bootstrap_mean: Some(value + 0.01),
            bootstrap_bias: Some(0.01),
            standard_error: Some(0.05),
            lower: Some(value - 0.1),
            upper: Some(value + 0.1),
            p_value: Some(0.2),
            bootstrap_usable_replicates: Some(9),
            bootstrap_two_sided_exceedances: Some(1),
        }
    }

    fn document() -> CanonicalResultDocumentV2 {
        CanonicalResultDocumentV2 {
            schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
            document_id: "document_1".into(),
            title: "Canonical result".into(),
            provenance: CanonicalResultProvenanceV2 {
                run_id: "run_1".into(),
                project_id: "project_1".into(),
                model_id: "model_1".into(),
                model_digest: "a".repeat(64),
                dataset_id: "dataset_1".into(),
                dataset_fingerprint: "b".repeat(64),
                recipe_id: "recipe_1".into(),
                recipe_digest: "c".repeat(64),
                capability_cell: capability_cell(),
                method_version: "method_v1".into(),
                engine_version: "engine_v1".into(),
                seed: Some(1),
                workers: 1,
                started_at: "2026-08-18T00:00:00Z".into(),
                completed_at: "2026-08-18T00:00:01Z".into(),
            },
            capability_cells: Some(vec![capability_cell()]),
            general_sem_results: None,
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
        }
    }

    #[test]
    fn populated_general_sem_results_round_trip_through_project_and_core_wires() {
        let mut source = document();
        source.general_sem_results = Some(complete_general_sem_results());
        source.ensure_valid().unwrap();

        let encoded = serde_json::to_vec(&source).unwrap();
        let restored: CanonicalResultDocumentV2 = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, source);
        restored.ensure_valid().unwrap();

        let core_document: qpls_core::CanonicalResultDocumentV2 =
            serde_json::from_slice(&encoded).unwrap();
        let core_validation = qpls_core::validate_canonical_result_document_v2(&core_document);
        assert!(core_validation.passed, "{:?}", core_validation.errors);
    }

    #[test]
    fn general_sem_inference_receipt_round_trips_and_fails_closed_in_archive_wire() {
        let mut source = document();
        source.capability_cells = Some(vec![bootstrap_capability_cell(), capability_cell()]);
        source.general_sem_results = Some(complete_general_sem_inference_results());
        source.ensure_valid().unwrap();

        let attachment =
            CanonicalResultDocumentAttachmentV2::from_document(source.clone()).unwrap();
        let encoded = serde_json::to_vec(&attachment).unwrap();
        let restored: CanonicalResultDocumentAttachmentV2 =
            serde_json::from_slice(&encoded).unwrap();
        restored.ensure_valid("project_1").unwrap();
        assert_eq!(restored.canonical_document(), &source);
        let receipt = restored
            .canonical_document()
            .general_sem_results
            .as_ref()
            .unwrap()
            .inference_receipt
            .as_ref()
            .unwrap();
        assert_eq!(receipt.seed, "1");
        assert_eq!(receipt.resamples_usable, 9);

        let mut nonfinite = source.clone();
        nonfinite
            .general_sem_results
            .as_mut()
            .unwrap()
            .specific_indirect_effects[0]
            .value
            .bootstrap_mean = Some(f64::NAN);
        assert!(
            nonfinite
                .ensure_valid()
                .unwrap_err()
                .to_string()
                .contains("bootstrap_mean must be finite")
        );

        let mut tampered = source;
        tampered
            .general_sem_results
            .as_mut()
            .unwrap()
            .inference_receipt
            .as_mut()
            .unwrap()
            .effect_identity_set_sha256 = "0".repeat(64);
        assert!(
            tampered
                .ensure_valid()
                .unwrap_err()
                .to_string()
                .contains("effect_identity_set_sha256 does not match")
        );
    }

    #[test]
    fn legacy_document_omits_general_sem_results_without_changing_round_trip() {
        let source = document();
        source.ensure_valid().unwrap();
        let encoded = serde_json::to_vec(&source).unwrap();
        let value: Value = serde_json::from_slice(&encoded).unwrap();
        assert!(value.get("general_sem_results").is_none());

        let restored: CanonicalResultDocumentV2 = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, source);
        assert!(restored.general_sem_results.is_none());
    }

    #[test]
    fn general_sem_wire_rejects_unknown_fields_and_invalid_schema() {
        let mut source = document();
        source.general_sem_results = Some(complete_general_sem_results());
        let mut unknown = serde_json::to_value(&source).unwrap();
        unknown["general_sem_results"]["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CanonicalResultDocumentV2>(unknown).is_err());

        source.general_sem_results.as_mut().unwrap().schema_version = 2;
        let error = source.ensure_valid().unwrap_err().to_string();
        assert!(error.contains("general_sem_results.schema_version must equal 1"));
    }

    #[test]
    fn general_sem_validation_rejects_nonfinite_ordering_and_cross_reference_tampering() {
        let mut nonfinite = document();
        nonfinite.general_sem_results = Some(complete_general_sem_results());
        nonfinite
            .general_sem_results
            .as_mut()
            .unwrap()
            .specific_indirect_effects[0]
            .value
            .standard_error = Some(f64::NAN);
        assert!(
            nonfinite
                .ensure_valid()
                .unwrap_err()
                .to_string()
                .contains("standard_error must be finite")
        );

        let mut unsorted = document();
        unsorted.general_sem_results = Some(complete_general_sem_results());
        unsorted
            .general_sem_results
            .as_mut()
            .unwrap()
            .aggregate_effects
            .reverse();
        assert!(
            unsorted
                .ensure_valid()
                .unwrap_err()
                .to_string()
                .contains("aggregate_effects must be ordered")
        );

        let mut missing_probe = document();
        missing_probe.general_sem_results = Some(complete_general_sem_results());
        missing_probe
            .general_sem_results
            .as_mut()
            .unwrap()
            .conditional_effects[0]
            .probe_id = "probe_missing".into();
        assert!(
            missing_probe
                .ensure_valid()
                .unwrap_err()
                .to_string()
                .contains("references a missing conditional-effect probe")
        );

        let mut undeclared_capability = document();
        undeclared_capability.general_sem_results = Some(complete_general_sem_results());
        undeclared_capability
            .general_sem_results
            .as_mut()
            .unwrap()
            .specific_indirect_effects[0]
            .trace
            .capability_cell
            .cell_id = "qpls3.pls.other".into();
        assert!(
            undeclared_capability
                .ensure_valid()
                .unwrap_err()
                .to_string()
                .contains("references undeclared option cell")
        );
    }
}
