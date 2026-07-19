"""QuickPLS v1.1 combined desktop UX completion audit."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v11_desktop_ux_audit.json"


def read_json(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def file_nonempty(path: Path) -> bool:
    return path.exists() and path.stat().st_size > 0


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    ux = read_json(RESULTS / "v11_desktop_ux_smoke.json")
    native = read_json(RESULTS / "v11_native_desktop_smoke.json")
    workflow = read_json(RESULTS / "v11_native_workflow_smoke.json")
    parity = read_json(RESULTS / "v11_report_export_parity.json")
    keyboard = read_json(RESULTS / "v11_keyboard_workflow_smoke.json")
    docs = (ROOT / "docs" / "V1_1_UX_AUDIT.md").read_text(encoding="utf-8")
    screenshots = [
        RESULTS / "screens" / "v11" / name
        for name in [
            "v11_01_data_1440x900.png",
            "v11_02_model_1440x900.png",
            "v11_03_validate_1440x900.png",
            "v11_04_run_1440x900.png",
            "v11_05_results_1440x900.png",
            "v11_06_report_1440x900.png",
            "v11_07_results_completed_1440x900.png",
            "v11_08_model_completed_overlay_1440x900.png",
            "v11_09_report_completed_1440x900.png",
        ]
    ]
    checklist = {
        "desktop_visual_smoke_passed": ux.get("passed") is True,
        "native_launch_smoke_passed": native.get("passed") is True,
        "native_workflow_smoke_passed": workflow.get("passed") is True,
        "report_export_parity_passed": parity.get("passed") is True,
        "keyboard_workflow_smoke_passed": keyboard.get("passed") is True,
        "screenshots_exist": all(file_nonempty(path) for path in screenshots),
        "audit_doc_records_completion": "## v1.1 Completion Closure" in docs and "No remaining v1.1 desktop release blockers" in docs,
        "mobile_non_gating_recorded": "Mobile is non-gating" in docs,
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.1 desktop UX completion audit",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "artifacts": {
            "visual_smoke": "validation/results/v11_desktop_ux_smoke.json",
            "native_launch": "validation/results/v11_native_desktop_smoke.json",
            "native_workflow": "validation/results/v11_native_workflow_smoke.json",
            "report_parity": "validation/results/v11_report_export_parity.json",
            "keyboard_smoke": "validation/results/v11_keyboard_workflow_smoke.json",
            "screenshots": "validation/results/screens/v11/",
            "audit_doc": "docs/V1_1_UX_AUDIT.md",
        },
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
