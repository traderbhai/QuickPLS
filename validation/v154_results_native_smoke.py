"""QuickPLS v1.5.4 native results-workspace smoke.

This wrapper keeps the v1.5.4 results milestone tied to the desktop release
surface without introducing runtime dependencies. It verifies that the
browser-based results evidence passed, that the release executable can launch
without a dev server, and that the existing native import/run/save/reopen/XLSX
desktop command path still passes.
"""

from __future__ import annotations

import json
import os
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v154_results_native_smoke.json"
SMOKE = RESULTS / "v154_results_workspace_smoke.json"
SCREENS = RESULTS / "screens" / "v154" / "results-workspace"


def run(command: list[str], timeout: int = 240) -> dict[str, object]:
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


def launch_release_exe() -> dict[str, object]:
    exe = ROOT / "target" / "release" / "quickpls-desktop.exe"
    if not exe.exists():
        return {"passed": False, "reason": "release executable missing", "path": str(exe)}
    env = os.environ.copy()
    env["QUICKPLS_SMOKE_UI"] = "1"
    start = time.perf_counter()
    proc = subprocess.Popen([str(exe)], cwd=ROOT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(5)
    running = proc.poll() is None
    if running:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()
    return {
        "passed": running,
        "path": str(exe),
        "elapsed_seconds": round(time.perf_counter() - start, 4),
        "smoke_env": "QUICKPLS_SMOKE_UI=1",
    }


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    smoke = json.loads(SMOKE.read_text(encoding="utf-8")) if SMOKE.exists() else None
    native_workflow = run([
        "cargo",
        "test",
        "-p",
        "quickpls-desktop",
        "desktop_native_v11_workflow_smoke_import_run_save_reopen_and_export",
    ])
    launch = launch_release_exe()
    screenshot_count = len(list(SCREENS.glob("*.png"))) if SCREENS.exists() else 0
    checklist = {
        "results_workspace_smoke_passed": bool(smoke and smoke.get("passed") is True),
        "diagram_result_linking_evidence_present": bool(smoke and smoke.get("checklist", {}).get("diagram_selection_highlights_result_rows") is True),
        "native_import_run_save_reopen_xlsx_workflow_passed": native_workflow["passed"],
        "release_executable_launches_without_dev_server": launch["passed"],
        "results_workspace_screenshots_present": screenshot_count >= 8,
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.5.4 native results workspace smoke",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "commands": [native_workflow],
        "launch": launch,
        "artifacts": {
            "browser_smoke": str(SMOKE),
            "screenshots": str(SCREENS),
            "screenshot_count": screenshot_count,
        },
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
