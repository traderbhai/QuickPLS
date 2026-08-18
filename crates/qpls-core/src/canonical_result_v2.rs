use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub const CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION: u32 = 2;
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCellReferenceV2 {
    pub registry_schema_version: u32,
    pub capability_id: String,
    pub cell_id: String,
    pub capability_version: String,
}

pub fn capability_cell_reference_identity_v2(reference: &CapabilityCellReferenceV2) -> String {
    format!(
        "{}:{}:{}:{}",
        reference.registry_schema_version,
        reference.capability_id,
        reference.cell_id,
        reference.capability_version
    )
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalColumnType {
    Number,
    Text,
    Boolean,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalMissingReason {
    NotApplicable,
    NotEstimated,
    Undefined,
    Withheld,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanonicalResultCell {
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
        reason: CanonicalMissingReason,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display: Option<String>,
    },
}

impl CanonicalResultCell {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Number { .. } => "number",
            Self::Text { .. } => "text",
            Self::Boolean { .. } => "boolean",
            Self::Missing { .. } => "missing",
        }
    }

    fn matches_column(&self, data_type: CanonicalColumnType) -> bool {
        matches!(
            (self, data_type),
            (Self::Missing { .. }, _)
                | (Self::Number { .. }, CanonicalColumnType::Number)
                | (Self::Text { .. }, CanonicalColumnType::Text)
                | (Self::Boolean { .. }, CanonicalColumnType::Boolean)
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalColumnRole {
    Label,
    Estimate,
    Uncertainty,
    Decision,
    Diagnostic,
    Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultColumn {
    pub id: String,
    pub label: String,
    pub data_type: CanonicalColumnType,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<CanonicalColumnRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_precision: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultRow {
    pub id: String,
    pub cells: Vec<CanonicalResultCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultTable {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub columns: Vec<CanonicalResultColumn>,
    pub rows: Vec<CanonicalResultRow>,
    pub footnote_ids: Vec<String>,
    /// Explicit option cells that produced this table. Missing only for
    /// historical compatibility documents, which are not comparison or
    /// qualification-export eligible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cells: Option<Vec<CapabilityCellReferenceV2>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum CanonicalChartX {
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalChartPoint {
    pub x: CanonicalChartX,
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
pub struct CanonicalChartSeries {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub points: Vec<CanonicalChartPoint>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalChartDisplayOptions {
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
pub enum CanonicalChartKind {
    Line,
    Bar,
    Scatter,
    Interval,
    Heatmap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultChart {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: CanonicalChartKind,
    pub series: Vec<CanonicalChartSeries>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_table_id: Option<String>,
    pub display: CanonicalChartDisplayOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultSection {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub table_ids: Vec<String>,
    pub chart_ids: Vec<String>,
    /// Explicit union of option cells represented by this section. Missing
    /// only for historical compatibility documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cells: Option<Vec<CapabilityCellReferenceV2>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalNoticeSeverity {
    Information,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultNotice {
    pub id: String,
    pub code: String,
    pub severity: CanonicalNoticeSeverity,
    pub message: String,
    pub section_ids: Vec<String>,
    pub table_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultExclusion {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cell: Option<CapabilityCellReferenceV2>,
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultFootnote {
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
    pub seed: Option<i64>,
    pub workers: i64,
    pub started_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultPresentationV2 {
    pub default_section_id: Option<String>,
    pub default_table_id: Option<String>,
    pub precision: i32,
    pub missing_value_label: String,
    pub chart_defaults: CanonicalChartDisplayOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultDocumentV2 {
    pub schema_version: u32,
    pub document_id: String,
    pub title: String,
    pub provenance: CanonicalResultProvenanceV2,
    /// Sorted, distinct option-cell set. `provenance.capability_cell` remains
    /// the primary capability for wire compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_cells: Option<Vec<CapabilityCellReferenceV2>>,
    pub sections: Vec<CanonicalResultSection>,
    pub tables: Vec<CanonicalResultTable>,
    pub charts: Vec<CanonicalResultChart>,
    pub notices: Vec<CanonicalResultNotice>,
    pub exclusions: Vec<CanonicalResultExclusion>,
    pub footnotes: Vec<CanonicalResultFootnote>,
    pub presentation: CanonicalResultPresentationV2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalResultValidation {
    pub passed: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalResultQualificationIneligibilityV2 {
    LegacyCapabilityAttributionMissing,
    InvalidDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResultUseEligibilityV2 {
    pub readable: bool,
    pub comparison_eligible: bool,
    pub qualification_export_eligible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ineligibility: Option<CanonicalResultQualificationIneligibilityV2>,
}

fn is_stable_id(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    characters.all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '_' | '.' | ':' | '-')
    })
}

fn require_stable_id(errors: &mut Vec<String>, value: &str, context: &str) {
    if !is_stable_id(value) {
        errors.push(format!("{context} must be a stable lowercase identifier"));
    }
}

fn require_unique_ids<'a>(
    errors: &mut Vec<String>,
    ids: impl IntoIterator<Item = &'a str>,
    context: &str,
) {
    let ids: Vec<&str> = ids.into_iter().collect();
    let mut seen = HashSet::new();
    let mut duplicates = BTreeSet::new();
    for id in &ids {
        if !seen.insert(*id) {
            duplicates.insert(*id);
        }
    }
    if !duplicates.is_empty() {
        errors.push(format!(
            "{context} contains duplicate IDs: {}",
            duplicates.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    for id in ids {
        require_stable_id(errors, id, &format!("{context} ID {id:?}"));
    }
}

fn validate_capability_reference(
    errors: &mut Vec<String>,
    reference: &CapabilityCellReferenceV2,
    context: &str,
) {
    if reference.registry_schema_version != 2 {
        errors.push(format!("{context}.registry_schema_version must equal 2"));
    }
    require_stable_id(
        errors,
        &reference.capability_id,
        &format!("{context}.capability_id"),
    );
    require_stable_id(errors, &reference.cell_id, &format!("{context}.cell_id"));
    if reference.capability_version.trim().is_empty() {
        errors.push(format!("{context}.capability_version must be nonempty"));
    }
}

fn validate_capability_set(
    errors: &mut Vec<String>,
    references: &[CapabilityCellReferenceV2],
    context: &str,
) -> Vec<String> {
    if references.is_empty() {
        errors.push(format!("{context} must not be empty"));
    }
    let identities = references
        .iter()
        .enumerate()
        .map(|(index, reference)| {
            validate_capability_reference(errors, reference, &format!("{context}[{index}]"));
            capability_cell_reference_identity_v2(reference)
        })
        .collect::<Vec<_>>();
    let mut seen = HashSet::new();
    let duplicates = identities
        .iter()
        .filter(|identity| !seen.insert(identity.as_str()))
        .cloned()
        .collect::<BTreeSet<_>>();
    if !duplicates.is_empty() {
        errors.push(format!(
            "{context} contains duplicate references: {}",
            duplicates.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    let mut sorted = identities.clone();
    sorted.sort();
    if identities != sorted {
        errors.push(format!(
            "{context} must be ordered by exact option-cell identity"
        ));
    }
    identities
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_timestamp(value: &str) -> Option<DateTime<chrono::FixedOffset>> {
    DateTime::parse_from_rfc3339(value).ok()
}

pub fn validate_canonical_result_document_v2(
    document: &CanonicalResultDocumentV2,
) -> CanonicalResultValidation {
    let mut errors = Vec::new();
    if document.schema_version != CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version must equal {CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION}"
        ));
    }
    require_stable_id(&mut errors, &document.document_id, "document_id");
    if document.title.trim().is_empty() {
        errors.push("title must be nonempty".to_string());
    }

    require_unique_ids(
        &mut errors,
        document.sections.iter().map(|item| item.id.as_str()),
        "sections",
    );
    require_unique_ids(
        &mut errors,
        document.tables.iter().map(|item| item.id.as_str()),
        "tables",
    );
    require_unique_ids(
        &mut errors,
        document.charts.iter().map(|item| item.id.as_str()),
        "charts",
    );
    require_unique_ids(
        &mut errors,
        document.notices.iter().map(|item| item.id.as_str()),
        "notices",
    );
    require_unique_ids(
        &mut errors,
        document.exclusions.iter().map(|item| item.id.as_str()),
        "exclusions",
    );
    require_unique_ids(
        &mut errors,
        document.footnotes.iter().map(|item| item.id.as_str()),
        "footnotes",
    );

    let section_ids: HashSet<&str> = document
        .sections
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let table_ids: HashSet<&str> = document
        .tables
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let chart_ids: HashSet<&str> = document
        .charts
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let footnote_ids: HashSet<&str> = document
        .footnotes
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let document_capability_ids = document.capability_cells.as_ref().map(|references| {
        let identities = validate_capability_set(&mut errors, references, "capability_cells");
        let identities = identities.into_iter().collect::<HashSet<_>>();
        let primary_identity =
            capability_cell_reference_identity_v2(&document.provenance.capability_cell);
        if !identities.contains(&primary_identity) {
            errors.push("capability_cells must include provenance.capability_cell".to_string());
        }
        identities
    });

    for section in &document.sections {
        if section.title.trim().is_empty() {
            errors.push(format!("section {} title must be nonempty", section.id));
        }
        for table_id in &section.table_ids {
            if !table_ids.contains(table_id.as_str()) {
                errors.push(format!(
                    "section {} references missing table {table_id}",
                    section.id
                ));
            }
        }
        for chart_id in &section.chart_ids {
            if !chart_ids.contains(chart_id.as_str()) {
                errors.push(format!(
                    "section {} references missing chart {chart_id}",
                    section.id
                ));
            }
        }
        match (&document_capability_ids, &section.capability_cells) {
            (Some(document_ids), Some(references)) => {
                let identities = validate_capability_set(
                    &mut errors,
                    references,
                    &format!("section {}.capability_cells", section.id),
                );
                for identity in identities {
                    if !document_ids.contains(&identity) {
                        errors.push(format!(
                            "section {} references an undeclared option cell {identity}",
                            section.id
                        ));
                    }
                }
            }
            (Some(_), None) => errors.push(format!(
                "section {} must declare capability_cells",
                section.id
            )),
            (None, Some(_)) => errors.push(format!(
                "section {} cannot declare capability_cells without a document capability_cells set",
                section.id
            )),
            (None, None) => {}
        }
    }

    for table in &document.tables {
        if table.title.trim().is_empty() {
            errors.push(format!("table {} title must be nonempty", table.id));
        }
        require_unique_ids(
            &mut errors,
            table.columns.iter().map(|item| item.id.as_str()),
            &format!("table {} columns", table.id),
        );
        require_unique_ids(
            &mut errors,
            table.rows.iter().map(|item| item.id.as_str()),
            &format!("table {} rows", table.id),
        );
        for column in &table.columns {
            if column.label.trim().is_empty() {
                errors.push(format!(
                    "table {} column {} label must be nonempty",
                    table.id, column.id
                ));
            }
            if column.description.trim().is_empty() {
                errors.push(format!(
                    "table {} column {} description must be nonempty",
                    table.id, column.id
                ));
            }
            if column
                .default_precision
                .is_some_and(|precision| !(0..=12).contains(&precision))
            {
                errors.push(format!(
                    "table {} column {} default_precision must be an integer from 0 to 12",
                    table.id, column.id
                ));
            }
        }
        for row in &table.rows {
            if row.cells.len() != table.columns.len() {
                errors.push(format!(
                    "table {} row {} has {} cells; expected {}",
                    table.id,
                    row.id,
                    row.cells.len(),
                    table.columns.len()
                ));
                continue;
            }
            for (cell, column) in row.cells.iter().zip(&table.columns) {
                if !cell.matches_column(column.data_type) {
                    errors.push(format!(
                        "table {} row {} cell {} is {}; expected {} or missing",
                        table.id,
                        row.id,
                        column.id,
                        cell.kind_name(),
                        match column.data_type {
                            CanonicalColumnType::Number => "number",
                            CanonicalColumnType::Text => "text",
                            CanonicalColumnType::Boolean => "boolean",
                        }
                    ));
                }
                if let CanonicalResultCell::Number { value, .. } = cell {
                    if !value.is_finite() {
                        errors.push(format!(
                            "table {} row {} cell {} must be finite",
                            table.id, row.id, column.id
                        ));
                    }
                }
            }
        }
        for footnote_id in &table.footnote_ids {
            if !footnote_ids.contains(footnote_id.as_str()) {
                errors.push(format!(
                    "table {} references missing footnote {footnote_id}",
                    table.id
                ));
            }
        }
        match (&document_capability_ids, &table.capability_cells) {
            (Some(document_ids), Some(references)) => {
                let identities = validate_capability_set(
                    &mut errors,
                    references,
                    &format!("table {}.capability_cells", table.id),
                );
                for identity in identities {
                    if !document_ids.contains(&identity) {
                        errors.push(format!(
                            "table {} references an undeclared option cell {identity}",
                            table.id
                        ));
                    }
                }
            }
            (Some(_), None) => {
                errors.push(format!("table {} must declare capability_cells", table.id))
            }
            (None, Some(_)) => errors.push(format!(
                "table {} cannot declare capability_cells without a document capability_cells set",
                table.id
            )),
            (None, None) => {}
        }
    }

    if document_capability_ids.is_some() {
        let table_by_id = document
            .tables
            .iter()
            .map(|table| (table.id.as_str(), table))
            .collect::<BTreeMap<_, _>>();
        for section in &document.sections {
            let Some(section_references) = &section.capability_cells else {
                continue;
            };
            let section_capabilities = section_references
                .iter()
                .map(capability_cell_reference_identity_v2)
                .collect::<HashSet<_>>();
            let required_by_tables = section
                .table_ids
                .iter()
                .filter_map(|table_id| table_by_id.get(table_id.as_str()))
                .filter_map(|table| table.capability_cells.as_ref())
                .flatten()
                .map(capability_cell_reference_identity_v2)
                .collect::<BTreeSet<_>>();
            for identity in required_by_tables {
                if !section_capabilities.contains(&identity) {
                    errors.push(format!(
                        "section {} is missing table option cell {identity}",
                        section.id
                    ));
                }
            }
        }
    }

    for chart in &document.charts {
        if chart.title.trim().is_empty() || chart.description.trim().is_empty() {
            errors.push(format!(
                "chart {} needs a title and accessible description",
                chart.id
            ));
        }
        if let Some(source_table_id) = &chart.source_table_id {
            if !table_ids.contains(source_table_id.as_str()) {
                errors.push(format!(
                    "chart {} references missing table {source_table_id}",
                    chart.id
                ));
            }
        }
        require_unique_ids(
            &mut errors,
            chart.series.iter().map(|item| item.id.as_str()),
            &format!("chart {} series", chart.id),
        );
        for series in &chart.series {
            for (point_index, point) in series.points.iter().enumerate() {
                if let CanonicalChartX::Number(value) = point.x {
                    if !value.is_finite() {
                        errors.push(format!(
                            "chart {} series {} point {point_index} x must be finite",
                            chart.id, series.id
                        ));
                    }
                }
                for (name, value) in [
                    ("y", Some(point.y)),
                    ("lower", point.lower),
                    ("upper", point.upper),
                ] {
                    if value.is_some_and(|number| !number.is_finite()) {
                        errors.push(format!(
                            "chart {} series {} point {point_index} {name} must be finite",
                            chart.id, series.id
                        ));
                    }
                }
                if matches!((point.lower, point.upper), (Some(lower), Some(upper)) if lower > upper)
                {
                    errors.push(format!(
                        "chart {} series {} point {point_index} lower exceeds upper",
                        chart.id, series.id
                    ));
                }
            }
        }
    }

    for notice in &document.notices {
        if notice.code.trim().is_empty() || notice.message.trim().is_empty() {
            errors.push(format!(
                "notice {} code and message must be nonempty",
                notice.id
            ));
        }
        for section_id in &notice.section_ids {
            if !section_ids.contains(section_id.as_str()) {
                errors.push(format!(
                    "notice {} references missing section {section_id}",
                    notice.id
                ));
            }
        }
        for table_id in &notice.table_ids {
            if !table_ids.contains(table_id.as_str()) {
                errors.push(format!(
                    "notice {} references missing table {table_id}",
                    notice.id
                ));
            }
        }
    }

    for exclusion in &document.exclusions {
        if exclusion.title.trim().is_empty() || exclusion.reason.trim().is_empty() {
            errors.push(format!(
                "exclusion {} title and reason must be nonempty",
                exclusion.id
            ));
        }
        if let Some(capability_cell) = &exclusion.capability_cell {
            validate_capability_reference(
                &mut errors,
                capability_cell,
                &format!("exclusion {}.capability_cell", exclusion.id),
            );
        }
    }

    let provenance = &document.provenance;
    for (name, value) in [
        ("run_id", provenance.run_id.as_str()),
        ("project_id", provenance.project_id.as_str()),
        ("model_id", provenance.model_id.as_str()),
        ("dataset_id", provenance.dataset_id.as_str()),
        ("recipe_id", provenance.recipe_id.as_str()),
        ("method_version", provenance.method_version.as_str()),
        ("engine_version", provenance.engine_version.as_str()),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("provenance.{name} must be nonempty"));
        }
    }
    if !is_lowercase_sha256(&provenance.model_digest) {
        errors.push("provenance.model_digest must be lowercase SHA-256".to_string());
    }
    if !is_lowercase_sha256(&provenance.dataset_fingerprint) {
        errors.push("provenance.dataset_fingerprint must be lowercase SHA-256".to_string());
    }
    if !is_lowercase_sha256(&provenance.recipe_digest) {
        errors.push("provenance.recipe_digest must be lowercase SHA-256".to_string());
    }
    if provenance.seed.is_some_and(|seed| seed < 0) {
        errors.push("provenance.seed must be a nonnegative safe integer or null".to_string());
    } else if provenance.seed.is_some_and(|seed| seed > MAX_SAFE_INTEGER) {
        errors.push("provenance.seed must be a nonnegative safe integer or null".to_string());
    }
    if provenance.workers < 1 {
        errors.push("provenance.workers must be a positive integer".to_string());
    }
    validate_capability_reference(
        &mut errors,
        &provenance.capability_cell,
        "provenance.capability_cell",
    );
    let started_at = parse_timestamp(&provenance.started_at);
    let completed_at = parse_timestamp(&provenance.completed_at);
    if started_at.is_none() {
        errors.push("provenance.started_at must be an ISO timestamp".to_string());
    }
    if completed_at.is_none() {
        errors.push("provenance.completed_at must be an ISO timestamp".to_string());
    }
    if matches!((started_at, completed_at), (Some(started), Some(completed)) if completed < started)
    {
        errors.push("provenance.completed_at precedes started_at".to_string());
    }

    let presentation = &document.presentation;
    if presentation
        .default_section_id
        .as_deref()
        .is_some_and(|id| !section_ids.contains(id))
    {
        errors.push("presentation.default_section_id is missing".to_string());
    }
    if presentation
        .default_table_id
        .as_deref()
        .is_some_and(|id| !table_ids.contains(id))
    {
        errors.push("presentation.default_table_id is missing".to_string());
    }
    if !(0..=12).contains(&presentation.precision) {
        errors.push("presentation.precision must be an integer from 0 to 12".to_string());
    }
    if presentation.missing_value_label.trim().is_empty() {
        errors.push("presentation.missing_value_label must be nonempty".to_string());
    }

    CanonicalResultValidation {
        passed: errors.is_empty(),
        errors,
    }
}

/// Product-use boundary for the optional capability-attribution extension.
/// Historical documents remain readable but are never silently upgraded from
/// their primary capability into comparison or qualification-export evidence.
pub fn canonical_result_use_eligibility_v2(
    document: &CanonicalResultDocumentV2,
) -> CanonicalResultUseEligibilityV2 {
    let validation = validate_canonical_result_document_v2(document);
    if !validation.passed {
        return CanonicalResultUseEligibilityV2 {
            readable: false,
            comparison_eligible: false,
            qualification_export_eligible: false,
            ineligibility: Some(CanonicalResultQualificationIneligibilityV2::InvalidDocument),
        };
    }
    if document.capability_cells.is_none() {
        return CanonicalResultUseEligibilityV2 {
            readable: true,
            comparison_eligible: false,
            qualification_export_eligible: false,
            ineligibility: Some(
                CanonicalResultQualificationIneligibilityV2::LegacyCapabilityAttributionMissing,
            ),
        };
    }
    CanonicalResultUseEligibilityV2 {
        readable: true,
        comparison_eligible: true,
        qualification_export_eligible: true,
        ineligibility: None,
    }
}

fn stable_value(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.into_iter().map(stable_value).collect()),
        Value::Object(values) => {
            let sorted: BTreeMap<String, Value> = values
                .into_iter()
                .map(|(key, value)| (key, stable_value(value)))
                .collect();
            let mut object = Map::new();
            for (key, value) in sorted {
                object.insert(key, value);
            }
            Value::Object(object)
        }
        other => other,
    }
}

fn stable_json(value: Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(&stable_value(value))
}

pub fn canonical_result_document_json(
    document: &CanonicalResultDocumentV2,
) -> Result<String, serde_json::Error> {
    stable_json(serde_json::to_value(document)?)
}

/// Scientific projection for semantic equality. Execution-only worker/timing
/// fields and display caches/defaults are excluded while scientific identity,
/// ordered tables, chart data, notices, and exclusions remain bound.
pub fn canonical_analytical_result_json(
    document: &CanonicalResultDocumentV2,
) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(document)?;
    let Some(root) = value.as_object_mut() else {
        unreachable!("CanonicalResultDocumentV2 always serializes as an object");
    };
    root.remove("presentation");

    if let Some(provenance) = root.get_mut("provenance").and_then(Value::as_object_mut) {
        provenance.remove("workers");
        provenance.remove("started_at");
        provenance.remove("completed_at");
    }

    if let Some(tables) = root.get_mut("tables").and_then(Value::as_array_mut) {
        for table in tables {
            let Some(rows) = table.get_mut("rows").and_then(Value::as_array_mut) else {
                continue;
            };
            for row in rows {
                let Some(cells) = row.get_mut("cells").and_then(Value::as_array_mut) else {
                    continue;
                };
                for cell in cells {
                    if let Some(cell) = cell.as_object_mut() {
                        cell.remove("display");
                    }
                }
            }
        }
    }

    if let Some(charts) = root.get_mut("charts").and_then(Value::as_array_mut) {
        for chart in charts {
            if let Some(chart) = chart.as_object_mut() {
                chart.remove("display");
            }
        }
    }

    stable_json(value)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LegacyStringResultTable {
    pub id: String,
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LegacyResultMigrationContext {
    pub document_id: String,
    pub title: String,
    pub provenance: CanonicalResultProvenanceV2,
}

/// Preserve historical string-only result tables without interpreting formatted
/// strings as numbers. Method-specific adapters must emit typed analytical cells
/// directly for all new runs.
pub fn canonical_result_document_from_legacy_tables(
    context: LegacyResultMigrationContext,
    legacy_tables: Vec<LegacyStringResultTable>,
) -> CanonicalResultDocumentV2 {
    let tables: Vec<CanonicalResultTable> = legacy_tables
        .iter()
        .map(|table| CanonicalResultTable {
            id: table.id.clone(),
            title: table.title.clone(),
            description: Some(
                "Historical string-table result preserved without numeric reinterpretation."
                    .to_string(),
            ),
            columns: table
                .columns
                .iter()
                .enumerate()
                .map(|(index, label)| CanonicalResultColumn {
                    id: format!("column_{}", index + 1),
                    label: label.clone(),
                    data_type: CanonicalColumnType::Text,
                    description: format!("Historical column {label}"),
                    role: None,
                    unit: None,
                    default_precision: None,
                })
                .collect(),
            rows: table
                .rows
                .iter()
                .enumerate()
                .map(|(index, cells)| CanonicalResultRow {
                    id: format!("row_{}", index + 1),
                    cells: cells
                        .iter()
                        .map(|value| CanonicalResultCell::Text {
                            value: value.clone(),
                        })
                        .collect(),
                })
                .collect(),
            footnote_ids: Vec::new(),
            capability_cells: None,
        })
        .collect();

    let notices = legacy_tables
        .iter()
        .filter_map(|table| {
            table.warning.as_ref().map(|warning| CanonicalResultNotice {
                id: format!("historical_{}", table.id),
                code: "historical_string_table".to_string(),
                severity: CanonicalNoticeSeverity::Information,
                message: warning.clone(),
                section_ids: vec!["historical_results".to_string()],
                table_ids: vec![table.id.clone()],
            })
        })
        .collect();

    CanonicalResultDocumentV2 {
        schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
        document_id: context.document_id,
        title: context.title,
        provenance: context.provenance,
        capability_cells: None,
        sections: vec![CanonicalResultSection {
            id: "historical_results".to_string(),
            title: "Historical results".to_string(),
            description: None,
            table_ids: tables.iter().map(|table| table.id.clone()).collect(),
            chart_ids: Vec::new(),
            capability_cells: None,
        }],
        presentation: CanonicalResultPresentationV2 {
            default_section_id: Some("historical_results".to_string()),
            default_table_id: tables.first().map(|table| table.id.clone()),
            precision: 4,
            missing_value_label: "—".to_string(),
            chart_defaults: CanonicalChartDisplayOptions::default(),
        },
        tables,
        charts: Vec::new(),
        notices,
        exclusions: Vec::new(),
        footnotes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability_reference() -> CapabilityCellReferenceV2 {
        CapabilityCellReferenceV2 {
            registry_schema_version: 2,
            capability_id: "qpls3.pls.algorithm".to_string(),
            cell_id: "standard.reflective_recursive".to_string(),
            capability_version: "pls_algorithm_v2".to_string(),
        }
    }

    fn secondary_capability_reference() -> CapabilityCellReferenceV2 {
        CapabilityCellReferenceV2 {
            registry_schema_version: 2,
            capability_id: "smartpls.pls_bootstrapping".to_string(),
            cell_id: "qpls3.inference.bootstrap".to_string(),
            capability_version: "pls_bootstrap_v1".to_string(),
        }
    }

    fn provenance() -> CanonicalResultProvenanceV2 {
        CanonicalResultProvenanceV2 {
            run_id: "run-1".to_string(),
            project_id: "project-1".to_string(),
            model_id: "model-1".to_string(),
            model_digest: "a".repeat(64),
            dataset_id: "dataset-1".to_string(),
            dataset_fingerprint: "b".repeat(64),
            recipe_id: "recipe-1".to_string(),
            recipe_digest: "c".repeat(64),
            capability_cell: capability_reference(),
            method_version: "pls_algorithm_v2".to_string(),
            engine_version: "qpls-estimation-test".to_string(),
            seed: Some(42),
            workers: 4,
            started_at: "2026-08-14T00:00:00Z".to_string(),
            completed_at: "2026-08-14T00:00:01Z".to_string(),
        }
    }

    fn document_fixture() -> CanonicalResultDocumentV2 {
        CanonicalResultDocumentV2 {
            schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
            document_id: "result.document:1".to_string(),
            title: "PLS path results".to_string(),
            provenance: provenance(),
            capability_cells: Some(vec![capability_reference()]),
            sections: vec![CanonicalResultSection {
                id: "structural".to_string(),
                title: "Structural model".to_string(),
                description: None,
                table_ids: vec!["paths".to_string()],
                chart_ids: vec!["path_plot".to_string()],
                capability_cells: Some(vec![capability_reference()]),
            }],
            tables: vec![CanonicalResultTable {
                id: "paths".to_string(),
                title: "Path coefficients".to_string(),
                description: None,
                columns: vec![
                    CanonicalResultColumn {
                        id: "path".to_string(),
                        label: "Path".to_string(),
                        data_type: CanonicalColumnType::Text,
                        description: "Directed structural path".to_string(),
                        role: Some(CanonicalColumnRole::Label),
                        unit: None,
                        default_precision: None,
                    },
                    CanonicalResultColumn {
                        id: "estimate".to_string(),
                        label: "Estimate".to_string(),
                        data_type: CanonicalColumnType::Number,
                        description: "Standardized path estimate".to_string(),
                        role: Some(CanonicalColumnRole::Estimate),
                        unit: None,
                        default_precision: Some(4),
                    },
                ],
                rows: vec![CanonicalResultRow {
                    id: "x_to_y".to_string(),
                    cells: vec![
                        CanonicalResultCell::Text {
                            value: "X → Y".to_string(),
                        },
                        CanonicalResultCell::Number {
                            value: 0.42,
                            display: Some("0.4200".to_string()),
                        },
                    ],
                }],
                footnote_ids: vec!["standardized".to_string()],
                capability_cells: Some(vec![capability_reference()]),
            }],
            charts: vec![CanonicalResultChart {
                id: "path_plot".to_string(),
                title: "Path coefficient".to_string(),
                description: "One bar showing the standardized X to Y path coefficient."
                    .to_string(),
                kind: CanonicalChartKind::Bar,
                series: vec![CanonicalChartSeries {
                    id: "estimate".to_string(),
                    label: "Estimate".to_string(),
                    group: None,
                    points: vec![CanonicalChartPoint {
                        x: CanonicalChartX::Text("X → Y".to_string()),
                        y: 0.42,
                        lower: None,
                        upper: None,
                        label: None,
                    }],
                }],
                source_table_id: Some("paths".to_string()),
                display: CanonicalChartDisplayOptions {
                    palette: Some("institutional_navy".to_string()),
                    show_values: Some(true),
                    ..CanonicalChartDisplayOptions::default()
                },
            }],
            notices: Vec::new(),
            exclusions: Vec::new(),
            footnotes: vec![CanonicalResultFootnote {
                id: "standardized".to_string(),
                text: "Standardized estimates.".to_string(),
                reference: None,
            }],
            presentation: CanonicalResultPresentationV2 {
                default_section_id: Some("structural".to_string()),
                default_table_id: Some("paths".to_string()),
                precision: 4,
                missing_value_label: "—".to_string(),
                chart_defaults: CanonicalChartDisplayOptions {
                    show_legend: Some(true),
                    ..CanonicalChartDisplayOptions::default()
                },
            },
        }
    }

    #[test]
    fn valid_microcase_passes() {
        let validation = validate_canonical_result_document_v2(&document_fixture());
        assert!(validation.passed, "{:?}", validation.errors);
        assert!(validation.errors.is_empty());
        assert_eq!(
            canonical_result_use_eligibility_v2(&document_fixture()),
            CanonicalResultUseEligibilityV2 {
                readable: true,
                comparison_eligible: true,
                qualification_export_eligible: true,
                ineligibility: None,
            }
        );
    }

    #[test]
    fn multi_capability_sets_roundtrip_with_primary_and_table_attribution() {
        let mut document = document_fixture();
        let capabilities = vec![capability_reference(), secondary_capability_reference()];
        document.capability_cells = Some(capabilities.clone());
        document.sections[0].capability_cells = Some(capabilities.clone());
        document.tables[0].capability_cells = Some(capabilities);

        let validation = validate_canonical_result_document_v2(&document);
        assert!(validation.passed, "{:?}", validation.errors);
        let encoded = serde_json::to_vec(&document).unwrap();
        let decoded: CanonicalResultDocumentV2 = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, document);
        assert_eq!(
            canonical_result_use_eligibility_v2(&decoded),
            CanonicalResultUseEligibilityV2 {
                readable: true,
                comparison_eligible: true,
                qualification_export_eligible: true,
                ineligibility: None,
            }
        );
    }

    #[test]
    fn capability_set_order_duplicates_primary_and_cross_level_tampering_fail_closed() {
        let primary = capability_reference();
        let secondary = secondary_capability_reference();
        let mut unsorted = document_fixture();
        unsorted.capability_cells = Some(vec![secondary.clone(), primary.clone()]);
        unsorted.sections[0].capability_cells = Some(vec![primary.clone(), secondary.clone()]);
        unsorted.tables[0].capability_cells = Some(vec![primary.clone(), secondary.clone()]);
        assert!(
            validate_canonical_result_document_v2(&unsorted)
                .errors
                .iter()
                .any(|error| error.contains("must be ordered by exact option-cell identity"))
        );

        let mut duplicate = document_fixture();
        duplicate.capability_cells = Some(vec![primary.clone(), primary.clone()]);
        assert!(
            validate_canonical_result_document_v2(&duplicate)
                .errors
                .iter()
                .any(|error| error.contains("contains duplicate references"))
        );

        let mut missing_primary = document_fixture();
        missing_primary.capability_cells = Some(vec![secondary.clone()]);
        missing_primary.sections[0].capability_cells = Some(vec![secondary.clone()]);
        missing_primary.tables[0].capability_cells = Some(vec![secondary.clone()]);
        assert!(
            validate_canonical_result_document_v2(&missing_primary)
                .errors
                .iter()
                .any(|error| error.contains("must include provenance.capability_cell"))
        );

        let mut undeclared_table = document_fixture();
        undeclared_table.tables[0].capability_cells = Some(vec![secondary.clone()]);
        assert!(
            validate_canonical_result_document_v2(&undeclared_table)
                .errors
                .iter()
                .any(|error| error.contains("table paths references an undeclared option cell"))
        );

        let mut incomplete_section = document_fixture();
        incomplete_section.capability_cells = Some(vec![primary.clone(), secondary.clone()]);
        incomplete_section.tables[0].capability_cells =
            Some(vec![primary.clone(), secondary.clone()]);
        incomplete_section.sections[0].capability_cells = Some(vec![primary]);
        assert!(
            validate_canonical_result_document_v2(&incomplete_section)
                .errors
                .iter()
                .any(|error| error.contains("section structural is missing table option cell"))
        );

        let mut partial_legacy = document_fixture();
        partial_legacy.capability_cells = None;
        assert!(
            validate_canonical_result_document_v2(&partial_legacy)
                .errors
                .iter()
                .any(|error| error.contains("cannot declare capability_cells without a document"))
        );
    }

    #[test]
    fn capability_sets_are_part_of_analytical_identity() {
        let single = document_fixture();
        let mut multiple = document_fixture();
        let capabilities = vec![capability_reference(), secondary_capability_reference()];
        multiple.capability_cells = Some(capabilities.clone());
        multiple.sections[0].capability_cells = Some(capabilities.clone());
        multiple.tables[0].capability_cells = Some(capabilities);
        assert_ne!(
            canonical_analytical_result_json(&single).unwrap(),
            canonical_analytical_result_json(&multiple).unwrap()
        );
    }

    #[test]
    fn duplicate_dangling_nonfinite_type_and_row_errors_fail_closed() {
        let mut document = document_fixture();
        document.sections.push(document.sections[0].clone());
        document.tables[0].rows[0].cells = vec![CanonicalResultCell::Number {
            value: f64::NAN,
            display: None,
        }];
        document.tables[0].rows.push(CanonicalResultRow {
            id: "wrong_type".to_string(),
            cells: vec![
                CanonicalResultCell::Boolean { value: true },
                CanonicalResultCell::Number {
                    value: f64::INFINITY,
                    display: None,
                },
            ],
        });
        document.charts[0].source_table_id = Some("missing_table".to_string());
        document.presentation.default_section_id = Some("missing_section".to_string());

        let validation = validate_canonical_result_document_v2(&document);
        assert!(!validation.passed);
        for expected in [
            "sections contains duplicate IDs",
            "has 1 cells; expected 2",
            "is boolean; expected text or missing",
            "must be finite",
            "references missing table missing_table",
            "default_section_id is missing",
        ] {
            assert!(
                validation
                    .errors
                    .iter()
                    .any(|error| error.contains(expected)),
                "missing {expected:?} in {:?}",
                validation.errors
            );
        }
    }

    #[test]
    fn canonical_json_is_deterministic_for_object_key_order() {
        let first = document_fixture();
        let first_json = serde_json::to_string(&first).unwrap();
        let value = serde_json::from_str::<Value>(&first_json).unwrap();
        let reordered = stable_value(value);
        let second: CanonicalResultDocumentV2 = serde_json::from_value(reordered).unwrap();

        assert_eq!(
            canonical_result_document_json(&first).unwrap(),
            canonical_result_document_json(&second).unwrap()
        );
    }

    #[test]
    fn analytical_projection_ignores_presentation_workers_timing_and_display_caches() {
        let first = document_fixture();
        let mut second = document_fixture();
        second.presentation.precision = 6;
        second.presentation.chart_defaults.palette = Some("high_contrast".to_string());
        second.tables[0].rows[0].cells[1] = CanonicalResultCell::Number {
            value: 0.42,
            display: Some("0.420000".to_string()),
        };
        second.charts[0].display.palette = Some("journal_mono".to_string());
        second.provenance.workers = 1;
        second.provenance.completed_at = "2026-08-14T00:00:09Z".to_string();

        assert_ne!(
            canonical_result_document_json(&first).unwrap(),
            canonical_result_document_json(&second).unwrap()
        );
        assert_eq!(
            canonical_analytical_result_json(&first).unwrap(),
            canonical_analytical_result_json(&second).unwrap()
        );
    }

    #[test]
    fn analytical_projection_changes_with_analytical_values_and_order() {
        let first = document_fixture();
        let mut changed_value = document_fixture();
        changed_value.tables[0].rows[0].cells[1] = CanonicalResultCell::Number {
            value: 0.43,
            display: Some("0.4300".to_string()),
        };
        assert_ne!(
            canonical_analytical_result_json(&first).unwrap(),
            canonical_analytical_result_json(&changed_value).unwrap()
        );

        let mut changed_order = document_fixture();
        let row = changed_order.tables[0].rows[0].clone();
        changed_order.tables[0].rows.push(CanonicalResultRow {
            id: "z_to_y".to_string(),
            ..row
        });
        let before = canonical_analytical_result_json(&changed_order).unwrap();
        changed_order.tables[0].rows.reverse();
        assert_ne!(
            before,
            canonical_analytical_result_json(&changed_order).unwrap()
        );
    }

    #[test]
    fn historical_string_tables_migrate_losslessly_without_numeric_inference() {
        let migrated = canonical_result_document_from_legacy_tables(
            LegacyResultMigrationContext {
                document_id: "historical.result:1".to_string(),
                title: "Historical result".to_string(),
                provenance: provenance(),
            },
            vec![LegacyStringResultTable {
                id: "legacy_paths".to_string(),
                title: "Paths".to_string(),
                columns: vec!["Path".to_string(), "Estimate".to_string()],
                rows: vec![vec!["X → Y".to_string(), "0.4200".to_string()]],
                warning: Some(
                    "This result was created by a historical method version.".to_string(),
                ),
            }],
        );

        let validation = validate_canonical_result_document_v2(&migrated);
        assert!(validation.passed, "{:?}", validation.errors);
        assert_eq!(
            migrated.tables[0].rows[0].cells[1],
            CanonicalResultCell::Text {
                value: "0.4200".to_string()
            }
        );
        assert_eq!(migrated.notices.len(), 1);
        assert!(migrated.capability_cells.is_none());
        assert!(migrated.sections[0].capability_cells.is_none());
        assert!(migrated.tables[0].capability_cells.is_none());
        assert_eq!(
            canonical_result_use_eligibility_v2(&migrated),
            CanonicalResultUseEligibilityV2 {
                readable: true,
                comparison_eligible: false,
                qualification_export_eligible: false,
                ineligibility: Some(
                    CanonicalResultQualificationIneligibilityV2::LegacyCapabilityAttributionMissing
                ),
            }
        );
    }

    #[test]
    fn serde_rejects_unknown_fields() {
        let mut value = serde_json::to_value(document_fixture()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected".to_string(), Value::Bool(true));
        assert!(serde_json::from_value::<CanonicalResultDocumentV2>(value).is_err());
    }

    #[test]
    fn provenance_constraints_fail_closed() {
        let mut document = document_fixture();
        document.provenance.recipe_digest = "ABC".to_string();
        document.provenance.workers = 0;
        document.provenance.seed = Some(MAX_SAFE_INTEGER + 1);
        document.provenance.completed_at = "2026-08-13T23:59:59Z".to_string();
        document.presentation.precision = 13;

        let validation = validate_canonical_result_document_v2(&document);
        assert!(!validation.passed);
        for expected in [
            "recipe_digest must be lowercase SHA-256",
            "workers must be a positive integer",
            "seed must be a nonnegative safe integer or null",
            "completed_at precedes started_at",
            "precision must be an integer from 0 to 12",
        ] {
            assert!(
                validation
                    .errors
                    .iter()
                    .any(|error| error.contains(expected)),
                "missing {expected:?} in {:?}",
                validation.errors
            );
        }
    }
}
