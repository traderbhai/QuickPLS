"""QuickPLS v1.1.1 disabled-action explanation audit."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v111_disabled_actions_audit.json"


def contains(path: str, needle: str) -> bool:
    return needle in (ROOT / path).read_text(encoding="utf-8")


def main() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    checks = {
        "topbar_run_disabled_reason": contains("src/components/TopBar.tsx", "run-disabled-reason") and contains("src/components/TopBar.tsx", "command-disabled-reason"),
        "run_workspace_disabled_reason": contains("src/components/RunWorkspace.tsx", "disabled-reason"),
        "report_export_disabled_reason": contains("src/components/ReportsWorkspace.tsx", "exportDisabledReason") and contains("src/components/ReportsWorkspace.tsx", "export-disabled-reason"),
        "data_metadata_disabled_reason": contains("src/components/DataWorkspace.tsx", "Select a column in the data preview"),
        "canvas_locked_mode_reason": contains("src/components/ModelCanvas.tsx", "Result and publication views are locked"),
        "canvas_selected_action_reason": contains("src/components/ModelCanvas.tsx", "Select a construct, indicator, or path"),
        "covariance_action_reason": contains("src/components/ModelCanvas.tsx", "Covariance display arcs cannot be reversed"),
        "native_xlsx_reason": contains("src/components/ReportsWorkspace.tsx", "XLSX export requires the desktop runtime"),
        "progressive_settings_guidance": contains("src/components/AnalysisCatalog.tsx", "Recommended defaults"),
        "disabled_reason_css_exists": contains("src/styles.css", "disabled-reason") and contains("src/styles.css", "canvas-disabled-action-reason"),
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.1.1 disabled-action explanation audit",
        "passed": all(checks.values()),
        "checklist": checks,
        "required_surfaces": [
            "TopBar Run/Cancel",
            "Run workspace",
            "Report CSV/HTML/XLSX/SVG/PDF exports",
            "Data metadata Apply",
            "SEM canvas locked result/publication actions",
            "SEM canvas edge/covariance actions",
            "Analysis settings progressive disclosure",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
