import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v152_data_workspace_audit.json"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def json_file(path: str):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def main() -> int:
    data = read("src/components/DataWorkspace.tsx")
    helpers = read("src/domain/dataWorkspace.ts")
    helper_tests = read("src/domain/dataWorkspace.test.ts")
    status = read("src/components/StatusBar.tsx")
    styles = read("src/styles.css")
    package = json_file("package.json")
    registry = json_file("validation/development_slices.json")
    smoke_path = RESULTS / "v152_data_workspace_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}

    checks = {
        "version_is_152": package["version"] == "1.5.2" and '"version": "1.5.2"' in read("src-tauri/tauri.conf.json") and 'version = "1.5.2"' in read("Cargo.toml"),
        "artifact_label_is_152": "v1_5_2_data_workspace_hardening" in package["scripts"]["qpls:release:artifacts"],
        "scripts_registered": all(key in package["scripts"] for key in ["qpls:v152:data-smoke", "qpls:v152:data-audit", "qpls:v152:data-workspace"]),
        "single_sample_dataset_language": "Load Sample Dataset" in data and "Validation fixture" not in data,
        "mode_specific_import_guidance": all(text in data for text in ["Covariance matrix", "Correlation matrix", "Current loaded dataset preview", "Matrix imports require the native QuickPLS desktop application"]),
        "quality_helpers_present": all(text in helpers for text in ["dataQualitySummary", "detectPrefixGroups", "filteredColumns", "missingHeavyColumns"]),
        "quality_cards_present": all(text in data for text in ["Rows", "Variables", "Missing cells", "Nonnumeric", "Constant columns", "Header issues"]),
        "prefix_bridge_present": "Create Constructs From Prefixes" in data and "addConstructsFromIndicatorGroups" in data,
        "metadata_editor_sections": all(text in data for text in ["Essentials", "Bounds", "Reset draft", "Selected column metadata"]),
        "table_search_filter_present": all(text in data for text in ["Search variables in data preview", "Filter variables by metadata", "Showing {visibleColumns.length} of {dataset.columns.length} columns"]),
        "status_copy_clear": "Validated engine scope: v1.0" in status and "Engine 1.0.0 stable scope" not in status,
        "css_supports_data_sections": all(text in styles for text in [".data-import-panel", ".data-quality-grid", ".data-model-bridge", ".data-preview-panel", ".metadata-actions"]),
        "helper_tests_present": all(text in helper_tests for text in ["summarizes data quality", "detects construct-ready", "filters visible preview columns"]),
        "smoke_passed": bool(smoke.get("passed")),
        "registry_current_stage": registry["current_stage"] == "v1_5_2_data_workspace_hardening",
        "registry_slice_registered": any(item["id"] == "v1_5_2_data_workspace_hardening" and item["status"] == "validated" for item in registry["slices"]),
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.5.2 Data workspace hardening",
        "passed": all(checks.values()),
        "checklist": checks,
        "evidence": [
            "src/components/DataWorkspace.tsx",
            "src/domain/dataWorkspace.ts",
            "src/domain/dataWorkspace.test.ts",
            "validation/results/v152_data_workspace_smoke.json",
            "validation/results/screens/v152/data-workspace/",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
