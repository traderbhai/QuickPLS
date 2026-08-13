import json
from datetime import UTC, datetime
from pathlib import Path

from lib.v2_ui_audit import (
    FORBIDDEN_MOJIBAKE,
    ROOT,
    no_forbidden_tokens,
    no_smartpls_equivalence,
    read_json,
    read_text,
    write_result,
)

OUTPUT = "v236_native_ui_spec_audit.json"
MILESTONE = "v2_36_0_native_desktop_ui_spec_and_component_plan"
DOC = "docs/V2_36_0_NATIVE_DESKTOP_UI_SPEC_AND_COMPONENT_PLAN.md"

REQUIRED_SURFACES = [
    "Home / Project Manager",
    "Data Workbench",
    "Model Workbench",
    "Setup / Method Applicability Center",
    "Run / Calculation Monitor",
    "Results Workbook",
    "Report / Export Wizard",
    "Trust Center / Evidence Workbench",
    "Settings / Preferences",
    "Sample Project Gallery",
    "Import Data Dialog",
    "Calculation Setup Dialog",
    "Method Scope / Evidence Dialog",
    "Export Options Dialog",
    "Help / Shortcuts Dialog",
    "Focus Diagram Mode",
]

REQUIRED_COMPONENTS = [
    "DesktopShell",
    "DesktopMenuBar",
    "DesktopCommandToolbar",
    "WorkflowRail",
    "DockedPane",
    "PropertySheet",
    "ResearchTable",
    "StatusBar",
    "EvidenceDrawer",
    "DialogManager",
    "TaskDialog",
    "WizardDialog",
    "CommandRegistry",
    "MessageCenter",
]

REQUIRED_BOUNDARIES = [
    "Do not change statistical engines.",
    "Do not change formulas.",
    "Do not change result schemas.",
    "Do not change project archive format.",
    "Do not change validation tolerances.",
    "Do not change numerical fingerprints.",
    "Preserve SEM designer core behavior",
]


def main() -> int:
    registry = read_json("validation/development_slices.json")
    package = read_json("package.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    active = read_text("docs/V2_ACTIVE_MILESTONE.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    doc_path = ROOT / DOC
    doc = doc_path.read_text(encoding="utf-8") if doc_path.exists() else ""
    current = next((item for item in registry["slices"] if item["id"] == MILESTONE), None)

    checks = {
        "doc_present": doc_path.exists(),
        "current_stage_registered": registry.get("current_stage") == MILESTONE,
        "registry_slice_validated": bool(current and current.get("status") == "validated"),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "roadmap_current_stage_updated": MILESTONE in roadmap,
        "package_scripts_registered": all(
            key in package.get("scripts", {})
            for key in [
                "qpls:v236:native-ui-spec-audit",
                "qpls:v236:native-ui-spec",
            ]
        ),
        "all_surfaces_specified": all(surface in doc for surface in REQUIRED_SURFACES),
        "all_shared_components_specified": all(component in doc for component in REQUIRED_COMPONENTS),
        "all_boundaries_specified": all(boundary in doc for boundary in REQUIRED_BOUNDARIES),
        "mockup_paths_recorded": doc.count(".png`") >= len(REQUIRED_SURFACES),
        "implementation_order_present": all(
            milestone in doc
            for milestone in [
                "v2_32_0_trust_center_evidence_workbench",
                "v2_33_0_settings_preferences_environment",
                "v2_34_0_desktop_polish_accessibility_qa",
                "v2_35_0_native_desktop_release_candidate",
            ]
        ),
        "no_mojibake": no_forbidden_tokens("\n".join([doc, active, delivery, ledger]), FORBIDDEN_MOJIBAKE),
        "no_smartpls_equivalence_claim": no_smartpls_equivalence("\n".join([doc, active, delivery, ledger])),
        "docs_updated": MILESTONE in active and MILESTONE in delivery and "v2.36.0 - Native Desktop UI Spec" in ledger,
    }

    issues = [
        {"id": key, "severity": "high", "detail": f"Failed check: {key}"}
        for key, passed in checks.items()
        if not passed
    ]
    payload = {
        "passed": not issues,
        "milestone": MILESTONE,
        "generatedAt": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "checks": checks,
        "issues": issues,
    }
    write_result(OUTPUT, payload)
    print(json.dumps(payload, indent=2))
    return 0 if payload["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
