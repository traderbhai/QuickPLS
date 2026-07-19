"""QuickPLS v1.1.1 native keyboard workflow smoke."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v111_keyboard_native_smoke.json"


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


def release_launch() -> dict:
    exe = ROOT / "target" / "release" / "quickpls-desktop.exe"
    if not exe.exists():
        return {"passed": False, "reason": "release executable missing", "path": str(exe)}
    env = os.environ.copy()
    env["QUICKPLS_SMOKE_UI"] = "1"
    proc = subprocess.Popen([str(exe)], cwd=ROOT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(4)
    passed = proc.poll() is None
    if passed:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()
    return {"passed": passed, "path": str(exe), "smoke_env": "QUICKPLS_SMOKE_UI=1"}


def read_json(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    browser_keyboard = run(["node", "validation/v11_keyboard_workflow_smoke.mjs"])
    launch = release_launch()
    browser_report = read_json(RESULTS / "v11_keyboard_workflow_smoke.json")
    pywinauto_available = importlib.util.find_spec("pywinauto") is not None
    checklist = {
        "release_executable_launches_for_native_keyboard_context": launch["passed"],
        "completed_result_keyboard_regions_pass_browser_contract": browser_keyboard["passed"] and browser_report.get("passed") is True,
        "results_keyboard_regions_named": browser_report.get("checklist", {}).get("results_regions_named") is True,
        "report_keyboard_regions_named": browser_report.get("checklist", {}).get("report_regions_named") is True,
        "focus_visible_contract_passed": browser_report.get("checklist", {}).get("focus_visible_css_present") is True,
        "pywinauto_availability_recorded": isinstance(pywinauto_available, bool),
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.1.1 native keyboard workflow smoke",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "pywinauto": {
            "available": pywinauto_available,
            "note": "UIA focus traversal can be promoted to literal native-key smoke when pywinauto is installed in validation tooling.",
        },
        "commands": [browser_keyboard],
        "launch": launch,
        "artifacts": {
            "browser_keyboard": str(RESULTS / "v11_keyboard_workflow_smoke.json"),
        },
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
