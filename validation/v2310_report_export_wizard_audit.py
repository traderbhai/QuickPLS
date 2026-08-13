import json
from datetime import UTC, datetime
from pathlib import Path

from lib.v2_ui_audit import (
    FORBIDDEN_MOJIBAKE,
    ROOT,
    RESULTS,
    frontend_boundary_check,
    no_forbidden_tokens,
    no_smartpls_equivalence,
    read_json,
    read_text,
    shared_v2_metadata_checks,
    write_result,
)

OUTPUT = "v2310_report_export_wizard_audit.json"
MILESTONE = "v2_31_0_report_export_wizard"
VERSION = "2.31.0"
LABEL = "v2.31 report export wizard"


def main() -> int:
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    report = read_text("src/components/ReportsWorkspace.tsx")
    css = read_text("src/styles.css")
    smoke_path = RESULTS / "v2310_report_export_wizard_smoke.json"
    smoke_payload = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}
    current = next((item for item in registry["slices"] if item["id"] == MILESTONE), None)
    source = "\n".join([report, css, topbar])

    checks = {
        **shared_v2_metadata_checks(
            version=VERSION,
            target_stage=MILESTONE,
            expected_label=LABEL,
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "scripts_registered": all(key in package.get("scripts", {}) for key in [
            "qpls:v2310:report-export-wizard-smoke",
            "qpls:v2310:report-export-wizard-audit",
            "qpls:v2310:report-export-wizard",
        ]),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "wizard_root_marker_present": 'data-v2310-report-wizard="true"' in report,
        "four_step_navigation_present": all(token in report for token in ["Select content", "Preview", "Document settings", "Export package"]),
        "step_panes_present": all(token in report for token in [
            'data-v2310-wizard-step="content"',
            'data-v2310-wizard-step="preview"',
            'data-v2310-wizard-step="settings"',
            'data-v2310-wizard-step="export"',
        ]),
        "preset_and_preview_flow_present": all(token in report for token in ["Reviewer pack", "Publication diagram preview", "ReportTablePreview"]),
        "export_actions_explicit": all(token in report for token in ["CSV tables", "HTML report", "XLSX workbook", "Print / PDF", "Model diagram SVG"]),
        "comparison_delegated_to_results": "Open Results Comparison" in report and "setResultWorkspaceState({ selectedTab: \"comparison\"" in report,
        "css_contract_present": all(token in css for token in [".report-wizard-nav", ".report-wizard-pane", ".report-wizard-footer"]),
        "no_mojibake_source": no_forbidden_tokens(source, FORBIDDEN_MOJIBAKE),
        "no_smartpls_equivalence_claim": no_smartpls_equivalence(source),
        "frontend_boundary": frontend_boundary_check(source),
        "docs_present": (ROOT / "docs" / "V2_31_0_REPORT_EXPORT_WIZARD.md").exists(),
        "delivery_docs_updated": MILESTONE in read_text("docs/DELIVERY_STATUS.md") and "v2.31.0 - Report Export Wizard" in read_text("docs/DEVELOPMENT_LEDGER.md"),
        "smoke_passed": smoke_payload.get("passed") is True,
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
