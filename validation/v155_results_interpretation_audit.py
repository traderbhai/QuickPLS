import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v155_results_interpretation_audit.json"
SMOKE = RESULTS / "v155_results_interpretation_smoke.json"

SOURCE_FILES = [
    ROOT / "src" / "App.tsx",
    ROOT / "src" / "components" / "RunHistory.tsx",
    ROOT / "src" / "styles.css",
    ROOT / "src" / "types.ts",
    ROOT / "src" / "store.ts",
]

REQUIRED_SNIPPETS = [
    "interpretationRegistry",
    "Threshold guidance",
    "What to report",
    "selectedDetailRow",
    "resultPrecision",
    "comparisonRunIds",
    "showInterpretationColumns",
    "Copyable report wording",
    "Path coefficient deltas",
    "R" + chr(0x00B2) + " deltas",
    "formatDisplayCell",
    "v1_5_5_results_interpretation_polish",
]

FORBIDDEN_SOURCE = [
    "R" + chr(0x00C2),
    "Q" + chr(0x00C2),
    "f" + chr(0x00C2),
    "R" + chr(0x00C3),
    chr(0xFFFD),
    "QuickPLS v" + "1.0.0 supported scope",
    "Comparison workflow\" detail=\"Run at least two compatible models",
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
        "registry_current_stage": registry.get("current_stage") == "v1_5_5_results_interpretation_polish",
        "registry_gate_registered": any(item.get("id") == "v1_5_5_results_interpretation_polish" for item in registry.get("slices", [])),
        "source_contracts_present": not missing,
        "no_results_mojibake_or_stale_scope": not forbidden,
        "smoke_report_exists": smoke is not None,
        "smoke_report_passed": bool(smoke and smoke.get("passed") is True),
        "smoke_has_interpretation_and_comparison": bool(smoke and smoke.get("checklist", {}).get("interpretation_has_report_wording") and smoke.get("checklist", {}).get("comparison_has_real_tables")),
        "roadmap_current_stage_updated": "v1_5_5_results_interpretation_polish" in read(ROOT / "crates" / "qpls-core" / "src" / "roadmap.rs"),
        "docs_present": (ROOT / "docs" / "V1_5_5_RESULTS_INTERPRETATION_POLISH.md").exists(),
    }
    result = {
        "schema_version": 1,
        "target": "QuickPLS v1.5.5 results interpretation polish audit",
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
