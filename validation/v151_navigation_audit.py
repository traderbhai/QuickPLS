import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v151_navigation_audit.json"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def json_file(path: str):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def main() -> int:
    nav = read("src/components/NavRail.tsx")
    workflow = read("src/components/WorkflowStrip.tsx")
    setup = read("src/components/AnalysisCatalog.tsx")
    results = read("src/components/RunHistory.tsx")
    data = read("src/components/DataWorkspace.tsx")
    home = read("src/components/OnboardingWorkspace.tsx")
    commands = read("src/components/ProductivityOverlays.tsx")
    package = json_file("package.json")
    registry = json_file("validation/development_slices.json")
    smoke_path = RESULTS / "v151_navigation_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}

    checks = {
        "version_is_151": package["version"] == "1.5.1" and '"version": "1.5.1"' in read("src-tauri/tauri.conf.json") and 'version = "1.5.1"' in read("Cargo.toml"),
        "artifact_label_is_151": "v1_5_1_navigation_workspace_hardening" in package["scripts"]["qpls:release:artifacts"],
        "rail_sequence_updated": all(label in nav for label in ['label: "Home"', 'label: "Data"', 'label: "Model"', 'label: "Setup"', 'label: "Run"', 'label: "Results"', 'label: "Report"']),
        "rail_removed_validate_groups": 'label: "Validate"' not in nav and 'label: "Groups"' not in nav,
        "workflow_uses_setup": 'label: "Setup"' in workflow and 'label: "Validate"' not in workflow,
        "home_hardened": "Current project" in home and "quickpls:save-project" in home,
        "data_has_next_step": "Next step after data import" in data and "Build model" in data,
        "setup_has_group_workflows": "Group and prediction workflows" in setup and "Results > Groups" in setup,
        "results_has_groups_bridge": "Groups and segmentation results" in results and "Configure group workflow in Setup" in results,
        "command_palette_updated": "Open Setup" in commands and "Open Results: Groups" in commands and "Open Publication Report" in commands,
        "no_mojibake": "RÂ²" not in "".join([nav, workflow, setup, results, data, home, commands]),
        "smoke_passed": bool(smoke.get("passed")),
        "registry_current_stage": registry["current_stage"] == "v1_5_1_navigation_workspace_hardening",
        "registry_slice_registered": any(item["id"] == "v1_5_1_navigation_workspace_hardening" and item["status"] == "validated" for item in registry["slices"]),
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.5.1 navigation workspace hardening",
        "passed": all(checks.values()),
        "checklist": checks,
        "evidence": [
            "src/components/NavRail.tsx",
            "src/components/WorkflowStrip.tsx",
            "src/components/AnalysisCatalog.tsx",
            "src/components/RunHistory.tsx",
            "validation/results/v151_navigation_smoke.json",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
