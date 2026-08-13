import json
from datetime import UTC, datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v2290_research_tables_audit.json"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def main() -> int:
    package = json.loads(read("package.json"))
    registry = json.loads(read("validation/development_slices.json"))
    run_history = read("src/components/RunHistory.tsx")
    css = read("src/styles.css")
    roadmap = read("crates/qpls-core/src/roadmap.rs")
    smoke_path = RESULTS / "v2290_research_tables_smoke.json"
    smoke_payload = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}
    current = next((item for item in registry["slices"] if item["id"] == "v2_29_0_research_table_system"), None)

    checks = {
        "package_version_2290": package.get("version") == "2.29.0",
        "scripts_registered": all(key in package.get("scripts", {}) for key in [
            "qpls:v2290:research-tables-smoke",
            "qpls:v2290:research-tables-audit",
            "qpls:v2290:research-tables",
        ]),
        "release_artifact_label_current": "v2_29_0_research_table_system" in package.get("scripts", {}).get("qpls:release:artifacts", ""),
        "registry_current_stage": registry.get("current_stage") == "v2_29_0_research_table_system",
        "registry_slice_validated": bool(current and current.get("status") == "validated" and current.get("stable_output") is True),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "roadmap_current_stage_updated": '"v2_29_0_research_table_system"' in roadmap,
        "research_table_marker_present": 'data-v229-research-table="true"' in run_history,
        "shared_toolbar_present": "research-table-toolbar" in run_history,
        "selected_row_copy_present": "Copy selected" in run_history and "copySelectedRows" in run_history,
        "select_all_present": "Select all visible" in run_history and "toggleAllVisibleRows" in run_history,
        "local_table_search_present": "localSearch" in run_history and "aria-label={`Search ${title}`}" in run_history,
        "precision_and_density_controls_present": "Precision for ${title}" in run_history and "Comfortable rows" in run_history and "Compact rows" in run_history,
        "row_detail_present": "Selected row" in run_history and "resultState.selectedDetailRow" in run_history,
        "prediction_tables_converted": "PLSpredict target metrics" in run_history and "CVPAT paired loss comparisons" in run_history and "<SectionTable" in run_history,
        "split_wide_tables_present": all(token in run_history for token in [
            "Mediation effects summary",
            "Mediation inference",
            "Mediation classification",
            "Bootstrap estimates",
            "Percentile confidence intervals",
            "BCa confidence intervals",
            "Bootstrap-t confidence intervals",
            '<HtmtTable label="HTMT+"' ,
            "construct pairs",
        ]),
        "css_contract_present": all(token in css for token in [
            ".v2290-research-table",
            ".research-table-toolbar",
            ".research-table-select-cell",
            "th:nth-child(2)",
            "tbody tr.active-result-row td:nth-child(2)",
        ]),
        "no_tauri_or_backend_invocation_added": "invoke(" not in run_history,
        "no_r2_mojibake_source": all(bad not in run_history and bad not in css for bad in ["RÃ", "RÂ²", "Ãƒ", "Ã‚"]),
        "docs_present": (ROOT / "docs" / "V2_29_0_RESEARCH_TABLE_SYSTEM.md").exists(),
        "delivery_docs_updated": "v2_29_0_research_table_system" in read("docs/DELIVERY_STATUS.md") and "v2.29.0 - Research Table System" in read("docs/DEVELOPMENT_LEDGER.md"),
        "smoke_passed": smoke_payload.get("passed") is True,
    }

    issues = [
        {"id": key, "severity": "high", "detail": f"Failed check: {key}"}
        for key, passed in checks.items()
        if not passed
    ]
    payload = {
        "passed": not issues,
        "milestone": "v2_29_0_research_table_system",
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
