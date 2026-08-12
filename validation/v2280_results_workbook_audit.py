import json
from datetime import UTC, datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v2280_results_workbook_audit.json"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def main() -> int:
    package = json.loads(read("package.json"))
    registry = json.loads(read("validation/development_slices.json"))
    run_history = read("src/components/RunHistory.tsx")
    css = read("src/styles.css")
    roadmap = read("crates/qpls-core/src/roadmap.rs")
    doc = ROOT / "docs" / "V2_28_0_RESULTS_WORKBOOK_REDESIGN.md"
    smoke = RESULTS / "v2280_results_workbook_smoke.json"
    smoke_payload = json.loads(smoke.read_text(encoding="utf-8")) if smoke.exists() else {}

    current = next((item for item in registry["slices"] if item["id"] == "v2_28_0_results_workbook_redesign"), None)
    checks = {
        "package_version_2280": package.get("version") == "2.28.0",
        "scripts_registered": all(key in package.get("scripts", {}) for key in [
            "qpls:v2280:results-workbook-smoke",
            "qpls:v2280:results-workbook-audit",
            "qpls:v2280:results-workbook",
        ]),
        "release_artifact_label_current": "v2_28_0_results_workbook_redesign" in package.get("scripts", {}).get("qpls:release:artifacts", ""),
        "registry_current_stage": registry.get("current_stage") == "v2_28_0_results_workbook_redesign",
        "registry_slice_validated": bool(current and current.get("status") == "validated" and current.get("stable_output") is True),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "roadmap_current_stage_updated": '"v2_28_0_results_workbook_redesign"' in roadmap,
        "component_marker_present": 'data-v228-results-workbook="true"' in run_history,
        "sticky_run_header_present": "results-v228-run-header" in run_history,
        "workbook_split_present": "results-v228-workbook-body" in run_history and "results-v228-table-area" in run_history,
        "detail_pane_present": "ResultsV228DetailPane" in run_history and "results-v228-detail-pane" in run_history,
        "findings_lanes_present": "ResultsV228FindingLane" in run_history and "Must address" in run_history and "Review" in run_history and "Info" in run_history,
        "provenance_footer_present": "ResultsV228ProvenanceFooter" in run_history and "results-v228-provenance-footer" in run_history,
        "method_confidence_reused": "MethodConfidencePanel" in run_history,
        "result_tabs_complete": all(label in run_history for label in [
            "Overview", "Measurement", "Structural", "Validity", "Inference",
            "Prediction", "Groups", "Diagnostics", "Interpretation", "Comparison",
        ]),
        "css_contract_present": all(token in css for token in [
            ".results-v228-workspace",
            ".results-v228-workbook-body",
            ".results-v228-detail-pane",
            ".results-v228-finding-lane",
            ".results-v228-provenance-footer",
        ]),
        "no_tauri_or_backend_invocation_added": "invoke(" not in run_history,
        "no_r2_mojibake_source": "RÃ" not in run_history and "RÂ²" not in run_history and "RÃ" not in css and "RÂ²" not in css,
        "docs_present": doc.exists(),
        "delivery_docs_updated": "v2_28_0_results_workbook_redesign" in read("docs/DELIVERY_STATUS.md") and "v2.28.0 - Results Workbook Redesign" in read("docs/DEVELOPMENT_LEDGER.md"),
        "smoke_passed": smoke_payload.get("passed") is True,
    }

    issues = [
        {"id": key, "severity": "high", "detail": f"Failed check: {key}"}
        for key, passed in checks.items()
        if not passed
    ]
    payload = {
        "passed": not issues,
        "milestone": "v2_28_0_results_workbook_redesign",
        "generatedAt": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "checks": checks,
        "issues": issues,
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(json.dumps(payload, indent=2))
    return 0 if payload["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
