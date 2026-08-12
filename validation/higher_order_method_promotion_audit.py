#!/usr/bin/env python3
import json
from pathlib import Path

from final_method_promotion_common import audit_method


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


native_report = load_json(RESULTS / "v247_tauri_native_acceptance.json")
visual_report = load_json(RESULTS / "v247_native_desktop_visual_acceptance.json")
native_checks = native_report.get("checks", {})
hoc_dialog = native_checks.get("hocDialog", {})
hoc_result = native_checks.get("hocResult", {})
hoc_export = native_checks.get("hocExport", {})
hoc_reopen = native_checks.get("hocSaveReopen", {})
responsive_checks = visual_report.get("checks", {}).get("higherOrderAuthoring", [])

source_contracts = {
    "dialog": (ROOT / "src" / "native" / "NativeHigherOrderDialog.tsx").read_text(encoding="utf-8"),
    "scope": (ROOT / "src" / "native" / "nativeHigherOrder.ts").read_text(encoding="utf-8"),
    "results": (ROOT / "src" / "native" / "nativeResults.ts").read_text(encoding="utf-8"),
    "persistence": (ROOT / "crates" / "qpls-project" / "src" / "lib.rs").read_text(encoding="utf-8"),
}

native_source_passed = (
    "Create higher-order construct" in source_contracts["dialog"]
    and "Reflective–reflective disjoint two-stage" in source_contracts["scope"]
    and "exactly one HOC-to-outcome structural path" in source_contracts["scope"]
    and "hoc_component_relationships" in source_contracts["results"]
    and "hoc_structural_paths" in source_contracts["results"]
    and "hoc_scope" in source_contracts["results"]
    and "validate_higher_order_contract" in source_contracts["persistence"]
    and "runner_generated_two_stage_hoc_appends_round_trips_and_rejects_contract_tampering" in source_contracts["persistence"]
)

responsive_passed = (
    visual_report.get("passed") is True
    and len(responsive_checks) == 3
    and all(
        check.get("fixture") == {"variables": 3, "models": 0}
        and check.get("constructCount") == 3
        and check.get("componentCount") == 3
        and check.get("selectedComponents")
        and len(check["selectedComponents"]) == 2
        and check.get("exactBoundedScope") is True
        and check.get("unsupportedInferenceControls") == 0
        and check.get("createEnabled") is True
        and check.get("dialogOverflow") is False
        and check.get("pageOverflow") is False
        and check.get("dialogClosed") is True
        and check.get("focusRestored") is True
        for check in responsive_checks
    )
)

packaged_passed = (
    native_report.get("passed") is True
    and hoc_dialog.get("createEnabled") is True
    and hoc_dialog.get("inferenceControls") == 0
    and hoc_result.get("initialSelectedTable") == "hoc_component_relationships"
    and hoc_result.get("component", {}).get("rows") == 2
    and hoc_result.get("structural", {}).get("rows") == 1
    and hoc_result.get("scope", {}).get("rows") == 1
    and hoc_result.get("noTechnicalIds") is True
    and hoc_result.get("noPlaceholder") is True
    and hoc_export.get("nativeXlsx", {}).get("attempted") is True
    and hoc_export.get("nativeXlsx", {}).get("file", {}).get("isFile") is True
    and hoc_reopen.get("sameRunRestored") is True
    and hoc_reopen.get("componentRows") == 2
    and hoc_reopen.get("structuralRows") == 1
    and hoc_reopen.get("scopeRows") == 1
)


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "higher_order",
    "Backend-qualified repeated-indicator, two-stage, and documented hybrid estimation; packaged native authoring is limited to one reflective-reflective disjoint two-stage point-estimate HOC.",
    [
        "higher_order_reference_report.json",
        "higher_order_metamorphic_report.json",
        "higher_order_two_stage_reference_report.json",
        "higher_order_hybrid_reference_report.json",
        "higher_order_hybrid_guard_report.json",
        "v05_extended_pls_evidence.json",
        "v247_native_desktop_visual_acceptance.json",
        "v247_tauri_native_acceptance.json",
    ],
    ["PLS_HIGHER_ORDER_V1.md"],
    [
        {"name": "hoc_native_source_contract", "passed": native_source_passed, "detail": "Native source contains bounded authoring, readiness, result-table, strict persistence, and runner-backed tamper contracts."},
        {"name": "hoc_responsive_authoring", "passed": responsive_passed, "detail": "Three production-bundle viewports author ordinary components and verify the exact bounded HOC dialog without fabricated results."},
        {"name": "hoc_packaged_workflow", "passed": packaged_passed, "detail": "A genuine packaged run exposes component/path/scope tables, real XLSX output, strict archive shape, explicit save, and same-run reopen."},
    ],
))
