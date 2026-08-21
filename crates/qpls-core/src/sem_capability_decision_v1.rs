use serde::{Deserialize, Deserializer, Serialize, de};
use std::{cmp::Ordering, collections::BTreeSet};
use unicode_normalization::UnicodeNormalization;

pub const SEM_CAPABILITY_DECISION_V1_SCHEMA_VERSION: u32 = 1;
pub const SEM_CAPABILITY_DIAGNOSTIC_CODE_PREFIX_V1: &str = "sem.capability.";

/// Aggregate availability of one exact estimator for one exact scientific request.
///
/// `Supported` and `Experimental` are runnable decisions. `Blocked` is not.
/// Estimator integration remains outside this schema-only contract.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SemCapabilityDecisionStatusV1 {
    Supported,
    Experimental,
    Blocked,
}

impl SemCapabilityDecisionStatusV1 {
    pub fn accessible_label(self) -> &'static str {
        match self {
            Self::Supported => "Supported",
            Self::Experimental => "Experimental",
            Self::Blocked => "Blocked",
        }
    }

    fn is_runnable(self) -> bool {
        self != Self::Blocked
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SemCapabilityDiagnosticSeverityV1 {
    Error,
    Warning,
    Info,
}

impl SemCapabilityDiagnosticSeverityV1 {
    fn canonical_rank(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
        }
    }
}

/// Open, namespaced diagnostic code. The newtype prevents arbitrary display
/// prose from being mistaken for a machine-actionable diagnostic identity.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct SemCapabilityDiagnosticCodeV1(String);

impl SemCapabilityDiagnosticCodeV1 {
    pub fn new(value: impl Into<String>) -> Result<Self, SemCapabilityDecisionV1ValidationError> {
        let code = Self(value.into());
        code.ensure_valid("diagnostic.code")?;
        Ok(code)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn ensure_valid(&self, context: &str) -> Result<(), SemCapabilityDecisionV1ValidationError> {
        validate_text(context.to_string(), &self.0)?;
        if !self.0.starts_with(SEM_CAPABILITY_DIAGNOSTIC_CODE_PREFIX_V1)
            || self.0.len() == SEM_CAPABILITY_DIAGNOSTIC_CODE_PREFIX_V1.len()
        {
            return Err(
                SemCapabilityDecisionV1ValidationError::DiagnosticCodeNamespaceInvalid {
                    code: self.0.clone(),
                },
            );
        }
        Ok(())
    }
}

impl std::fmt::Display for SemCapabilityDiagnosticCodeV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SemCapabilityDiagnosticCodeV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Exact Capability Registry cell identity. `cell_id` alone is deliberately
/// insufficient because catalogue rows may share it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SemCapabilityCellIdV1 {
    registry_schema_version: u32,
    capability_id: String,
    cell_id: String,
    capability_version: String,
}

impl SemCapabilityCellIdV1 {
    pub fn new(
        registry_schema_version: u32,
        capability_id: impl Into<String>,
        cell_id: impl Into<String>,
        capability_version: impl Into<String>,
    ) -> Result<Self, SemCapabilityDecisionV1ValidationError> {
        let cell = Self {
            registry_schema_version,
            capability_id: capability_id.into(),
            cell_id: cell_id.into(),
            capability_version: capability_version.into(),
        };
        cell.ensure_valid("capability_cell")?;
        Ok(cell)
    }

    pub fn registry_schema_version(&self) -> u32 {
        self.registry_schema_version
    }

    pub fn capability_id(&self) -> &str {
        &self.capability_id
    }

    pub fn cell_id(&self) -> &str {
        &self.cell_id
    }

    pub fn capability_version(&self) -> &str {
        &self.capability_version
    }

    fn ensure_valid(&self, context: &str) -> Result<(), SemCapabilityDecisionV1ValidationError> {
        if self.registry_schema_version == 0 {
            return Err(
                SemCapabilityDecisionV1ValidationError::RegistrySchemaVersionZero {
                    context: format!("{context}.registry_schema_version"),
                },
            );
        }
        validate_text(format!("{context}.capability_id"), &self.capability_id)?;
        validate_text(format!("{context}.cell_id"), &self.cell_id)?;
        validate_text(
            format!("{context}.capability_version"),
            &self.capability_version,
        )
    }

    fn canonical_cmp(&self, other: &Self) -> Ordering {
        self.registry_schema_version
            .cmp(&other.registry_schema_version)
            .then_with(|| self.capability_id.cmp(&other.capability_id))
            .then_with(|| self.cell_id.cmp(&other.cell_id))
            .then_with(|| self.capability_version.cmp(&other.capability_version))
    }

    fn readable_identity(&self) -> String {
        format!(
            "{}::{}::{}::{}",
            self.registry_schema_version, self.capability_id, self.cell_id, self.capability_version
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SemCapabilityDiagnosticV1 {
    code: SemCapabilityDiagnosticCodeV1,
    severity: SemCapabilityDiagnosticSeverityV1,
    subject: Option<String>,
    message: String,
    corrections: Vec<String>,
}

impl SemCapabilityDiagnosticV1 {
    pub fn new(
        code: impl Into<String>,
        severity: SemCapabilityDiagnosticSeverityV1,
        subject: Option<String>,
        message: impl Into<String>,
        corrections: Vec<String>,
    ) -> Result<Self, SemCapabilityDecisionV1ValidationError> {
        let mut diagnostic = Self {
            code: SemCapabilityDiagnosticCodeV1::new(code)?,
            severity,
            subject,
            message: message.into(),
            corrections,
        };
        diagnostic.canonicalize();
        diagnostic.ensure_valid("diagnostic")?;
        Ok(diagnostic)
    }

    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    pub fn severity(&self) -> SemCapabilityDiagnosticSeverityV1 {
        self.severity
    }

    pub fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn corrections(&self) -> &[String] {
        &self.corrections
    }

    fn canonicalize(&mut self) {
        self.corrections.sort();
    }

    fn ensure_valid(&self, context: &str) -> Result<(), SemCapabilityDecisionV1ValidationError> {
        self.code.ensure_valid(&format!("{context}.code"))?;
        if let Some(subject) = &self.subject {
            validate_text(format!("{context}.subject"), subject)?;
        }
        validate_text(format!("{context}.message"), &self.message)?;
        if self.severity != SemCapabilityDiagnosticSeverityV1::Info && self.corrections.is_empty() {
            return Err(
                SemCapabilityDecisionV1ValidationError::ActionableCorrectionRequired {
                    diagnostic_code: self.code.as_str().to_string(),
                },
            );
        }
        let mut previous: Option<&str> = None;
        for (index, correction) in self.corrections.iter().enumerate() {
            validate_text(format!("{context}.corrections[{index}]"), correction)?;
            if previous == Some(correction) {
                return Err(
                    SemCapabilityDecisionV1ValidationError::DuplicateCorrection {
                        diagnostic_code: self.code.as_str().to_string(),
                        correction: correction.clone(),
                    },
                );
            }
            previous = Some(correction);
        }
        Ok(())
    }

    fn canonical_cmp(&self, other: &Self) -> Ordering {
        self.severity
            .canonical_rank()
            .cmp(&other.severity.canonical_rank())
            .then_with(|| self.code.cmp(&other.code))
            .then_with(|| self.subject.cmp(&other.subject))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SemCapabilityEvidenceV1 {
    evidence_id: String,
    description: String,
}

impl SemCapabilityEvidenceV1 {
    pub fn new(
        evidence_id: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, SemCapabilityDecisionV1ValidationError> {
        let evidence = Self {
            evidence_id: evidence_id.into(),
            description: description.into(),
        };
        evidence.ensure_valid("evidence")?;
        Ok(evidence)
    }

    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    fn ensure_valid(&self, context: &str) -> Result<(), SemCapabilityDecisionV1ValidationError> {
        validate_text(format!("{context}.evidence_id"), &self.evidence_id)?;
        validate_text(format!("{context}.description"), &self.description)
    }
}

/// Versioned, estimator-specific capability decision with complete provenance.
///
/// All status communication has an exact textual label and explanatory message;
/// consumers never need color alone to distinguish supported, experimental, or
/// blocked decisions. Collections are canonicalized during construction and
/// deserialization, so source declaration order cannot change serialized identity.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SemCapabilityDecisionV1 {
    schema_version: u32,
    status: SemCapabilityDecisionStatusV1,
    status_label: String,
    estimator_id: String,
    capability_cells: Vec<SemCapabilityCellIdV1>,
    diagnostics: Vec<SemCapabilityDiagnosticV1>,
    evidence: Vec<SemCapabilityEvidenceV1>,
    summary: String,
    explanation: String,
}

impl SemCapabilityDecisionV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        status: SemCapabilityDecisionStatusV1,
        estimator_id: impl Into<String>,
        capability_cells: Vec<SemCapabilityCellIdV1>,
        diagnostics: Vec<SemCapabilityDiagnosticV1>,
        evidence: Vec<SemCapabilityEvidenceV1>,
        summary: impl Into<String>,
        explanation: impl Into<String>,
    ) -> Result<Self, SemCapabilityDecisionV1ValidationError> {
        let mut decision = Self {
            schema_version: SEM_CAPABILITY_DECISION_V1_SCHEMA_VERSION,
            status,
            status_label: status.accessible_label().to_string(),
            estimator_id: estimator_id.into(),
            capability_cells,
            diagnostics,
            evidence,
            summary: summary.into(),
            explanation: explanation.into(),
        };
        decision.canonicalize();
        decision.ensure_valid()?;
        Ok(decision)
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn status(&self) -> SemCapabilityDecisionStatusV1 {
        self.status
    }

    pub fn status_label(&self) -> &str {
        &self.status_label
    }

    pub fn estimator_id(&self) -> &str {
        &self.estimator_id
    }

    pub fn capability_cells(&self) -> &[SemCapabilityCellIdV1] {
        &self.capability_cells
    }

    pub fn diagnostics(&self) -> &[SemCapabilityDiagnosticV1] {
        &self.diagnostics
    }

    pub fn evidence(&self) -> &[SemCapabilityEvidenceV1] {
        &self.evidence
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn explanation(&self) -> &str {
        &self.explanation
    }

    pub fn ensure_valid(&self) -> Result<(), SemCapabilityDecisionV1ValidationError> {
        if self.schema_version != SEM_CAPABILITY_DECISION_V1_SCHEMA_VERSION {
            return Err(SemCapabilityDecisionV1ValidationError::SchemaVersion {
                found: self.schema_version,
            });
        }
        validate_text("decision.estimator_id".into(), &self.estimator_id)?;
        if self.status_label != self.status.accessible_label() {
            return Err(
                SemCapabilityDecisionV1ValidationError::StatusLabelMismatch {
                    status: self.status,
                    expected: self.status.accessible_label(),
                    found: self.status_label.clone(),
                },
            );
        }
        validate_text("decision.summary".into(), &self.summary)?;
        validate_text("decision.explanation".into(), &self.explanation)?;

        if self.capability_cells.is_empty() {
            return Err(SemCapabilityDecisionV1ValidationError::CapabilityCellsEmpty);
        }
        let mut previous_cell: Option<&SemCapabilityCellIdV1> = None;
        for (index, cell) in self.capability_cells.iter().enumerate() {
            cell.ensure_valid(&format!("decision.capability_cells[{index}]"))?;
            if previous_cell == Some(cell) {
                return Err(
                    SemCapabilityDecisionV1ValidationError::DuplicateCapabilityCell {
                        identity: cell.readable_identity(),
                    },
                );
            }
            previous_cell = Some(cell);
        }

        let mut diagnostic_identities = BTreeSet::new();
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            diagnostic.ensure_valid(&format!("decision.diagnostics[{index}]"))?;
            if !diagnostic_identities.insert((diagnostic.code.clone(), diagnostic.subject.clone()))
            {
                return Err(
                    SemCapabilityDecisionV1ValidationError::DuplicateDiagnostic {
                        code: diagnostic.code.as_str().to_string(),
                        subject: diagnostic.subject.clone(),
                    },
                );
            }
        }

        if self.evidence.is_empty() {
            return Err(SemCapabilityDecisionV1ValidationError::EvidenceEmpty);
        }
        let mut previous_evidence_id: Option<&str> = None;
        for (index, item) in self.evidence.iter().enumerate() {
            item.ensure_valid(&format!("decision.evidence[{index}]"))?;
            if previous_evidence_id == Some(item.evidence_id.as_str()) {
                return Err(SemCapabilityDecisionV1ValidationError::DuplicateEvidence {
                    evidence_id: item.evidence_id.clone(),
                });
            }
            previous_evidence_id = Some(&item.evidence_id);
        }

        let first_error = self
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == SemCapabilityDiagnosticSeverityV1::Error);
        if self.status.is_runnable()
            && let Some(diagnostic) = first_error
        {
            return Err(
                SemCapabilityDecisionV1ValidationError::RunnableStatusHasBlockingDiagnostic {
                    status: self.status,
                    diagnostic_code: diagnostic.code.as_str().to_string(),
                },
            );
        }
        if self.status == SemCapabilityDecisionStatusV1::Blocked && first_error.is_none() {
            return Err(SemCapabilityDecisionV1ValidationError::BlockedWithoutBlockingDiagnostic);
        }
        Ok(())
    }

    fn canonicalize(&mut self) {
        self.capability_cells
            .sort_by(SemCapabilityCellIdV1::canonical_cmp);
        for diagnostic in &mut self.diagnostics {
            diagnostic.canonicalize();
        }
        self.diagnostics
            .sort_by(SemCapabilityDiagnosticV1::canonical_cmp);
        self.evidence
            .sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SemCapabilityDecisionWireV1 {
    schema_version: u32,
    status: SemCapabilityDecisionStatusV1,
    status_label: String,
    estimator_id: String,
    capability_cells: Vec<SemCapabilityCellIdV1>,
    diagnostics: Vec<SemCapabilityDiagnosticV1>,
    evidence: Vec<SemCapabilityEvidenceV1>,
    summary: String,
    explanation: String,
}

impl<'de> Deserialize<'de> for SemCapabilityDecisionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SemCapabilityDecisionWireV1::deserialize(deserializer)?;
        let mut decision = Self {
            schema_version: wire.schema_version,
            status: wire.status,
            status_label: wire.status_label,
            estimator_id: wire.estimator_id,
            capability_cells: wire.capability_cells,
            diagnostics: wire.diagnostics,
            evidence: wire.evidence,
            summary: wire.summary,
            explanation: wire.explanation,
        };
        decision.canonicalize();
        decision.ensure_valid().map_err(de::Error::custom)?;
        Ok(decision)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum SemCapabilityDecisionV1ValidationError {
    #[error("SEM capability decision v1 requires schema_version 1 (found {found})")]
    SchemaVersion { found: u32 },
    #[error("registry schema version at {context} must be greater than zero")]
    RegistrySchemaVersionZero { context: String },
    #[error("text at {context} cannot be empty")]
    EmptyText { context: String },
    #[error("text at {context} cannot contain surrounding whitespace")]
    TextHasSurroundingWhitespace { context: String },
    #[error("text at {context} cannot contain control characters")]
    TextContainsControlCharacter { context: String },
    #[error("text at {context} must use Unicode NFC normalization")]
    TextNotNfc { context: String },
    #[error("diagnostic code {code} must use the sem.capability. namespace")]
    DiagnosticCodeNamespaceInvalid { code: String },
    #[error("a capability decision requires at least one exact capability cell")]
    CapabilityCellsEmpty,
    #[error("capability cell {identity} is duplicated")]
    DuplicateCapabilityCell { identity: String },
    #[error("diagnostic {code} for subject {subject:?} is duplicated")]
    DuplicateDiagnostic {
        code: String,
        subject: Option<String>,
    },
    #[error("warning or error diagnostic {diagnostic_code} requires an actionable correction")]
    ActionableCorrectionRequired { diagnostic_code: String },
    #[error("diagnostic {diagnostic_code} repeats correction {correction}")]
    DuplicateCorrection {
        diagnostic_code: String,
        correction: String,
    },
    #[error("a capability decision requires at least one evidence item")]
    EvidenceEmpty,
    #[error("evidence item {evidence_id} is duplicated")]
    DuplicateEvidence { evidence_id: String },
    #[error("status {status:?} requires accessible label {expected}, found {found}")]
    StatusLabelMismatch {
        status: SemCapabilityDecisionStatusV1,
        expected: &'static str,
        found: String,
    },
    #[error("runnable status {status:?} contradicts blocking diagnostic {diagnostic_code}")]
    RunnableStatusHasBlockingDiagnostic {
        status: SemCapabilityDecisionStatusV1,
        diagnostic_code: String,
    },
    #[error("blocked status requires at least one error diagnostic")]
    BlockedWithoutBlockingDiagnostic,
}

fn validate_text(
    context: String,
    value: &str,
) -> Result<(), SemCapabilityDecisionV1ValidationError> {
    if value.trim().is_empty() {
        return Err(SemCapabilityDecisionV1ValidationError::EmptyText { context });
    }
    if value.trim() != value {
        return Err(
            SemCapabilityDecisionV1ValidationError::TextHasSurroundingWhitespace { context },
        );
    }
    if value.chars().any(char::is_control) {
        return Err(
            SemCapabilityDecisionV1ValidationError::TextContainsControlCharacter { context },
        );
    }
    if value.nfc().collect::<String>() != value {
        return Err(SemCapabilityDecisionV1ValidationError::TextNotNfc { context });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn blocked_wire() -> Value {
        json!({
            "schema_version": 1,
            "status": "blocked",
            "status_label": "Blocked",
            "estimator_id": "estimator:general_sem_ml_v1",
            "capability_cells": [
                {
                    "registry_schema_version": 2,
                    "capability_id": "smartpls.cbsem_moderator",
                    "cell_id": "qpls3.cbsem.moderator.product_indicator",
                    "capability_version": "cbsem_product_indicator_v1"
                },
                {
                    "registry_schema_version": 2,
                    "capability_id": "smartpls.cbsem",
                    "cell_id": "qpls3.cbsem.ml",
                    "capability_version": "cbsem_ml_v1"
                }
            ],
            "diagnostics": [
                {
                    "code": "sem.capability.qualification_missing",
                    "severity": "warning",
                    "subject": null,
                    "message": "Qualification evidence is incomplete.",
                    "corrections": ["Run the registered qualification suite."]
                },
                {
                    "code": "sem.capability.interaction_order_unsupported",
                    "severity": "error",
                    "subject": "term:x:m:w",
                    "message": "This estimator cannot execute the authored three-way interaction.",
                    "corrections": [
                        "Reduce the interaction to two operands.",
                        "Choose an estimator that supports three-way interactions."
                    ]
                }
            ],
            "evidence": [
                {
                    "evidence_id": "qualification:moderation:v1",
                    "description": "The moderation qualification covers two-way interactions only."
                },
                {
                    "evidence_id": "compiler:general_sem:v1",
                    "description": "The compiler preserved all authored operands."
                }
            ],
            "summary": "Blocked: the selected estimator cannot execute this model.",
            "explanation": "The scientific model is preserved, but execution requires a qualified estimator for every exact capability cell."
        })
    }

    #[test]
    fn strict_wire_round_trip_is_canonical_and_textual() {
        let first: SemCapabilityDecisionV1 = serde_json::from_value(blocked_wire()).unwrap();
        assert_eq!(first.status(), SemCapabilityDecisionStatusV1::Blocked);
        assert_eq!(first.status_label(), "Blocked");
        assert!(first.summary().starts_with("Blocked:"));
        assert_eq!(
            first.capability_cells()[0].capability_id(),
            "smartpls.cbsem"
        );
        assert_eq!(
            first.diagnostics()[0].severity(),
            SemCapabilityDiagnosticSeverityV1::Error
        );
        assert_eq!(
            first.diagnostics()[0].corrections(),
            &[
                "Choose an estimator that supports three-way interactions.".to_string(),
                "Reduce the interaction to two operands.".to_string(),
            ]
        );
        assert_eq!(first.evidence()[0].evidence_id(), "compiler:general_sem:v1");

        let encoded = serde_json::to_vec(&first).unwrap();
        let second: SemCapabilityDecisionV1 = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(second, first);
        assert_eq!(serde_json::to_vec(&second).unwrap(), encoded);
    }

    #[test]
    fn serde_denies_unknown_fields_and_invalid_enums_at_every_boundary() {
        let mut top = blocked_wire();
        top["unexpected"] = json!(true);
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(top)
                .unwrap_err()
                .to_string()
                .contains("unknown field")
        );

        let mut nested = blocked_wire();
        nested["diagnostics"][0]["color"] = json!("red");
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(nested)
                .unwrap_err()
                .to_string()
                .contains("unknown field")
        );

        let mut status = blocked_wire();
        status["status"] = json!("available");
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(status)
                .unwrap_err()
                .to_string()
                .contains("unknown variant")
        );

        let mut severity = blocked_wire();
        severity["diagnostics"][0]["severity"] = json!("danger");
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(severity)
                .unwrap_err()
                .to_string()
                .contains("unknown variant")
        );

        let mut code = blocked_wire();
        code["diagnostics"][0]["code"] = json!("qualification_missing");
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(code)
                .unwrap_err()
                .to_string()
                .contains("sem.capability. namespace")
        );
    }

    #[test]
    fn status_and_blocking_diagnostics_cannot_contradict() {
        for status in ["supported", "experimental"] {
            let mut runnable = blocked_wire();
            runnable["status"] = json!(status);
            runnable["status_label"] = json!(if status == "supported" {
                "Supported"
            } else {
                "Experimental"
            });
            assert!(
                serde_json::from_value::<SemCapabilityDecisionV1>(runnable)
                    .unwrap_err()
                    .to_string()
                    .contains("contradicts blocking diagnostic")
            );
        }

        let mut blocked_without_error = blocked_wire();
        blocked_without_error["diagnostics"][1]["severity"] = json!("warning");
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(blocked_without_error)
                .unwrap_err()
                .to_string()
                .contains("blocked status requires at least one error")
        );
    }

    #[test]
    fn exact_cells_evidence_and_actionable_diagnostics_are_required_and_unique() {
        let mut no_cells = blocked_wire();
        no_cells["capability_cells"] = json!([]);
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(no_cells)
                .unwrap_err()
                .to_string()
                .contains("at least one exact capability cell")
        );

        let mut duplicate_cell = blocked_wire();
        let first_cell = duplicate_cell["capability_cells"][0].clone();
        duplicate_cell["capability_cells"][1] = first_cell;
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(duplicate_cell)
                .unwrap_err()
                .to_string()
                .contains("is duplicated")
        );

        let mut no_evidence = blocked_wire();
        no_evidence["evidence"] = json!([]);
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(no_evidence)
                .unwrap_err()
                .to_string()
                .contains("at least one evidence item")
        );

        let mut no_correction = blocked_wire();
        no_correction["diagnostics"][1]["corrections"] = json!([]);
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(no_correction)
                .unwrap_err()
                .to_string()
                .contains("requires an actionable correction")
        );
    }

    #[test]
    fn accessible_status_and_message_fields_are_strict() {
        let mut wrong_label = blocked_wire();
        wrong_label["status_label"] = json!("Unavailable");
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(wrong_label)
                .unwrap_err()
                .to_string()
                .contains("requires accessible label Blocked")
        );

        let mut blank_summary = blocked_wire();
        blank_summary["summary"] = json!("   ");
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(blank_summary)
                .unwrap_err()
                .to_string()
                .contains("decision.summary")
        );

        let mut blank_message = blocked_wire();
        blank_message["diagnostics"][0]["message"] = json!("");
        assert!(
            serde_json::from_value::<SemCapabilityDecisionV1>(blank_message)
                .unwrap_err()
                .to_string()
                .contains("diagnostics[1].message")
        );
    }

    #[test]
    fn constructors_return_typed_errors_and_canonicalize_collections() {
        let error = SemCapabilityDiagnosticV1::new(
            "sem.capability.blocked",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            "Execution is blocked.",
            vec!["Choose another estimator.".into()],
        )
        .unwrap();
        let evidence = SemCapabilityEvidenceV1::new(
            "evidence:blocked:v1",
            "No qualified execution path exists.",
        )
        .unwrap();
        let cell = SemCapabilityCellIdV1::new(
            2,
            "smartpls.general_sem",
            "qpls3.sem.general",
            "general_sem_v1",
        )
        .unwrap();
        assert_eq!(
            SemCapabilityDecisionV1::new(
                SemCapabilityDecisionStatusV1::Supported,
                "estimator:general_sem_ml_v1",
                vec![cell],
                vec![error],
                vec![evidence],
                "Supported with a contradiction.",
                "This fixture must fail before it can be serialized.",
            ),
            Err(
                SemCapabilityDecisionV1ValidationError::RunnableStatusHasBlockingDiagnostic {
                    status: SemCapabilityDecisionStatusV1::Supported,
                    diagnostic_code: "sem.capability.blocked".into(),
                }
            )
        );
    }
}
