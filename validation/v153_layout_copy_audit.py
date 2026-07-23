import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "v153_layout_copy_audit.json"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def json_file(path: str):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def normal_ui_sources() -> list[Path]:
    roots = [ROOT / "src" / "components", ROOT / "src" / "domain", ROOT / "src"]
    files: set[Path] = set()
    for root in roots:
        if root.is_file():
            files.add(root)
        elif root.exists():
            for path in root.rglob("*"):
                if path.suffix in {".ts", ".tsx", ".css"} and ".test." not in path.name:
                    files.add(path)
    return sorted(files)


def main() -> int:
    package = json_file("package.json")
    registry = json_file("validation/development_slices.json")
    app = read("src/App.tsx")
    topbar = read("src/components/TopBar.tsx")
    ui = read("src/components/Ui.tsx")
    data = read("src/components/DataWorkspace.tsx")
    analysis = read("src/components/AnalysisCatalog.tsx")
    run = read("src/components/RunWorkspace.tsx")
    results = read("src/components/RunHistory.tsx")
    reports = read("src/components/ReportsWorkspace.tsx")
    sem_edge = read("src/components/SemEdge.tsx")
    helpers = read("src/domain/dataWorkspace.ts")
    styles = read("src/styles.css")
    roadmap = read("crates/qpls-core/src/roadmap.rs")
    docs = read("docs/V1_5_3_LAYOUT_COPY_READINESS_POLISH.md") if (ROOT / "docs" / "V1_5_3_LAYOUT_COPY_READINESS_POLISH.md").exists() else ""
    smoke_path = RESULTS / "v153_layout_copy_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}

    combined_ui = "\n".join(path.read_text(encoding="utf-8", errors="replace") for path in normal_ui_sources())
    forbidden_copy = [
        "Start new projectStart",
        "Import datasetCSV",
        "Missing dataset9",
        "Experimental scopeValidated",
        "Diagram exportSVG",
        "Table exportsRun",
        "Validation fixture",
        "RÂ²",
        "RÃ",
    ]

    screenshots = smoke.get("screenshots", {})
    screenshot_files_exist = all(Path(path).exists() for path in screenshots.values()) and len(screenshots) >= 10

    checks = {
        "scripts_registered": all(key in package["scripts"] for key in ["qpls:v153:layout-copy-smoke", "qpls:v153:layout-copy-audit", "qpls:v153:layout-copy-polish"]),
        "registry_current_stage": registry["current_stage"] == "v1_5_3_layout_copy_readiness_polish",
        "registry_slice_registered": any(item["id"] == "v1_5_3_layout_copy_readiness_polish" and item["status"] == "validated" for item in registry["slices"]),
        "roadmap_current_stage_updated": "v1_5_3_layout_copy_readiness_polish" in roadmap,
        "shared_card_structure": all(text in ui for text in ["ui-card-heading", "ui-card-actions"]) and all(text in styles for text in [".ui-card-heading", ".ui-card-actions"]),
        "compact_blocker_chip": "command-blocker-chip" in topbar and "topBlockerLabel" in topbar and ".command-blocker-chip" in styles,
        "scroll_reset_present": "pageHostRef" in app and "scrollTo({ top: 0, left: 0 })" in app,
        "data_profile_and_copy": "columnProfile" in data and "Selected column profile" in data and "Sample dataset details" in data and "columnProfile" in helpers,
        "generic_path_labels_hidden": "isGenericPathLabel" in sem_edge and "shouldShowLabel" in sem_edge,
        "setup_status_copy": "Scope status" in analysis and "Experimental scope" not in analysis and "Experimental / watermarked" in analysis,
        "setup_progressive_groups": 'setup.mode === "expert"' in analysis and "setup-run-action" in analysis,
        "run_disabled_reason_near_action": "Run disabled:" in run and "disabled={!runs.length}" in run,
        "results_empty_cta_matches_blocker": all(text in results for text in ["emptyPrimary", "emptyPrimaryLabel", "result-preview-tabs"]),
        "report_controls_polished": all(text in reports for text in ["report-preset-strip", "Export disabled:", "Run a method before CSV export", "Model-only SVG preview"]),
        "css_supports_new_controls": all(text in styles for text in [".checkbox-row", ".column-profile", ".setup-run-action", ".result-preview-tabs", ".report-preset-strip"]),
        "normal_ui_forbidden_copy_removed": not any(text in combined_ui for text in forbidden_copy),
        "docs_present": "frontend-only" in docs and "No statistical" in docs,
        "smoke_passed": bool(smoke.get("passed")),
        "screenshots_exist": screenshot_files_exist,
    }
    report = {
        "schema_version": 1,
        "target": "QuickPLS v1.5.3 layout, copy, and readiness polish",
        "passed": all(checks.values()),
        "checklist": checks,
        "evidence": [
            "src/components/Ui.tsx",
            "src/components/TopBar.tsx",
            "src/components/DataWorkspace.tsx",
            "src/components/AnalysisCatalog.tsx",
            "src/components/RunWorkspace.tsx",
            "src/components/RunHistory.tsx",
            "src/components/ReportsWorkspace.tsx",
            "src/components/SemEdge.tsx",
            "src/styles.css",
            "validation/results/v153_layout_copy_smoke.json",
            "validation/results/screens/v153/layout-copy/",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
