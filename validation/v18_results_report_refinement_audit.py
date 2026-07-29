import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUT = RESULTS / "v18_results_report_refinement_audit.json"
REQUIRED = [
    RESULTS / "v18_real_dataset_results_report_audit.json",
    RESULTS / "v18_results_clutter_smoke.json",
    RESULTS / "v18_table_layouts_smoke.json",
    RESULTS / "v18_interpretation_wording_smoke.json",
    RESULTS / "v18_report_export_flow_smoke.json",
]


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    package = read_json(ROOT / "package.json")
    slices = read_json(ROOT / "validation" / "development_slices.json")
    missing = [str(path.relative_to(ROOT)) for path in REQUIRED if not path.exists()]
    failed = []
    for path in REQUIRED:
        if path.exists() and not read_json(path).get("passed"):
            failed.append(str(path.relative_to(ROOT)))
    checks = {
        "package_version_1_8_0": package.get("version") == "1.8.0",
        "artifact_label_v18": "v1_8_results_report_refinement_real_user_testing" in package["scripts"].get("qpls:release:artifacts", ""),
        "current_stage_v18": slices.get("current_stage") == "v1_8_results_report_refinement_real_user_testing",
        "registry_slice_exists": any(item.get("id") == "v1_8_results_report_refinement_real_user_testing" for item in slices.get("slices", [])),
        "required_artifacts_present": not missing,
        "required_artifacts_pass": not failed,
    }
    payload = {
        "passed": all(checks.values()),
        "milestone": "v1_8_results_report_refinement_real_user_testing",
        "checks": checks,
        "missing": missing,
        "failed": failed,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(f"wrote {OUT}")
    if not payload["passed"]:
        raise SystemExit(1)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
