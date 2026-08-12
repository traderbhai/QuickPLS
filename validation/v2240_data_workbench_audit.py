import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "v2240_data_workbench_audit.json"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def main() -> int:
    package = json.loads(read("package.json"))
    slices = json.loads(read("validation/development_slices.json"))
    data = read("src/components/DataWorkspace.tsx")
    styles = read("src/styles.css")
    roadmap = read("crates/qpls-core/src/roadmap.rs")

    checks = {
        "version is 2.24.0": package.get("version") == "2.24.0",
        "current stage is v2.24": slices.get("current_stage") == "v2_24_0_data_workbench_redesign",
        "roadmap expects v2.24": "v2_24_0_data_workbench_redesign" in roadmap,
        "scripts exist": all(script in package.get("scripts", {}) for script in ["qpls:v2240:data-workbench-smoke", "qpls:v2240:data-workbench-audit", "qpls:v2240:data-workbench"]),
        "data is tabbed workbench": all(token in data for token in ["Data View", "Variable View", "Import History", "Data Quality", "Notes"]),
        "spss-like variable view exists": "Variable metadata table" in data and "one row per variable" in data,
        "quality view has variable issue table": "Variable issues" in data and "missingHeavyColumns" in data,
        "import workflow stays native-aware": "importNativeDataset" in data and "Matrix imports require the native QuickPLS desktop application" in data,
        "styles define desktop data shell": all(token in styles for token in [".data-v224-tabs", ".data-v224-variable-grid", ".data-v224-issue-table"]),
        "documentation exists": (ROOT / "docs" / "V2_24_0_DATA_WORKBENCH_REDESIGN.md").exists(),
        "no numerical backend references": not any(token in data for token in ["F_ml", "qpls-estimation", "AnalysisResultEnvelope"]),
    }

    result = {
        "passed": all(checks.values()),
        "milestone": "v2_24_0_data_workbench_redesign",
        "checks": checks,
        "failed": [name for name, passed in checks.items() if not passed],
    }
    RESULT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    if not result["passed"]:
        print(json.dumps(result, indent=2))
        return 1
    print(f"v2.24 data workbench audit passed: {RESULT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
