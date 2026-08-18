//! Stateless Standard-workspace SemModelV4 document-digest compare-and-swap.
//!
//! This command validates and canonicalizes detached model documents only. It
//! has no graph, schema-5 project, active-project, save, or persistence access.

use qpls_core::{SemModelV4, SemModelV4Issue};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CAS_RESULT_SCHEMA_VERSION: u32 = 1;
const LOWER_SHA256_LENGTH: usize = 64;
const COMPLETE_MODEL_FIELDS: [&str; 12] = [
    "schema_version",
    "id",
    "name",
    "variables",
    "relations",
    "parameters",
    "constraints",
    "derived_terms",
    "group",
    "data_binding",
    "annotations",
    "presentation",
];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StandardSemModelV4AuthorityCasRequestV1 {
    expected_source_model_document_sha256: String,
    source_model: Value,
    candidate: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StandardSemModelV4AuthorityResolveRequestV1 {
    model: Value,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum StandardSemModelV4ReadinessV1 {
    Ready,
    AuthoringOnly,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StandardSemModelV4AuthorityCasResultV1 {
    schema_version: u32,
    source_model_document_sha256: String,
    canonical_candidate: SemModelV4,
    candidate_model_document_sha256: String,
    candidate_scientific_sha256: Option<String>,
    readiness: StandardSemModelV4ReadinessV1,
    authoring_issues: Vec<SemModelV4Issue>,
    readiness_issues: Vec<SemModelV4Issue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StandardSemModelV4AuthorityResolveResultV1 {
    schema_version: u32,
    canonical_model: SemModelV4,
    model_document_sha256: String,
    scientific_sha256: Option<String>,
    readiness: StandardSemModelV4ReadinessV1,
    authoring_issues: Vec<SemModelV4Issue>,
    readiness_issues: Vec<SemModelV4Issue>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StandardSemModelV4AuthorityCasDiagnosticV1 {
    code: String,
    message: String,
    corrective_action: String,
    authoring_issues: Vec<SemModelV4Issue>,
    readiness_issues: Vec<SemModelV4Issue>,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum StandardSemModelV4AuthorityCasOutcomeV1 {
    Ok {
        value: Box<StandardSemModelV4AuthorityCasResultV1>,
    },
    Blocked {
        diagnostic: StandardSemModelV4AuthorityCasDiagnosticV1,
    },
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum StandardSemModelV4AuthorityResolveOutcomeV1 {
    Ok {
        value: Box<StandardSemModelV4AuthorityResolveResultV1>,
    },
    Blocked {
        diagnostic: StandardSemModelV4AuthorityCasDiagnosticV1,
    },
}

fn blocked(
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
    authoring_issues: Vec<SemModelV4Issue>,
    readiness_issues: Vec<SemModelV4Issue>,
) -> StandardSemModelV4AuthorityCasOutcomeV1 {
    StandardSemModelV4AuthorityCasOutcomeV1::Blocked {
        diagnostic: StandardSemModelV4AuthorityCasDiagnosticV1 {
            code: code.into(),
            message: message.into(),
            corrective_action: corrective_action.into(),
            authoring_issues,
            readiness_issues,
        },
    }
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == LOWER_SHA256_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_complete_model(
    value: Value,
    subject: &str,
) -> Result<SemModelV4, StandardSemModelV4AuthorityCasOutcomeV1> {
    let Some(object) = value.as_object() else {
        return Err(blocked(
            "standard_sem_model_v4_authority.model_object_required",
            format!("{subject} must be a complete SemModelV4 object."),
            "Send the complete current model document, including annotations and presentation.",
            Vec::new(),
            Vec::new(),
        ));
    };
    let missing = COMPLETE_MODEL_FIELDS
        .iter()
        .filter(|field| !object.contains_key(**field))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(blocked(
            "standard_sem_model_v4_authority.complete_model_required",
            format!("{subject} is missing: {}.", missing.join(", ")),
            "Send the complete SemModelV4 document rather than a graph patch or partial record.",
            Vec::new(),
            Vec::new(),
        ));
    }
    serde_json::from_value(value).map_err(|error| {
        blocked(
            "standard_sem_model_v4_authority.model_wire_invalid",
            format!("{subject} is not a strict SemModelV4 document: {error}"),
            "Remove unknown fields and correct the exact typed SemModelV4 structure.",
            Vec::new(),
            Vec::new(),
        )
    })
}

fn compare_and_swap(
    request: StandardSemModelV4AuthorityCasRequestV1,
) -> StandardSemModelV4AuthorityCasOutcomeV1 {
    if !is_lower_sha256(&request.expected_source_model_document_sha256) {
        return blocked(
            "standard_sem_model_v4_authority.expected_digest_invalid",
            "The expected source model-document digest must be an exact lowercase SHA-256 value.",
            "Refresh the current Standard model authority and retry with its native document digest.",
            Vec::new(),
            Vec::new(),
        );
    }
    let source = match parse_complete_model(request.source_model, "sourceModel") {
        Ok(model) => model,
        Err(outcome) => return outcome,
    };
    let source_authoring_issues = source.validate_authoring_integrity();
    if !source_authoring_issues.is_empty() {
        let source_readiness_issues = source.validate();
        return blocked(
            "standard_sem_model_v4_authority.source_invalid",
            "The current source model does not pass native authoring integrity.",
            "Restore or refresh the current Standard model authority before applying a candidate.",
            source_authoring_issues,
            source_readiness_issues,
        );
    }
    let observed_source_digest = source
        .model_document_sha256()
        .expect("authoring-valid source must have a document digest");
    if observed_source_digest != request.expected_source_model_document_sha256 {
        return blocked(
            "standard_sem_model_v4_authority.stale_source_digest",
            format!(
                "The source model-document digest changed (expected {}, observed {}).",
                request.expected_source_model_document_sha256, observed_source_digest
            ),
            "Refresh the current Standard model authority and reapply the edit to its latest document.",
            Vec::new(),
            Vec::new(),
        );
    }

    let candidate = match parse_complete_model(request.candidate, "candidate") {
        Ok(model) => model,
        Err(outcome) => return outcome,
    };
    if candidate.id != source.id {
        return blocked(
            "standard_sem_model_v4_authority.model_id_mismatch",
            format!(
                "Candidate model id {:?} differs from source model id {:?}.",
                candidate.id, source.id
            ),
            "Keep the complete candidate model id equal to the current Standard authority id.",
            Vec::new(),
            Vec::new(),
        );
    }

    let authoring_issues = candidate.validate_authoring_integrity();
    if !authoring_issues.is_empty() {
        let readiness_issues = candidate.validate();
        return blocked(
            "standard_sem_model_v4_authority.candidate_authoring_invalid",
            "The candidate failed native SemModelV4 authoring integrity.",
            "Correct every returned authoring issue before replacing the Standard authority.",
            authoring_issues,
            readiness_issues,
        );
    }

    let canonical_candidate = candidate.canonicalized();
    let readiness_issues = canonical_candidate.validate();
    let candidate_model_document_sha256 = canonical_candidate
        .model_document_sha256()
        .expect("authoring-valid candidate must have a document digest");
    let (readiness, candidate_scientific_sha256) = if readiness_issues.is_empty() {
        (
            StandardSemModelV4ReadinessV1::Ready,
            Some(
                canonical_candidate
                    .scientific_sha256()
                    .expect("ready candidate must have a scientific digest"),
            ),
        )
    } else {
        (StandardSemModelV4ReadinessV1::AuthoringOnly, None)
    };

    StandardSemModelV4AuthorityCasOutcomeV1::Ok {
        value: Box::new(StandardSemModelV4AuthorityCasResultV1 {
            schema_version: CAS_RESULT_SCHEMA_VERSION,
            source_model_document_sha256: observed_source_digest,
            canonical_candidate,
            candidate_model_document_sha256,
            candidate_scientific_sha256,
            readiness,
            authoring_issues,
            readiness_issues,
        }),
    }
}

fn resolve(
    request: StandardSemModelV4AuthorityResolveRequestV1,
) -> StandardSemModelV4AuthorityResolveOutcomeV1 {
    let model = match parse_complete_model(request.model, "model") {
        Ok(model) => model,
        Err(StandardSemModelV4AuthorityCasOutcomeV1::Blocked { diagnostic }) => {
            return StandardSemModelV4AuthorityResolveOutcomeV1::Blocked { diagnostic };
        }
        Err(StandardSemModelV4AuthorityCasOutcomeV1::Ok { .. }) => unreachable!(),
    };
    let authoring_issues = model.validate_authoring_integrity();
    if !authoring_issues.is_empty() {
        let readiness_issues = model.validate();
        return StandardSemModelV4AuthorityResolveOutcomeV1::Blocked {
            diagnostic: StandardSemModelV4AuthorityCasDiagnosticV1 {
                code: "standard_sem_model_v4_authority.model_authoring_invalid".into(),
                message: "The model failed native SemModelV4 authoring integrity.".into(),
                corrective_action:
                    "Correct every returned authoring issue before installing Standard authority."
                        .into(),
                authoring_issues,
                readiness_issues,
            },
        };
    }

    let canonical_model = model.canonicalized();
    let readiness_issues = canonical_model.validate();
    let model_document_sha256 = canonical_model
        .model_document_sha256()
        .expect("authoring-valid model must have a document digest");
    let (readiness, scientific_sha256) = if readiness_issues.is_empty() {
        (
            StandardSemModelV4ReadinessV1::Ready,
            Some(
                canonical_model
                    .scientific_sha256()
                    .expect("ready model must have a scientific digest"),
            ),
        )
    } else {
        (StandardSemModelV4ReadinessV1::AuthoringOnly, None)
    };

    StandardSemModelV4AuthorityResolveOutcomeV1::Ok {
        value: Box::new(StandardSemModelV4AuthorityResolveResultV1 {
            schema_version: CAS_RESULT_SCHEMA_VERSION,
            canonical_model,
            model_document_sha256,
            scientific_sha256,
            readiness,
            authoring_issues,
            readiness_issues,
        }),
    }
}

#[tauri::command]
pub(crate) fn compare_and_swap_standard_sem_model_v4_authority(
    request: StandardSemModelV4AuthorityCasRequestV1,
) -> StandardSemModelV4AuthorityCasOutcomeV1 {
    compare_and_swap(request)
}

#[tauri::command]
pub(crate) fn resolve_standard_sem_model_v4_authority(
    request: StandardSemModelV4AuthorityResolveRequestV1,
) -> StandardSemModelV4AuthorityResolveOutcomeV1 {
    resolve(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{
        Construct, FactorIdentificationV4, LegacyBasicModelInterpretationV4, MeasurementMode,
        ModelSpec, SemPresentationV4, SemVariableV4, convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn ready_model() -> SemModelV4 {
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::nil(),
                name: "Standard authority fixture".into(),
                constructs: vec![Construct {
                    id: "factor".into(),
                    name: "Factor".into(),
                    short_name: "F".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into(), "x3".into()],
                }],
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap()
    }

    fn ready_two_factor_model() -> SemModelV4 {
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::nil(),
                name: "Unsorted Standard authority fixture".into(),
                constructs: vec![
                    Construct {
                        id: "factor-z".into(),
                        name: "Factor Z".into(),
                        short_name: "FZ".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["z1".into(), "z2".into(), "z3".into()],
                    },
                    Construct {
                        id: "factor-a".into(),
                        name: "Factor A".into(),
                        short_name: "FA".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["a1".into(), "a2".into(), "a3".into()],
                    },
                ],
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap()
    }

    fn request(
        source: &SemModelV4,
        candidate: SemModelV4,
    ) -> StandardSemModelV4AuthorityCasRequestV1 {
        StandardSemModelV4AuthorityCasRequestV1 {
            expected_source_model_document_sha256: source.model_document_sha256().unwrap(),
            source_model: serde_json::to_value(source).unwrap(),
            candidate: serde_json::to_value(candidate).unwrap(),
        }
    }

    #[test]
    fn canonical_candidate_returns_document_and_ready_scientific_digests() {
        let source = ready_model();
        let source_scientific = source.scientific_sha256().unwrap();
        let mut candidate = source.clone();
        candidate.presentation = SemPresentationV4::Canvas {
            nodes: Vec::new(),
            edges: Vec::new(),
            shapes: Vec::new(),
            images: Vec::new(),
            lines: Vec::new(),
            zoom: Some(1.25),
            pan_x: Some(2.0),
            pan_y: Some(3.0),
        };

        let StandardSemModelV4AuthorityCasOutcomeV1::Ok { value } =
            compare_and_swap(request(&source, candidate))
        else {
            panic!("valid same-id candidate was blocked")
        };
        assert_eq!(value.readiness, StandardSemModelV4ReadinessV1::Ready);
        assert!(value.authoring_issues.is_empty());
        assert!(value.readiness_issues.is_empty());
        assert_ne!(
            value.candidate_model_document_sha256,
            value.source_model_document_sha256
        );
        assert_eq!(
            value.candidate_scientific_sha256.as_deref(),
            Some(source_scientific.as_str())
        );
        assert_eq!(
            value.candidate_model_document_sha256,
            value.canonical_candidate.model_document_sha256().unwrap()
        );
    }

    #[test]
    fn authoring_only_candidate_returns_exact_readiness_issues_without_scientific_digest() {
        let source = ready_model();
        let mut candidate = source.clone();
        let identification = candidate
            .variables
            .iter_mut()
            .find_map(|variable| match variable {
                SemVariableV4::CommonFactor { identification, .. } => Some(identification),
                _ => None,
            })
            .unwrap();
        *identification = FactorIdentificationV4::FixedVariance;

        let StandardSemModelV4AuthorityCasOutcomeV1::Ok { value } =
            compare_and_swap(request(&source, candidate))
        else {
            panic!("authoring-valid draft was blocked")
        };
        assert_eq!(
            value.readiness,
            StandardSemModelV4ReadinessV1::AuthoringOnly
        );
        assert!(value.authoring_issues.is_empty());
        assert!(value.candidate_scientific_sha256.is_none());
        assert!(
            value
                .readiness_issues
                .iter()
                .any(|issue| issue.code == "identification.fixed_variance.missing")
        );
    }

    #[test]
    fn unsorted_authoring_only_input_returns_canonical_multi_issue_order() {
        let source = ready_two_factor_model();
        let mut draft = source.clone();
        for variable in &mut draft.variables {
            if let SemVariableV4::CommonFactor { identification, .. } = variable {
                *identification = FactorIdentificationV4::FixedVariance;
            }
        }
        draft
            .variables
            .sort_by(|left, right| right.id().cmp(left.id()));
        draft.relations.reverse();
        draft.parameters.reverse();

        assert!(draft.validate_authoring_integrity().is_empty());
        let input_readiness_issues = draft.validate();
        let canonical = draft.canonicalized();
        let canonical_readiness_issues = canonical.validate();
        assert_eq!(
            canonical_readiness_issues
                .iter()
                .filter(|issue| issue.code == "identification.fixed_variance.missing")
                .count(),
            2
        );
        assert_ne!(input_readiness_issues, canonical_readiness_issues);

        let StandardSemModelV4AuthorityCasOutcomeV1::Ok { value } =
            compare_and_swap(request(&source, draft.clone()))
        else {
            panic!("authoring-valid unsorted candidate was blocked")
        };
        assert_eq!(value.canonical_candidate, canonical);
        assert_eq!(value.readiness_issues, canonical_readiness_issues);

        let StandardSemModelV4AuthorityResolveOutcomeV1::Ok { value } =
            resolve(StandardSemModelV4AuthorityResolveRequestV1 {
                model: serde_json::to_value(draft).unwrap(),
            })
        else {
            panic!("authoring-valid unsorted model was blocked")
        };
        assert_eq!(value.canonical_model, canonical);
        assert_eq!(value.readiness_issues, canonical_readiness_issues);
    }

    #[test]
    fn stale_digest_id_change_and_invalid_authoring_fail_closed() {
        let source = ready_model();
        let mut unexpected = request(&source, source.clone());
        unexpected
            .candidate
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::json!(true));
        assert!(matches!(
            compare_and_swap(unexpected),
            StandardSemModelV4AuthorityCasOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "standard_sem_model_v4_authority.model_wire_invalid"
        ));

        let mut stale = request(&source, source.clone());
        stale.expected_source_model_document_sha256 = "f".repeat(64);
        assert!(matches!(
            compare_and_swap(stale),
            StandardSemModelV4AuthorityCasOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "standard_sem_model_v4_authority.stale_source_digest"
        ));

        let mut changed_id = source.clone();
        changed_id.id = "different-model".into();
        assert!(matches!(
            compare_and_swap(request(&source, changed_id)),
            StandardSemModelV4AuthorityCasOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "standard_sem_model_v4_authority.model_id_mismatch"
        ));

        let mut invalid = source.clone();
        if let Some(parameter) = invalid
            .relations
            .iter_mut()
            .find_map(|relation| match relation {
                qpls_core::SemRelationV4::MeasurementEffect { parameter, .. } => Some(parameter),
                _ => None,
            })
        {
            *parameter = "missing-parameter".into();
        }
        let StandardSemModelV4AuthorityCasOutcomeV1::Blocked { diagnostic } =
            compare_and_swap(request(&source, invalid))
        else {
            panic!("invalid candidate was accepted")
        };
        assert_eq!(
            diagnostic.code,
            "standard_sem_model_v4_authority.candidate_authoring_invalid"
        );
        assert!(
            diagnostic
                .authoring_issues
                .iter()
                .any(|issue| issue.code == "relation.parameter.unknown")
        );
    }

    #[test]
    fn bootstrap_resolves_canonical_authority_and_rejects_invalid_documents() {
        let model = ready_model();
        let expected_document = model.canonicalized().model_document_sha256().unwrap();
        let expected_scientific = model.canonicalized().scientific_sha256().unwrap();
        let StandardSemModelV4AuthorityResolveOutcomeV1::Ok { value } =
            resolve(StandardSemModelV4AuthorityResolveRequestV1 {
                model: serde_json::to_value(&model).unwrap(),
            })
        else {
            panic!("ready complete model was blocked")
        };
        assert_eq!(value.model_document_sha256, expected_document);
        assert_eq!(
            value.scientific_sha256.as_deref(),
            Some(expected_scientific.as_str())
        );
        assert_eq!(value.canonical_model, model.canonicalized());
        assert_eq!(value.readiness, StandardSemModelV4ReadinessV1::Ready);

        let mut unexpected = serde_json::to_value(&model).unwrap();
        unexpected
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::json!(true));
        assert!(matches!(
            resolve(StandardSemModelV4AuthorityResolveRequestV1 { model: unexpected }),
            StandardSemModelV4AuthorityResolveOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "standard_sem_model_v4_authority.model_wire_invalid"
        ));

        let mut draft = model;
        let identification = draft
            .variables
            .iter_mut()
            .find_map(|variable| match variable {
                SemVariableV4::CommonFactor { identification, .. } => Some(identification),
                _ => None,
            })
            .unwrap();
        *identification = FactorIdentificationV4::FixedVariance;
        let StandardSemModelV4AuthorityResolveOutcomeV1::Ok { value } =
            resolve(StandardSemModelV4AuthorityResolveRequestV1 {
                model: serde_json::to_value(draft).unwrap(),
            })
        else {
            panic!("authoring-only complete model was blocked")
        };
        assert_eq!(
            value.readiness,
            StandardSemModelV4ReadinessV1::AuthoringOnly
        );
        assert!(value.scientific_sha256.is_none());
        assert!(value.authoring_issues.is_empty());
        assert!(!value.readiness_issues.is_empty());
    }
}
