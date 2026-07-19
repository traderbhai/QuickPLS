"""QuickPLS v1.1 deterministic native desktop workflow smoke."""

import json
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v11_native_workflow_smoke.json"


def run(command: list[str], timeout: int = 240) -> dict:
    start = time.perf_counter()
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "elapsed_seconds": round(time.perf_counter() - start, 4),
        "stdout_tail": proc.stdout[-4000:],
        "stderr_tail": proc.stderr[-4000:],
    }


def read_json(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def file_nonempty(path: Path) -> bool:
    return path.exists() and path.stat().st_size > 0


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    desktop_test = run([
        "cargo",
        "test",
        "-p",
        "quickpls-desktop",
        "desktop_native_v11_workflow_smoke_import_run_save_reopen_and_export",
    ])
    ux_smoke = read_json(RESULTS / "v11_desktop_ux_smoke.json")
    native_smoke = read_json(RESULTS / "v11_native_desktop_smoke.json")
    release_exe = ROOT / "target" / "release" / "quickpls-desktop.exe"
    installer = ROOT / "target" / "release" / "bundle" / "nsis" / "QuickPLS_1.0.0_x64-setup.exe"
    checklist = {
        "desktop_workflow_rust_test_passed": desktop_test["passed"],
        "release_executable_exists": file_nonempty(release_exe),
        "installer_artifact_exists": file_nonempty(installer),
        "visual_desktop_ux_smoke_passed": ux_smoke.get("passed") is True,
        "native_launch_smoke_passed": native_smoke.get("passed") is True,
        "completed_run_overlay_evidence_passed": ux_smoke.get("metrics", {}).get("completedModelMetrics", {}).get("overlayStatusActive") is True,
        "report_export_overlay_evidence_passed": ux_smoke.get("metrics", {}).get("completedReportMetrics", {}).get("svgHasPaths") is True,
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.1 native workflow smoke",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "commands": [desktop_test],
        "artifacts": {
            "release_executable": str(release_exe),
            "installer": str(installer),
            "visual_smoke": "validation/results/v11_desktop_ux_smoke.json",
            "native_launch_smoke": "validation/results/v11_native_desktop_smoke.json",
        },
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
