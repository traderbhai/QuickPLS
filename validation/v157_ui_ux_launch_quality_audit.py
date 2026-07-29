import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v157_ui_ux_launch_quality_audit.json"
SMOKE = RESULTS / "v157_ui_ux_launch_quality_smoke.json"
DOC = ROOT / "docs" / "V1_5_7_UI_UX_LAUNCH_QUALITY_AUDIT.md"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    registry = json.loads(read(ROOT / "validation" / "development_slices.json"))
    package = json.loads(read(ROOT / "package.json"))
    roadmap = read(ROOT / "crates" / "qpls-core" / "src" / "roadmap.rs")
    smoke = json.loads(read(SMOKE)) if SMOKE.exists() else None
    doc = read(DOC) if DOC.exists() else ""
    issue_ids = sorted(set(re.findall(r"UX-\d{3}", doc)))
    required_doc_sections = [
        "Severity Summary",
        "Screen Evidence",
        "Issue Register",
        "Implementation Sequence",
        "Acceptance Criteria",
        "Non-Engine Boundary",
    ]
    required_scripts = [
        "qpls:v157:ui-ux-smoke",
        "qpls:v157:ui-ux-audit",
        "qpls:v157:ui-ux-launch-audit",
    ]
    checklist = {
        "registry_current_stage": registry.get("current_stage") == "v1_5_7_ui_ux_launch_quality_audit",
        "registry_slice_registered": any(
            item.get("id") == "v1_5_7_ui_ux_launch_quality_audit"
            and item.get("status") == "validated"
            and item.get("stable_output") is True
            for item in registry.get("slices", [])
        ),
        "roadmap_current_stage_updated": "v1_5_7_ui_ux_launch_quality_audit" in roadmap,
        "scripts_registered": all(script in package.get("scripts", {}) for script in required_scripts),
        "doc_present": DOC.exists(),
        "doc_sections_present": all(section in doc for section in required_doc_sections),
        "issue_register_has_minimum_depth": len(issue_ids) >= 60,
        "issue_register_has_priorities": all(text in doc for text in ["P0", "P1", "P2", "P3"]),
        "issue_register_has_screen_coverage": all(screen in doc for screen in ["Home", "Data", "Model", "Setup", "Run", "Results", "Report"]),
        "issue_register_has_category_coverage": all(category in doc for category in ["layout", "tables", "diagram", "copy", "accessibility", "workflow"]),
        "smoke_report_exists": smoke is not None,
        "smoke_report_passed": bool(smoke and smoke.get("passed")),
        "screenshots_preserved": bool(smoke and len(smoke.get("screenshots", [])) >= 15),
        "smoke_issue_count_matches_doc": bool(smoke and smoke.get("issue_count", 0) >= 60 and smoke.get("issue_count", 0) <= len(issue_ids)),
        "recommended_sequence_recorded": bool(smoke and len(smoke.get("recommended_sequence", [])) >= 6),
        "non_engine_boundary_explicit": "No statistical engine" in doc and "numerical fingerprints" in doc,
    }
    result = {
        "schema_version": 1,
        "target": "QuickPLS v1.5.7 UI/UX launch-quality audit",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "issue_ids_detected": issue_ids,
        "smoke_report": str(SMOKE),
        "doc": str(DOC),
    }
    OUTPUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
