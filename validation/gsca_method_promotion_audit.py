#!/usr/bin/env python3
"""Promotion gate for the bounded, executable GSCA ALS v2 workflow."""

from __future__ import annotations

from final_method_promotion_common import ROOT, RESULTS, audit_method, load_json


reference = load_json(RESULTS / "gsca_als_v2_reference_report.json")
browser = load_json(RESULTS / "v247_native_desktop_visual_acceptance.json")
native = load_json(RESULTS / "v247_tauri_native_acceptance_gsca.json")

reference_deltas = reference.get("deltas", {})
reference_contract = reference.get("contract_checks", {})
browser_gsca = browser.get("checks", {}).get("gsca", [])
native_checks = native.get("checks", {})
native_result = native_checks.get("gscaResult", {})
native_export = native_checks.get("gscaExport", {})
native_reopen = native_checks.get("gscaSaveReopen", {})
archive_gsca = native_reopen.get("archive", {}).get("gsca", {})

engine_source = (ROOT / "crates" / "qpls-estimation" / "src" / "pls.rs").read_text(encoding="utf-8")
project_source = (ROOT / "crates" / "qpls-project" / "src" / "lib.rs").read_text(encoding="utf-8")
catalog_source = (ROOT / "src" / "native" / "nativeAnalysisCatalog.ts").read_text(encoding="utf-8")
results_source = (ROOT / "src" / "native" / "nativeResults.ts").read_text(encoding="utf-8")
legacy_doc = (ROOT / "docs" / "methods" / "GSCA_V1.md").read_text(encoding="utf-8")

checks = [
    {
        "name": "independent_global_criterion_reference",
        "passed": reference.get("passed") is True
        and reference.get("method_version") == "gsca_als_v2"
        and reference.get("reference") == "independent_scipy_slsqp_global_gsca_criterion_v1"
        and reference.get("tolerance") == 2e-6
        and reference_deltas
        and max(reference_deltas.values()) <= reference.get("tolerance")
        and reference_contract
        and all(reference_contract.values()),
        "detail": "Independent SLSQP minimization reproduces every bounded GSCA ALS v2 numerical and version contract within 2e-6.",
    },
    {
        "name": "executable_and_persistence_contract",
        "passed": all(token in engine_source for token in [
            'GSCA_METHOD_VERSION: &str = "gsca_als_v2"',
            'GSCA_ALGORITHM_VERSION: &str = "alternating_least_squares_v1"',
            "estimate_gsca_method",
        ]) and all(token in project_source for token in [
            "validate_gsca_payload_contract",
            "runner_generated_gsca_als_v2_commits_saves_reopens_and_rejects_contract_tampering",
            "gsca_als_v2",
        ]),
        "detail": "The dedicated ALS estimator and strict append/save/reopen/tamper validator are present; this is not the historical PLS-derived preview.",
    },
    {
        "name": "native_setup_results_and_export_contract",
        "passed": all(token in catalog_source for token in ["component_models", 'kind: "gsca"'])
        and all(token in results_source for token in [
            'id: "gsca_fit"', 'id: "gsca_paths"', 'id: "gsca_loadings"',
            'id: "gsca_weights"', 'id: "gsca_scope"',
        ])
        and len(browser_gsca) == 3
        and all(item.get("pointerSelected") is True
                and item.get("modelScopeBlockerAbsent") is True
                and item.get("truthAndOverflow", {}).get("noHorizontalOverflow") is True
                for item in browser_gsca),
        "detail": "All three browser viewports prove the exact bounded setup with no fabricated run state; native result/export tables are method-specific.",
    },
    {
        "name": "genuine_packaged_native_workflow",
        "passed": native.get("passed") is True
        and native_checks.get("gscaProgress", {}).get("captured") is True
        and native_result.get("runLabel") == "GSCA run"
        and native_result.get("initialSelectedTable") == "gsca_fit"
        and native_result.get("noPlaceholder") is True
        and native_result.get("noGenericPlsOrInference") is True
        and native_export.get("everyFormatPresentOnce") is True
        and native_export.get("nativeXlsx", {}).get("attempted") is True
        and native_export.get("nativeXlsx", {}).get("file", {}).get("size", 0) > 0
        and native_reopen.get("sameRunRestored") is True
        and archive_gsca.get("methodVersion") == "gsca_als_v2"
        and all(archive_gsca.get(name) is True for name in [
            "metricsMatch", "weightsMatch", "loadingsMatch", "pathsMatch", "rSquaredMatch",
        ]),
        "detail": "The packaged Windows app completed a real GSCA job, native XLSX export, strict archive check, explicit save, and same-run reopen.",
    },
    {
        "name": "legacy_v1_not_reinterpreted",
        "passed": "legacy/archive compatibility only" in legacy_doc
        and "must not be described as validated GSCA" in legacy_doc
        and "never upgraded or relabeled as v2" in legacy_doc,
        "detail": "Historical gsca_v1 remains a clearly disclosed preview record and is not current scientific evidence.",
    },
]

raise SystemExit(audit_method(
    "gsca_als_v2_native_promotion",
    "gsca",
    "Bounded joint global least-squares GSCA ALS v2 for standardized raw data, disjoint reflective/formative blocks, and recursive single-group paths; point estimates only.",
    ["gsca_als_v2_reference_report.json", "v247_native_desktop_visual_acceptance.json", "v247_tauri_native_acceptance_gsca.json"],
    ["GSCA_ALS_V2.md", "GSCA_V1.md"],
    checks,
))
