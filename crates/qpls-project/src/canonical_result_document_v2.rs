use chrono::DateTime;
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
            ("dataset_fingerprint", self.dataset_fingerprint.as_str()),
            ("recipe_digest", self.recipe_digest.as_str()),
        ] {
            require_sha256(digest, &format!("provenance.{field}"))?;
        }
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
