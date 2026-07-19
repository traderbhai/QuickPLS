"""QuickPLS v1.1 native desktop smoke audit.

This verifies that the release desktop executable was rebuilt with the current
frontend bundle and still launches without a dev server. Deep visual coverage
is provided by the v1.1 Playwright desktop-width smoke artifact.
"""

import json
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v11_native_desktop_smoke.json"


def file_status(path: Path) -> dict:
    if not path.exists():
        return {"exists": False, "path": str(path)}
    stat = path.stat()
    return {
        "exists": True,
        "path": str(path),
        "size": stat.st_size,
        "mtime": stat.st_mtime,
        "last_write_time": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime(stat.st_mtime)),
    }


def newest_dist_asset() -> Path | None:
    assets = [path for path in (ROOT / "dist").rglob("*") if path.is_file()]
    if not assets:
        return None
    return max(assets, key=lambda path: path.stat().st_mtime)


def launch_release_exe(exe: Path) -> dict:
    if not exe.exists():
        return {"passed": False, "path": str(exe), "reason": "release executable missing"}
    proc = None
    try:
        proc = subprocess.Popen([str(exe)], cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(4)
        started = proc.poll() is None
        if started:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
        return {"passed": started, "path": str(exe), "started": started}
    except Exception as exc:
        if proc and proc.poll() is None:
            proc.kill()
        return {"passed": False, "path": str(exe), "error": str(exc)}


def read_json(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    release_exe = ROOT / "target" / "release" / "quickpls-desktop.exe"
    nsis = ROOT / "target" / "release" / "bundle" / "nsis" / "QuickPLS_1.0.0_x64-setup.exe"
    dist_asset = newest_dist_asset()
    exe_status = file_status(release_exe)
    dist_status = file_status(dist_asset) if dist_asset else {"exists": False, "path": None}
    ux_smoke = read_json(RESULTS / "v11_desktop_ux_smoke.json")
    launch = launch_release_exe(release_exe)
    checklist = {
        "release_executable_exists": exe_status["exists"],
        "installer_artifact_exists": nsis.exists() and nsis.stat().st_size > 0,
        "frontend_dist_exists": dist_status["exists"],
        "release_exe_newer_than_frontend_dist": exe_status.get("mtime", 0) >= dist_status.get("mtime", 0),
        "launches_without_dev_server": launch["passed"],
        "v11_desktop_width_ux_smoke_passed": ux_smoke.get("passed") is True,
        "completed_run_overlay_smoke_passed": ux_smoke.get("metrics", {}).get("completedModelMetrics", {}).get("overlayHasLoadings") is True
            and ux_smoke.get("metrics", {}).get("completedReportMetrics", {}).get("svgHasPaths") is True,
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.1 native desktop smoke",
        "passed": all(checklist.values()),
        "checklist": checklist,
        "release_executable": exe_status,
        "installer": file_status(nsis),
        "newest_frontend_dist_asset": dist_status,
        "launch_probe": launch,
        "linked_evidence": {
            "ux_smoke": "validation/results/v11_desktop_ux_smoke.json",
            "screenshots": "validation/results/screens/v11/",
        },
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
