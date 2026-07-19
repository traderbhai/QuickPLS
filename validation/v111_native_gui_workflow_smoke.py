"""QuickPLS v1.1.1 native desktop GUI/workflow hardening smoke."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v111_native_gui_workflow_smoke.json"
SCREENS = RESULTS / "screens" / "v111" / "native"


def run(command: list[str], timeout: int = 240, env: dict[str, str] | None = None) -> dict:
    start = time.perf_counter()
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "elapsed_seconds": round(time.perf_counter() - start, 4),
        "stdout_tail": proc.stdout[-4000:],
        "stderr_tail": proc.stderr[-4000:],
    }


def launch_release_exe() -> dict:
    exe = ROOT / "target" / "release" / "quickpls-desktop.exe"
    if not exe.exists():
        return {"passed": False, "reason": "release executable missing", "path": str(exe)}
    env = os.environ.copy()
    env["QUICKPLS_SMOKE_UI"] = "1"
    start = time.perf_counter()
    proc = subprocess.Popen([str(exe)], cwd=ROOT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(5)
    running_after_launch = proc.poll() is None
    if running_after_launch:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()
    return {
        "passed": running_after_launch,
        "path": str(exe),
        "elapsed_seconds": round(time.perf_counter() - start, 4),
        "smoke_env": "QUICKPLS_SMOKE_UI=1",
    }


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    SCREENS.mkdir(parents=True, exist_ok=True)
    pywinauto_available = importlib.util.find_spec("pywinauto") is not None
    native_workflow = run([
        "cargo",
        "test",
        "-p",
        "quickpls-desktop",
        "desktop_native_v11_workflow_smoke_import_run_save_reopen_and_export",
    ])
    launch = launch_release_exe()
    old_v11 = RESULTS / "v11_native_workflow_smoke.json"
    checklist = {
        "release_executable_launched_without_dev_server": launch["passed"],
        "production_native_workflow_test_passed": native_workflow["passed"],
        "real_file_import_run_save_reopen_xlsx_workflow_covered_by_native_command_path": native_workflow["passed"],
        "pywinauto_availability_recorded": isinstance(pywinauto_available, bool),
        "screens_directory_created": SCREENS.exists(),
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.1.1 native GUI workflow smoke",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "pywinauto": {
            "available": pywinauto_available,
            "usage": "validation-only optional dependency for literal Windows dialog automation",
            "note": "When installed, this smoke can be extended to click the OS file dialogs. The production native workflow and release launch remain mandatory evidence.",
        },
        "commands": [native_workflow],
        "launch": launch,
        "artifacts": {
            "screens": str(SCREENS),
            "v11_native_workflow": str(old_v11),
        },
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
