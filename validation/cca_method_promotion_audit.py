#!/usr/bin/env python3
"""Promotion gate for the bounded descriptive CCA residual workflow.

This audit deliberately refuses to infer native readiness from an estimator
artifact alone. A passing result requires the non-saturated numerical
reference, current native catalog/result contracts, project persistence
support, and genuine packaged-Tauri run/export/reopen evidence.
"""

from __future__ import annotations

import json
from pathlib import Path

from final_method_promotion_common import ROOT, audit_method


RESULTS = ROOT / "validation" / "results"
CCA_METHOD_VERSION = "cca_composite_residual_v1"
CCA_MODEL_VERSION = "recursive_standardized_composite_path_model_v1"
CCA_PROVENANCE_METHOD_VERSION = (
    "pls_pm_v1+cca_composite_residual_v1+pls_mediation_v1+pls_assessment_v7"
)


def load_json(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def source_contains(path: str, *snippets: str) -> bool:
    text = (ROOT / path).read_text(encoding="utf-8")
    return all(snippet in text for snippet in snippets)


reference = load_json(RESULTS / "cca_reference_report.json")
reference_checks = reference.get("checks", {})
reference_contract_passed = (
    reference.get("passed") is True
    and reference.get("method_version") == CCA_METHOD_VERSION
    and reference.get("nested_model_version") == CCA_MODEL_VERSION
    and reference.get("fixture", {}).get("saturated") is False
    and reference.get("scope") == {
        "kind": "descriptive_composite_correlation_residuals_only",
        "inference": False,
        "thresholds": False,
        "classification": False,
    }
    and bool(reference_checks)
    and all(value is True for value in reference_checks.values())
)

native_catalog_and_results = (
    source_contains(
        "src/native/nativeAnalysisCatalog.ts",
        '| "cca"',
        'kind: "cca"',
        'categoryId: "assessment"',
        "Inspect descriptive residuals between observed and model-reproduced composite correlations.",
    )
    and source_contains(
        "src/native/nativeResults.ts",
        '"cca_residual_summary"',
        '"cca_composite_residuals"',
        'title: "Residual summary"',
        'title: "Composite residuals"',
    )
)

native_readiness_contract = source_contains(
    "src/native/nativePlsReadiness.ts",
    'settings.method === "cca"',
    "CCA composite residual diagnostics are selected",
    "no thresholds or inferential classification are calculated",
)

browser = load_json(RESULTS / "v247_native_desktop_visual_acceptance.json")
browser_checks = browser.get("checks", {})
browser_cca_checks = browser_checks.get("cca", [])
browser_catalog_checks = browser_checks.get("calculationCatalog", [])
browser_viewports = {"1024x700", "1280x720", "1440x900"}
expected_catalog = [
    "PLS-SEM Algorithm",
    "Consistent PLS",
    "Weighted PLS",
    "CCA composite residual diagnostics",
    "PLS-SEM Bootstrapping",
    "Structural Path Randomization",
    "Two-Group Permutation MGA",
    "Deterministic Construct Prediction",
]
cca_browser_skips = [
    item for item in browser.get("skipped", [])
    if item.get("id") == "cca-completed-results-browser"
]
browser_visual_contract_passed = (
    browser.get("passed") is True
    and not browser.get("failures")
    and not browser.get("consoleErrors")
    and {item.get("viewport") for item in browser_cca_checks} == browser_viewports
    and len(browser_cca_checks) == len(browser_viewports)
    and all(
        item.get("dialogOpened") is True
        and item.get("pointerSelected") is True
        and item.get("linkage", {}).get("linkage") is True
        and item.get("resultDataDetail") == "Standardized (fixed)"
        and item.get("scopeDetail") == "Reflective composite path model; descriptive residual diagnostics only"
        and item.get("missingDataDetail") == "Listwise deletion"
        and item.get("pcaWeightingDisabled") is True
        and item.get("resamplingControlCount") == 0
        and item.get("caseWeightControlCount") == 0
        and item.get("startCommandDisabled") is True
        and item.get("previewRuntimeBlockerVisible") is True
        and item.get("ccaModelScopeBlockerAbsent") is True
        and item.get("descriptiveBoundaryVisible") is True
        and item.get("truthAndOverflow", {}).get("noFabricatedRunState") is True
        for item in browser_cca_checks
    )
    and {item.get("viewport") for item in browser_catalog_checks} == browser_viewports
    and all(
        item.get("optionLabels") == expected_catalog
        and item.get("countStatus") == "8 methods"
        for item in browser_catalog_checks
    )
    and {item.get("viewport") for item in cca_browser_skips} == browser_viewports
    and all("packaged" in item.get("requiredNativeFollowUp", "").lower() for item in cca_browser_skips)
    and all(item.get("state") != "cca-results" for item in browser.get("screenshots", []))
    and {
        item.get("viewport") for item in browser.get("screenshots", [])
        if item.get("state") == "cca-dialog"
    } == browser_viewports
)

project_persistence_contract = (
    source_contains(
        "crates/qpls-project/src/lib.rs",
        "AnalysisMethod::Cca => Some(CCA_METHOD_VERSION)",
        "validate_cca_payload_contract",
        CCA_MODEL_VERSION,
    )
    and source_contains(
        "crates/qpls-runner/src/lib.rs",
        "qpls_core::AnalysisMethod::Cca",
        "estimation_method_version.as_str()",
    )
)

tauri = load_json(RESULTS / "v247_tauri_native_acceptance.json")
tauri_checks = tauri.get("checks", {})
required_tauri_checks = [
    "ccaFixtureProvisioning",
    "visibleCcaModelBuild",
    "ccaCalculationDialog",
    "ccaRunning",
    "ccaResult",
    "ccaExport",
    "ccaSaveReopen",
]
packaged_native_passed = (
    tauri.get("passed") is True
    and not tauri.get("failures")
    and not tauri.get("consoleErrors")
    and all(name in tauri_checks for name in required_tauri_checks)
    and tauri_checks.get("ccaResult", {}).get("methodVersion")
    == CCA_PROVENANCE_METHOD_VERSION
    and tauri_checks.get("ccaResult", {}).get("nestedModelVersion") == CCA_MODEL_VERSION
    and tauri_checks.get("ccaResult", {}).get("autoOpenedTable") == "Residual summary"
    and tauri_checks.get("ccaResult", {}).get("correlationPairs") == 3
    and tauri_checks.get("ccaResult", {}).get("maximumAbsoluteResidual", 0) > 0
    and tauri_checks.get("ccaExport", {}).get("residualTablesIncluded") is True
    and tauri_checks.get("ccaExport", {}).get("nativeXlsx", {}).get("attempted") is True
    and tauri_checks.get("ccaExport", {}).get("nativeXlsx", {}).get("file", {}).get("isFile") is True
    and tauri_checks.get("ccaSaveReopen", {}).get("sameRunRestored") is True
    and tauri_checks.get("ccaSaveReopen", {}).get("archive", {}).get("methodVersion")
    == CCA_PROVENANCE_METHOD_VERSION
    and tauri_checks.get("ccaSaveReopen", {}).get("archive", {}).get("nestedModelVersion") == CCA_MODEL_VERSION
    and tauri_checks.get("ccaSaveReopen", {}).get("archive", {}).get("residualIdentities") is True
)


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "cca",
    "Bounded descriptive composite-correlation residual diagnostics for recursive standardized reflective PLS path models.",
    [
        "cca_reference_report.json",
        "v247_native_desktop_visual_acceptance.json",
        "v247_tauri_native_acceptance.json",
    ],
    ["PLS_CCA_V1.md", "CCA_V1.md"],
    [
        {
            "name": "nonsaturated_reference_contract",
            "passed": reference_contract_passed,
            "detail": "The current independent reference must use a non-saturated model and verify exact descriptive payload identity, residual algebra, provenance, and bounded-scope guards.",
        },
        {
            "name": "native_catalog_readiness_results",
            "passed": native_catalog_and_results and native_readiness_contract,
            "detail": "The native workbench must expose one assessment method, truthful bounded readiness, and only the residual summary/detail tables actually present.",
        },
        {
            "name": "project_persistence_contract",
            "passed": project_persistence_contract,
            "detail": "Runner provenance and qpls-project validation must recognize the exact CCA method/payload versions before a completed run can be committed or reopened.",
        },
        {
            "name": "browser_visual_setup_and_follow_up",
            "passed": browser_visual_contract_passed,
            "detail": "Every supported viewport must prove the exact CCA catalog/setup/readiness contract and explicitly defer genuine completion to packaged Tauri without a synthetic result fixture.",
        },
        {
            "name": "packaged_native_workflow",
            "passed": packaged_native_passed,
            "detail": "The current packaged-Tauri report must prove a genuine CCA run, residual results, export, explicit save, and reopen. Browser fixtures cannot satisfy this check.",
        },
        {
            "name": "no_full_smartpls_cca_claim",
            "passed": source_contains(
                "docs/methods/PLS_CCA_V1.md",
                "not a full implementation of SmartPLS CCA",
                "No threshold, pass/fail decision, adequacy classification, p value, confidence interval, or bootstrap discrepancy test is produced.",
            ),
            "detail": "Documentation must retain the descriptive-only boundary and avoid claiming full SmartPLS CCA equivalence.",
        },
    ],
))
