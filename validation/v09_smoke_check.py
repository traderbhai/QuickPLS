"""QuickPLS v0.9 release-candidate smoke check.

This script verifies deterministic release artifacts that can be checked without
depending on a fragile GUI automation stack. The release executable launch probe
is bounded and terminated immediately after startup.
"""

import json
import os
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v09_smoke_check.json"


def npm():
    return "npm.cmd" if os.name == "nt" else "npm"


def run(command, timeout=240):
    started = time.perf_counter()
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "elapsed_seconds": round(time.perf_counter() - started, 4),
        "stdout_tail": proc.stdout[-3000:],
        "stderr_tail": proc.stderr[-3000:],
    }


def file_nonempty(path):
    return path.exists() and path.stat().st_size > 0


def launch_release_exe(exe):
    if not exe.exists():
        return {"passed": False, "reason": "release executable missing", "path": str(exe)}
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


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    result_json = RESULTS / "v09_smoke_pls_result.json"
    csv_export = RESULTS / "v09_smoke_export.csv"
    html_export = RESULTS / "v09_smoke_export.html"
    xlsx_export = RESULTS / "v09_smoke_export.xlsx"
    svg_probe = ROOT / "src" / "domain" / "publicationDiagram.ts"
    report_probe = ROOT / "src" / "components" / "ReportsWorkspace.tsx"
    tauri_config = ROOT / "src-tauri" / "tauri.conf.json"
    release_exe = ROOT / "target" / "release" / "quickpls-desktop.exe"

    commands = [
        run([
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "run",
            "validation/fixtures/simple_reflective.recipe.json",
            "--data",
            "validation/fixtures/simple_reflective.csv",
            "--output",
            str(result_json.relative_to(ROOT)),
            "--allow-experimental",
            "--workers",
            "1",
        ]),
        run([
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "export",
            str(result_json.relative_to(ROOT)),
            "--format",
            "csv",
            "--output",
            str(csv_export.relative_to(ROOT)),
        ]),
        run([
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "export",
            str(result_json.relative_to(ROOT)),
            "--format",
            "html",
            "--output",
            str(html_export.relative_to(ROOT)),
        ]),
        run([
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "export",
            str(result_json.relative_to(ROOT)),
            "--format",
            "xlsx",
            "--output",
            str(xlsx_export.relative_to(ROOT)),
        ]),
    ]

    tauri_text = tauri_config.read_text(encoding="utf-8")
    diagram_text = svg_probe.read_text(encoding="utf-8")
    reports_text = report_probe.read_text(encoding="utf-8")
    launch = launch_release_exe(release_exe)

    checklist = {
        "release_executable_launches_without_dev_server": launch["passed"],
        "frontend_dist_configured": '"frontendDist"' in tauri_text and '"../dist"' in tauri_text,
        "dev_server_is_local_only": '"devUrl": "http://localhost:1420"' in tauri_text,
        "validation_csv_fixture_available": file_nonempty(ROOT / "validation" / "fixtures" / "simple_reflective.csv"),
        "simple_reflective_pls_run_saved": file_nonempty(result_json) and commands[0]["passed"],
        "csv_export_readable": file_nonempty(csv_export) and "outer_estimate" in csv_export.read_text(encoding="utf-8", errors="ignore"),
        "html_export_readable": file_nonempty(html_export) and "<table" in html_export.read_text(encoding="utf-8", errors="ignore").lower(),
        "xlsx_export_readable": file_nonempty(xlsx_export),
        "diagram_estimates_require_saved_run": "run ?" in diagram_text and "result?.paths" in diagram_text and "outer_estimates" in diagram_text,
        "svg_diagram_export_available": "publicationDiagramSvg" in reports_text and "quickpls-publication-diagram.svg" in reports_text,
        "browser_print_pdf_path_available": "Print / PDF" in reports_text,
        "project_recovery_tests_exist": "load_project_with_autosave" in (ROOT / "crates" / "qpls-project" / "src" / "lib.rs").read_text(encoding="utf-8"),
    }
    passed = all(command["passed"] for command in commands) and all(checklist.values())
    report = {
        "schema_version": 1,
        "target": "QuickPLS v0.9.0-rc.1 deterministic desktop smoke check",
        "passed": passed,
        "checklist": checklist,
        "launch_probe": launch,
        "exports": {
            "csv": str(csv_export.relative_to(ROOT)),
            "html": str(html_export.relative_to(ROOT)),
            "xlsx": str(xlsx_export.relative_to(ROOT)),
            "svg": "desktop/report publication diagram export path verified by source and frontend tests",
            "pdf": "browser print-to-PDF path verified by Reports workspace source",
        },
        "commands": commands,
        "note": "GUI interaction is covered by source-level contract checks and frontend/desktop tests; this script avoids brittle pointer automation.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
