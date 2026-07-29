import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v156_result_interpretation_audit.json"
SMOKE = RESULTS / "v156_result_interpretation_smoke.json"

SOURCE_FILES = [
    ROOT / "src" / "domain" / "resultInterpretation.ts",
    ROOT / "src" / "domain" / "resultInterpretation.test.ts",
    ROOT / "src" / "components" / "RunHistory.tsx",
    ROOT / "src" / "components" / "ReportsWorkspace.tsx",
    ROOT / "src" / "styles.css",
]

REQUIRED_SNIPPETS = [
    "InterpretationFinding",
    "buildResultInterpretation",
    "diagramAdvisorFindings",
    "rowSpecificInterpretation",
    "FindingCards",
    "FindingChecklist",
    "Report wording",
    "Include interpretation notes",
    "copyableInterpretationText",
    "v1_5_6_result_specific_interpretation_engine",
]

FORBIDDEN_SOURCE = [
    "R" + chr(0x00C2),
    "Q" + chr(0x00C2),
    "f" + chr(0x00C2),
    chr(0xFFFD),
    "identical to SmartPLS",
    "equivalent to SmartPLS",
    "same as SmartPLS",
]


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    combined = "\n".join(read(path) for path in SOURCE_FILES)
    registry = json.loads(read(ROOT / "validation" / "development_slices.json"))
    smoke = json.loads(read(SMOKE)) if SMOKE.exists() else None
    missing = [snippet for snippet in REQUIRED_SNIPPETS if snippet not in combined and snippet not in json.dumps(registry)]
    forbidden = [snippet for snippet in FORBIDDEN_SOURCE if snippet in combined]
    checklist = {
        "registry_current_stage": registry.get("current_stage") == "v1_5_6_result_specific_interpretation_engine",
        "registry_gate_registered": any(item.get("id") == "v1_5_6_result_specific_interpretation_engine" for item in registry.get("slices", [])),
        "source_contracts_present": not missing,
        "no_mojibake_or_smartpls_overclaim": not forbidden,
        "unit_test_file_present": (ROOT / "src" / "domain" / "resultInterpretation.test.ts").exists(),
        "smoke_report_exists": smoke is not None,
        "smoke_report_passed": bool(smoke and smoke.get("passed") is True),
        "smoke_checks_value_specific_ui": bool(smoke and smoke.get("checklist", {}).get("overview_has_computed_findings") and smoke.get("checklist", {}).get("row_detail_uses_values")),
        "smoke_checks_report_option": bool(smoke and smoke.get("checklist", {}).get("report_has_interpretation_notes_option")),
        "roadmap_current_stage_updated": "v1_5_6_result_specific_interpretation_engine" in read(ROOT / "crates" / "qpls-core" / "src" / "roadmap.rs"),
        "docs_present": (ROOT / "docs" / "V1_5_6_RESULT_SPECIFIC_INTERPRETATION_ENGINE.md").exists(),
    }
    result = {
        "schema_version": 1,
        "target": "QuickPLS v1.5.6 result-specific interpretation audit",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "missing_required_snippets": missing,
        "forbidden_hits": forbidden,
        "smoke_report": str(SMOKE),
    }
    OUTPUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
