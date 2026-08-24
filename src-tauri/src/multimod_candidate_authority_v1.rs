//! Immutable build-embedded MultiMod candidate authority.
//!
//! Runtime requests, project archives, and process environment variables are
//! deliberately absent from this trust boundary. `build.rs` always emits an
//! OUT_DIR document: preview builds receive a Labs-only sentinel, while an
//! explicitly qualified package build receives one hash-bound authority.

use qpls_core::{
    AnalysisRecipeV4, CompiledMultiModRecipeV1,
    MULTIMOD_CANDIDATE_QUALIFICATION_RECEIPT_V1_SCHEMA_VERSION, MultiModAnalysisResultV1,
    MultiModCandidateAuthorityErrorV1, MultimodCandidateQualificationReceiptV1,
    MultimodQualificationStateV1, apply_multimod_candidate_qualification_v1,
    required_multimod_candidate_profile_cells_v1,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::cell::RefCell;
use std::{collections::BTreeSet, sync::OnceLock};

const EMBEDDED_AUTHORITY_JSON: &str = include_str!(concat!(
    env!("OUT_DIR"),
    "/qpls_multimod_embedded_candidate_authority_v1.json"
));
const EMBEDDED_PREPACKAGE_MANIFEST_SET_JSON: &str = include_str!(concat!(
    env!("OUT_DIR"),
    "/qpls_multimod_embedded_prepackage_manifest_set_v1.json"
));

#[derive(Debug, Clone, Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CandidateAuthorityBindingV1 {
    candidate_commit_sha: String,
    candidate_version: String,
    qualification_plan_sha256: String,
    gate_binding_sha256: String,
    capability_index_sha256: String,
    prepackage_manifest_set_sha256: String,
    exact_profile_cells: Vec<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct CandidateAuthorityDocumentV1 {
    schema_version: u32,
    authority_kind: String,
    state: NativeMultiModCandidateAuthorityStateV1,
    binding: Option<CandidateAuthorityBindingV1>,
    authority_binding_sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NativeMultiModCandidateAuthorityStateV1 {
    LabsOnly,
    ReleaseQualifiedCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeMultiModCandidateAuthorityStatusV1 {
    pub(crate) state: NativeMultiModCandidateAuthorityStateV1,
    /// SHA-256 of the exact immutable document embedded into this executable.
    pub(crate) embedded_document_sha256: String,
    pub(crate) authority_binding_sha256: Option<String>,
    binding: Option<CandidateAuthorityBindingV1>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeMultiModCandidateAuthorityStatusWireV1 {
    schema_version: u32,
    state: NativeMultiModCandidateAuthorityStateV1,
    embedded_document_sha256: String,
    authority_binding_sha256: Option<String>,
    candidate_commit_sha: Option<String>,
    candidate_version: Option<String>,
    qualification_plan_sha256: Option<String>,
    gate_binding_sha256: Option<String>,
    capability_index_sha256: Option<String>,
    prepackage_manifest_set_sha256: Option<String>,
    exact_profile_cells: Vec<String>,
}

static AUTHORITY: OnceLock<Result<NativeMultiModCandidateAuthorityStatusV1, String>> =
    OnceLock::new();

#[cfg(test)]
thread_local! {
    /// Test binaries may inject one fully parsed, typed authority on their own
    /// thread. Production builds do not compile this cell or consult mutable
    /// runtime state; they continue to trust only `EMBEDDED_AUTHORITY_JSON`.
    static QUALIFICATION_TEST_AUTHORITY_V1: RefCell<Option<NativeMultiModCandidateAuthorityStatusV1>> = const { RefCell::new(None) };
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn lower_sha(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn exact_cell(value: &str) -> bool {
    if value.is_empty()
        || value.trim() != value
        || value
            .chars()
            .any(|character| matches!(character, '*' | '?' | '[' | ']') || character.is_control())
    {
        return false;
    }
    let mut pieces = value.split("::");
    matches!((pieces.next(), pieces.next(), pieces.next()), (Some(profile), Some(procedure), None) if !profile.is_empty() && !procedure.is_empty())
}

fn parse_authority_document_v1(
    authority_json: &str,
    prepackage_manifest_set_json: &str,
) -> Result<NativeMultiModCandidateAuthorityStatusV1, String> {
    let document: CandidateAuthorityDocumentV1 = serde_json::from_str(authority_json)
        .map_err(|error| format!("embedded MultiMod authority is invalid: {error}"))?;
    if document.schema_version != 1
        || document.authority_kind != "qpls_multimod_embedded_candidate_authority_v1"
    {
        return Err("embedded MultiMod authority identity is unsupported".into());
    }
    let document_sha256 = sha256(authority_json.as_bytes());
    match document.state {
        NativeMultiModCandidateAuthorityStateV1::LabsOnly => {
            if document.binding.is_some() || document.authority_binding_sha256.is_some() {
                return Err(
                    "Labs-only MultiMod authority cannot carry candidate binding data".into(),
                );
            }
            Ok(NativeMultiModCandidateAuthorityStatusV1 {
                state: document.state,
                embedded_document_sha256: document_sha256,
                authority_binding_sha256: None,
                binding: None,
            })
        }
        NativeMultiModCandidateAuthorityStateV1::ReleaseQualifiedCandidate => {
            let binding = document
                .binding
                .ok_or_else(|| "candidate MultiMod authority binding is missing".to_owned())?;
            let authority_binding_sha256 = document
                .authority_binding_sha256
                .ok_or_else(|| "candidate MultiMod authority digest is missing".to_owned())?;
            let canonical = serde_json::to_vec(&binding).map_err(|error| {
                format!("candidate authority could not be canonicalized: {error}")
            })?;
            if sha256(&canonical) != authority_binding_sha256
                || !lower_sha(&authority_binding_sha256, 64)
                || !lower_sha(&binding.candidate_commit_sha, 40)
                || binding.candidate_version != env!("CARGO_PKG_VERSION")
                || !lower_sha(&binding.qualification_plan_sha256, 64)
                || !lower_sha(&binding.gate_binding_sha256, 64)
                || !lower_sha(&binding.capability_index_sha256, 64)
                || !lower_sha(&binding.prepackage_manifest_set_sha256, 64)
                || binding.exact_profile_cells.is_empty()
                || binding
                    .exact_profile_cells
                    .iter()
                    .any(|cell| !exact_cell(cell))
                || !binding
                    .exact_profile_cells
                    .windows(2)
                    .all(|pair| pair[0] < pair[1])
                || sha256(prepackage_manifest_set_json.as_bytes())
                    != binding.prepackage_manifest_set_sha256
            {
                return Err("embedded MultiMod candidate authority binding is inconsistent".into());
            }
            Ok(NativeMultiModCandidateAuthorityStatusV1 {
                state: document.state,
                embedded_document_sha256: document_sha256,
                authority_binding_sha256: Some(authority_binding_sha256),
                binding: Some(binding),
            })
        }
    }
}

fn parse_authority_v1() -> Result<NativeMultiModCandidateAuthorityStatusV1, String> {
    parse_authority_document_v1(
        EMBEDDED_AUTHORITY_JSON,
        EMBEDDED_PREPACKAGE_MANIFEST_SET_JSON,
    )
}

pub(crate) fn embedded_multimod_candidate_authority_v1()
-> Result<&'static NativeMultiModCandidateAuthorityStatusV1, String> {
    AUTHORITY
        .get_or_init(parse_authority_v1)
        .as_ref()
        .map_err(Clone::clone)
}

pub(crate) fn embedded_multimod_cache_authority_sha256_v1() -> Result<String, String> {
    Ok(embedded_multimod_candidate_authority_v1()?
        .embedded_document_sha256
        .clone())
}

#[tauri::command]
pub(crate) fn multimod_candidate_authority_status_v1()
-> Result<NativeMultiModCandidateAuthorityStatusWireV1, String> {
    let authority = embedded_multimod_candidate_authority_v1()?;
    let binding = authority.binding.as_ref();
    Ok(NativeMultiModCandidateAuthorityStatusWireV1 {
        schema_version: 1,
        state: authority.state,
        embedded_document_sha256: authority.embedded_document_sha256.clone(),
        authority_binding_sha256: authority.authority_binding_sha256.clone(),
        candidate_commit_sha: binding.map(|value| value.candidate_commit_sha.clone()),
        candidate_version: binding.map(|value| value.candidate_version.clone()),
        qualification_plan_sha256: binding.map(|value| value.qualification_plan_sha256.clone()),
        gate_binding_sha256: binding.map(|value| value.gate_binding_sha256.clone()),
        capability_index_sha256: binding.map(|value| value.capability_index_sha256.clone()),
        prepackage_manifest_set_sha256: binding
            .map(|value| value.prepackage_manifest_set_sha256.clone()),
        exact_profile_cells: binding
            .map(|value| value.exact_profile_cells.clone())
            .unwrap_or_default(),
    })
}

pub(crate) fn promote_completed_multimod_result_v1(
    recipe: &AnalysisRecipeV4,
    artifact: &CompiledMultiModRecipeV1,
    result: &mut MultiModAnalysisResultV1,
) -> Result<(), String> {
    let authority = embedded_multimod_candidate_authority_v1()?;
    if result.provenance().qualification != MultimodQualificationStateV1::UnqualifiedLabs
        || result
            .provenance()
            .candidate_qualification_receipt
            .is_some()
    {
        return Err(
            "runner output must enter native promotion as receipt-free Labs provenance".into(),
        );
    }
    if authority.state == NativeMultiModCandidateAuthorityStateV1::LabsOnly {
        return Ok(());
    }
    let required = match required_multimod_candidate_profile_cells_v1(recipe, artifact, result) {
        Ok(required) => required,
        Err(MultiModCandidateAuthorityErrorV1::CellNotQualified(_)) => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    let Some(receipt) = candidate_receipt_for_required_cells_v1(authority, &required)? else {
        return Ok(());
    };
    apply_multimod_candidate_qualification_v1(recipe, artifact, result, receipt)
        .map_err(|error| error.to_string())
}

fn candidate_receipt_for_required_cells_v1(
    authority: &NativeMultiModCandidateAuthorityStatusV1,
    required: &[String],
) -> Result<Option<MultimodCandidateQualificationReceiptV1>, String> {
    if authority.state == NativeMultiModCandidateAuthorityStateV1::LabsOnly {
        return Ok(None);
    }
    let binding = authority
        .binding
        .as_ref()
        .ok_or_else(|| "candidate authority binding is unavailable".to_owned())?;
    let qualified = binding
        .exact_profile_cells
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if required
        .iter()
        .any(|cell| !qualified.contains(cell.as_str()))
    {
        // A build may carry a legitimate partial authority. Exact cells not in
        // that immutable set remain ordinary Labs output; authority itself is
        // neither widened nor treated as malformed.
        return Ok(None);
    }
    Ok(Some(MultimodCandidateQualificationReceiptV1 {
        schema_version: MULTIMOD_CANDIDATE_QUALIFICATION_RECEIPT_V1_SCHEMA_VERSION,
        authority_binding_sha256: authority
            .authority_binding_sha256
            .clone()
            .ok_or_else(|| "candidate authority digest is unavailable".to_owned())?,
        candidate_commit_sha: binding.candidate_commit_sha.clone(),
        candidate_version: binding.candidate_version.clone(),
        qualification_plan_sha256: binding.qualification_plan_sha256.clone(),
        gate_binding_sha256: binding.gate_binding_sha256.clone(),
        capability_index_sha256: binding.capability_index_sha256.clone(),
        prepackage_manifest_set_sha256: binding.prepackage_manifest_set_sha256.clone(),
        required_profile_cells: required.to_vec(),
    }))
}

pub(crate) fn verify_multimod_candidate_receipt_against_embedded_v1(
    receipt: &MultimodCandidateQualificationReceiptV1,
) -> Result<(), String> {
    #[cfg(test)]
    if let Some(injected) =
        QUALIFICATION_TEST_AUTHORITY_V1.with(|authority| authority.borrow().clone())
    {
        return verify_multimod_candidate_receipt_against_authority_v1(receipt, &injected);
    }
    let authority = embedded_multimod_candidate_authority_v1()?;
    verify_multimod_candidate_receipt_against_authority_v1(receipt, authority)
}

fn verify_multimod_candidate_receipt_against_authority_v1(
    receipt: &MultimodCandidateQualificationReceiptV1,
    authority: &NativeMultiModCandidateAuthorityStatusV1,
) -> Result<(), String> {
    if authority.state != NativeMultiModCandidateAuthorityStateV1::ReleaseQualifiedCandidate {
        return Err("this executable has no embedded MultiMod candidate authority".into());
    }
    let binding = authority
        .binding
        .as_ref()
        .ok_or_else(|| "candidate authority binding is unavailable".to_owned())?;
    let exact = receipt.schema_version
        == MULTIMOD_CANDIDATE_QUALIFICATION_RECEIPT_V1_SCHEMA_VERSION
        && authority.authority_binding_sha256.as_deref()
            == Some(receipt.authority_binding_sha256.as_str())
        && receipt.candidate_commit_sha == binding.candidate_commit_sha
        && receipt.candidate_version == binding.candidate_version
        && receipt.qualification_plan_sha256 == binding.qualification_plan_sha256
        && receipt.gate_binding_sha256 == binding.gate_binding_sha256
        && receipt.capability_index_sha256 == binding.capability_index_sha256
        && receipt.prepackage_manifest_set_sha256 == binding.prepackage_manifest_set_sha256
        && !receipt.required_profile_cells.is_empty()
        && receipt
            .required_profile_cells
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        && receipt.required_profile_cells.iter().all(|cell| {
            exact_cell(cell) && binding.exact_profile_cells.binary_search(cell).is_ok()
        });
    if exact {
        Ok(())
    } else {
        Err("candidate receipt differs from this executable's embedded authority".into())
    }
}

/// Builds and installs a fully parsed candidate authority for one test thread.
///
/// This seam is deliberately unavailable outside `cfg(test)`. It lets native
/// publication qualification exercise the same typed parser and receipt
/// verifier as a packaged candidate without allowing a runtime environment,
/// project, or request to override the production embedded authority.
#[cfg(test)]
pub(crate) fn with_typed_qualification_test_authority_v1<T>(
    exact_profile_cells: &[&str],
    action: impl FnOnce(&MultimodCandidateQualificationReceiptV1) -> T,
) -> Result<T, String> {
    let mut cells = exact_profile_cells
        .iter()
        .map(|cell| (*cell).to_owned())
        .collect::<Vec<_>>();
    cells.sort();
    if cells.is_empty()
        || cells.windows(2).any(|pair| pair[0] == pair[1])
        || cells.iter().any(|cell| !exact_cell(cell))
    {
        return Err("qualification-test authority requires sorted unique exact cells".into());
    }
    let prepackage_manifest_set_json = serde_json::to_string(&serde_json::json!({
        "schema_version": 1,
        "manifest_set_kind": "qpls_multimod_qualification_test_prepackage_set_v1",
        "exact_profile_cells": cells,
    }))
    .map_err(|error| error.to_string())?;
    let binding = CandidateAuthorityBindingV1 {
        candidate_commit_sha: "c".repeat(40),
        candidate_version: env!("CARGO_PKG_VERSION").into(),
        qualification_plan_sha256: "d".repeat(64),
        gate_binding_sha256: "e".repeat(64),
        capability_index_sha256: "f".repeat(64),
        prepackage_manifest_set_sha256: sha256(prepackage_manifest_set_json.as_bytes()),
        exact_profile_cells: cells.clone(),
    };
    let authority_binding_sha256 =
        sha256(&serde_json::to_vec(&binding).map_err(|error| error.to_string())?);
    let authority_json = serde_json::to_string(&CandidateAuthorityDocumentV1 {
        schema_version: 1,
        authority_kind: "qpls_multimod_embedded_candidate_authority_v1".into(),
        state: NativeMultiModCandidateAuthorityStateV1::ReleaseQualifiedCandidate,
        binding: Some(binding.clone()),
        authority_binding_sha256: Some(authority_binding_sha256.clone()),
    })
    .map_err(|error| error.to_string())?;
    let parsed = parse_authority_document_v1(&authority_json, &prepackage_manifest_set_json)?;
    let receipt = MultimodCandidateQualificationReceiptV1 {
        schema_version: MULTIMOD_CANDIDATE_QUALIFICATION_RECEIPT_V1_SCHEMA_VERSION,
        authority_binding_sha256,
        candidate_commit_sha: binding.candidate_commit_sha,
        candidate_version: binding.candidate_version,
        qualification_plan_sha256: binding.qualification_plan_sha256,
        gate_binding_sha256: binding.gate_binding_sha256,
        capability_index_sha256: binding.capability_index_sha256,
        prepackage_manifest_set_sha256: binding.prepackage_manifest_set_sha256,
        required_profile_cells: cells,
    };
    verify_multimod_candidate_receipt_against_authority_v1(&receipt, &parsed)?;

    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            QUALIFICATION_TEST_AUTHORITY_V1.with(|authority| {
                *authority.borrow_mut() = None;
            });
        }
    }
    QUALIFICATION_TEST_AUTHORITY_V1.with(|authority| -> Result<(), String> {
        if authority.borrow().is_some() {
            return Err("qualification-test authority is already installed on this thread".into());
        }
        *authority.borrow_mut() = Some(parsed);
        Ok(())
    })?;
    let reset = Reset;
    let output = action(&receipt);
    drop(reset);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate_authority(cells: &[&str]) -> NativeMultiModCandidateAuthorityStatusV1 {
        NativeMultiModCandidateAuthorityStatusV1 {
            state: NativeMultiModCandidateAuthorityStateV1::ReleaseQualifiedCandidate,
            embedded_document_sha256: "a".repeat(64),
            authority_binding_sha256: Some("b".repeat(64)),
            binding: Some(CandidateAuthorityBindingV1 {
                candidate_commit_sha: "c".repeat(40),
                candidate_version: env!("CARGO_PKG_VERSION").into(),
                qualification_plan_sha256: "d".repeat(64),
                gate_binding_sha256: "e".repeat(64),
                capability_index_sha256: "f".repeat(64),
                prepackage_manifest_set_sha256: "1".repeat(64),
                exact_profile_cells: cells.iter().map(|cell| (*cell).into()).collect(),
            }),
        }
    }

    #[test]
    fn every_build_contains_a_parseable_immutable_authority_or_labs_sentinel() {
        let authority = embedded_multimod_candidate_authority_v1().unwrap();
        assert!(lower_sha(&authority.embedded_document_sha256, 64));
        if authority.state == NativeMultiModCandidateAuthorityStateV1::LabsOnly {
            assert!(authority.binding.is_none());
            assert!(authority.authority_binding_sha256.is_none());
        }
    }

    #[test]
    fn exact_listed_cells_receive_a_candidate_receipt() {
        let required: Vec<String> =
            vec!["conditional.multi_two_way_percentile.v2::explicit_path_target_math".into()];
        let receipt = candidate_receipt_for_required_cells_v1(
            &candidate_authority(&[required[0].as_str()]),
            &required,
        )
        .unwrap()
        .expect("exact listed candidate cell must be promoted");
        assert_eq!(receipt.required_profile_cells, required);
    }

    #[test]
    fn valid_partial_authority_leaves_a_missing_cell_in_labs() {
        let receipt = candidate_receipt_for_required_cells_v1(
            &candidate_authority(&[
                "conditional.multi_two_way_percentile.v2::explicit_path_target_math",
            ]),
            &["conditional.multi_two_way_percentile.v2::shared_ledger_percentile_type7".into()],
        )
        .unwrap();
        assert!(receipt.is_none());
    }
}
