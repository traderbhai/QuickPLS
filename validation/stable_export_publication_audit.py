"""Publication audit for stable export surfaces."""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "stable_export_publication_audit.json"


def run(command, timeout=240):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {"command": command, "returncode": proc.returncode, "passed": proc.returncode == 0, "stdout_tail": proc.stdout[-3000:], "stderr_tail": proc.stderr[-3000:]}


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    commands = [
        run(["cargo", "run", "-p", "qpls-cli", "--", "export", "validation/results/pls_quickpls_path_mode_a.json", "--format", "csv", "--output", "validation/results/stable_export_publication.csv"]),
        run(["cargo", "run", "-p", "qpls-cli", "--", "export", "validation/results/pls_quickpls_path_mode_a.json", "--format", "html", "--output", "validation/results/stable_export_publication.html"]),
        run(["cargo", "run", "-p", "qpls-cli", "--", "export", "validation/results/pls_quickpls_path_mode_a.json", "--format", "xlsx", "--output", "validation/results/stable_export_publication.xlsx"]),
        run(["cargo", "test", "-p", "qpls-cli"]),
        run(["cargo", "test", "-p", "quickpls-desktop"]),
        run(["npm.cmd" if __import__("os").name == "nt" else "npm", "test", "--", "--run"]),
    ]
    files = {
        "csv": (RESULTS / "stable_export_publication.csv").exists(),
        "html": (RESULTS / "stable_export_publication.html").exists(),
        "xlsx": (RESULTS / "stable_export_publication.xlsx").exists(),
    }
    report_source = (ROOT / "src" / "components" / "ReportsWorkspace.tsx").read_text(encoding="utf-8")
    gui_exports = {
        "csv_report_tables": "quickpls-result-tables.csv" in report_source,
        "html_report": "quickpls-result-report.html" in report_source,
        "xlsx_workbook": "XLSX workbook" in report_source,
        "browser_pdf_path": "Print / PDF" in report_source and "printable.print()" in report_source,
        "svg_diagram": "quickpls-publication-diagram.svg" in report_source,
    }
    cli_export_source = (ROOT / "crates" / "qpls-cli" / "src" / "main.rs").read_text(encoding="utf-8")
    cli_formats = {
        "csv": "ExportFormat::Csv" in cli_export_source,
        "html": "ExportFormat::Html" in cli_export_source,
        "xlsx": "ExportFormat::Xlsx" in cli_export_source,
        "stable_rejects_experimental_by_default": "export_includes_watermarked_experimental_method_tables_when_requested" in cli_export_source,
    }
    passed = all(command["passed"] for command in commands) and all(files.values()) and all(gui_exports.values()) and all(cli_formats.values())
    report = {
        "schema_version": 1,
        "target": "stable export publication audit",
        "passed": passed,
        "generated_files": files,
        "gui_exports": gui_exports,
        "cli_formats": cli_formats,
        "commands": commands,
        "note": "Stable CLI exports cover CSV/HTML/XLSX. Desktop/report workflow covers CSV/HTML/XLSX, SVG diagrams, and browser print-to-PDF.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
