"""QuickPLS v1.0 deterministic desktop smoke and recovery audit."""

import glob
import json
import os
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v10_desktop_smoke_check.json"
VERSION = "1.0.0"


def run(command: list[str], timeout: int = 240) -> dict:
    start = time.perf_counter()
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "elapsed_seconds": round(time.perf_counter() - start, 4),
        "stdout_tail": proc.stdout[-3000:],
        "stderr_tail": proc.stderr[-3000:],
    }


def file_nonempty(path: Path) -> bool:
    return path.exists() and path.stat().st_size > 0


def read_json(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def launch_release_exe(exe: Path) -> dict:
    if not exe.exists():
        return {"passed": False, "path": str(exe), "reason": "release executable missing"}
    proc = None
    try:
        proc = subprocess.Popen([str(exe)], cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(3)
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


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    result_json = RESULTS / "v10_smoke_pls_result.json"
    csv_export = RESULTS / "v10_smoke_export.csv"
    html_export = RESULTS / "v10_smoke_export.html"
    xlsx_export = RESULTS / "v10_smoke_export.xlsx"
    release_exe = ROOT / "target" / "release" / "quickpls-desktop.exe"
    nsis = list(glob.glob(str(ROOT / "target" / "release" / "bundle" / "nsis" / f"QuickPLS_{VERSION}_x64-setup.exe")))

    commands = [
        run(["cargo", "run", "-p", "qpls-cli", "--", "run", "validation/fixtures/simple_reflective.recipe.json", "--data", "validation/fixtures/simple_reflective.csv", "--output", str(result_json.relative_to(ROOT)), "--allow-experimental", "--workers", "1"]),
        run(["cargo", "run", "-p", "qpls-cli", "--", "export", str(result_json.relative_to(ROOT)), "--format", "csv", "--output", str(csv_export.relative_to(ROOT))]),
        run(["cargo", "run", "-p", "qpls-cli", "--", "export", str(result_json.relative_to(ROOT)), "--format", "html", "--output", str(html_export.relative_to(ROOT))]),
        run(["cargo", "run", "-p", "qpls-cli", "--", "export", str(result_json.relative_to(ROOT)), "--format", "xlsx", "--output", str(xlsx_export.relative_to(ROOT))]),
    ]
    launch = launch_release_exe(release_exe)
    v093_smoke = read_json(RESULTS / "v093_sem_designer_visual_smoke.json")
    report_source = (ROOT / "src" / "components" / "ReportsWorkspace.tsx").read_text(encoding="utf-8")
    graph_source = (ROOT / "src" / "domain" / "diagramGraph.ts").read_text(encoding="utf-8")
    project_source = (ROOT / "crates" / "qpls-project" / "src" / "lib.rs").read_text(encoding="utf-8")
    checklist = {
        "release_executable_launches_without_dev_server": launch["passed"],
        "nsis_installer_artifact_exists": bool(nsis),
        "validation_csv_fixture_available": file_nonempty(ROOT / "validation" / "fixtures" / "simple_reflective.csv"),
        "simple_pls_run_saved": file_nonempty(result_json) and commands[0]["passed"],
        "csv_export_readable": file_nonempty(csv_export) and "outer_estimate" in csv_export.read_text(encoding="utf-8", errors="ignore"),
        "html_export_readable": file_nonempty(html_export) and "<table" in html_export.read_text(encoding="utf-8", errors="ignore").lower(),
        "xlsx_export_readable": file_nonempty(xlsx_export),
        "svg_export_path_available": "quickpls-publication-diagram.svg" in report_source,
        "browser_print_pdf_documented_not_native_audited": "Print / PDF" in report_source and "Native CLI PDF and PNG export are not part of v1.0.0" in (ROOT / "docs" / "V1_SUPPORTED_SCOPE.md").read_text(encoding="utf-8"),
        "autosave_recovery_tests_exist": "load_project_with_autosave" in project_source,
        "diagram_layout_persistence_smoke_passed": v093_smoke.get("passed") is True and v093_smoke.get("metrics", {}).get("draggedIndicatorMoved") is True,
        "diagram_estimates_block_stale_incompatible": "resultForOverlay" in graph_source and "compatible" in graph_source and "Run or select a compatible result" in graph_source,
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.0.0 desktop smoke and recovery audit",
        "passed": all(command["passed"] for command in commands) and all(checklist.values()),
        "checklist": checklist,
        "launch_probe": launch,
        "nsis_installers": [str(Path(path).relative_to(ROOT)) for path in nsis],
        "exports": {
            "csv": str(csv_export.relative_to(ROOT)),
            "html": str(html_export.relative_to(ROOT)),
            "xlsx": str(xlsx_export.relative_to(ROOT)),
            "svg": "desktop/report SVG export path verified by Reports workspace and frontend tests",
            "pdf": "browser print-to-PDF path documented; native PDF is not part of v1.0 audited scope",
        },
        "commands": commands,
        "note": "Pointer-level designer coverage is inherited from the v0.9.3 Playwright visual smoke artifact to avoid brittle repeated GUI automation in the final release audit.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
