import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v154_results_workspace_audit.json"
SMOKE = RESULTS / "v154_results_workspace_smoke.json"

SOURCE_FILES = [
    ROOT / "src" / "App.tsx",
    ROOT / "src" / "components" / "RunHistory.tsx",
    ROOT / "src" / "styles.css",
    ROOT / "src" / "data" / "smokeRun.ts",
]

OLD_SCOPE_TEXT = "QuickPLS v" + "1.0.0 supported scope"

FORBIDDEN = [
    "RÂ",
    "QÂ",
    "fÂ",
    "RÃ",
    "ï¿½",
    OLD_SCOPE_TEXT,
    "Permutation path tests",
    "summary results",
]

REQUIRED_SOURCE_SNIPPETS = [
    "Total effects",
    "Outer loadings and weights",
    "Construct reliability and convergent validity",
    "Run provenance",
    "active-result-row",
    "selectEdge",
    "workspace-page > .action-strip:has(.result-search)",
    "result-table-scroll th:first-child",
]


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    combined = "\n".join(path.read_text(encoding="utf-8") for path in SOURCE_FILES)
    missing = [snippet for snippet in REQUIRED_SOURCE_SNIPPETS if snippet not in combined]
    forbidden_hits = [needle for needle in FORBIDDEN if needle in combined]
    smoke = json.loads(SMOKE.read_text(encoding="utf-8")) if SMOKE.exists() else None
    checklist = {
        "required_source_contracts_present": not missing,
        "no_forbidden_stale_or_mojibake_text": not forbidden_hits,
        "smoke_report_exists": smoke is not None,
        "smoke_report_passed": bool(smoke and smoke.get("passed") is True),
        "smoke_has_all_expected_tabs": bool(smoke and {"summary", "measurement", "structural", "quality", "inference", "prediction", "groups", "diagnostics"}.issubset(set(smoke.get("tabs", {}).keys()))),
    }
    result = {
        "schema_version": 1,
        "target": "QuickPLS results workspace source and smoke audit",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "missing_required_snippets": missing,
        "forbidden_hits": forbidden_hits,
        "smoke_report": str(SMOKE),
    }
    OUTPUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
