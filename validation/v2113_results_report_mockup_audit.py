import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
RESULTS.mkdir(parents=True, exist_ok=True)


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def read_json(relative: str):
    return json.loads(read_text(relative))


def main() -> int:
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    results = read_text("src/components/RunHistory.tsx")
    report = read_text("src/components/ReportsWorkspace.tsx")
    ui = read_text("src/components/Ui.tsx")
    topbar = read_text("src/components/TopBar.tsx")
    docs = read_text("docs/V2_1_3_RESULTS_REPORT_MOCKUP_ALIGNMENT.md")
    smoke_path = RESULTS / "v2113_results_report_mockup_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    required_results_primitives = [
        "WorkspacePage",
        "PageHeader",
        "Panel",
        "MetricCard",
        "StatusBadge",
        "MethodConfidencePanel",
        "ReportabilityChecklist",
    ]
    required_report_primitives = [
        "WorkspacePage",
        "PageHeader",
        "Panel",
        "MetricCard",
        "Card",
        "StatusBadge",
        "MethodScopeDrawer",
        "MethodConfidencePanel",
    ]
    combined_user_sources = results + "\n" + report + "\n" + topbar
    forbidden_mojibake = ["RÃƒÆ’", "RÃƒâ€šÃ‚Â²", "RÃ‚Â²", "ÃƒÆ’Ã†â€™", "Ãƒâ€š", "ÃƒÂ¢Ã¢â€šÂ¬"]
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.1.3": registry.get("current_stage") == "v2_1_3_results_report_mockup_alignment",
        "registry contains v2.1.3 slice": any(s.get("id") == "v2_1_3_results_report_mockup_alignment" for s in registry.get("slices", [])),
        "package version is 2.1.3": package.get("version") == "2.1.3",
        "package lock version is 2.1.3": package_lock.get("version") == "2.1.3",
        "cargo version is 2.1.3": 'version = "2.1.3"' in cargo,
        "tauri version is 2.1.3": tauri.get("version") == "2.1.3",
        "roadmap expects v2.1.3": "v2_1_3_results_report_mockup_alignment" in roadmap,
        "topbar shows v2.1.3": "v2.1.3 results/report mockup alignment" in topbar,
        "artifact label is v2.1.3": "v2_1_3_results_report_mockup_alignment" in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.1.3 scripts": all(key in package["scripts"] for key in [
            "qpls:v2113:results-report-smoke",
            "qpls:v2113:results-report-audit",
            "qpls:v2113:results-report",
        ]),
        "results uses all required primitives": all(token in results for token in required_results_primitives),
        "report uses all required primitives": all(token in report for token in required_report_primitives),
        "results keeps table and interpretation controls": all(token in results for token in [
            "ResultMenu",
            "exportCurrentTable",
            "copyableInterpretationText",
            "RunResultSections",
        ]),
        "report keeps export wiring": all(token in report for token in [
            "download(",
            "exportNativeXlsxTables",
            "publicationDiagramSvg",
            "tablesToCsv",
            "tablesToHtml",
            "printPdfReport",
        ]),
        "ui exposes v2.1 primitives": all(token in ui for token in ["WorkspacePage", "Panel", "MetricCard", "StatusBadge"]),
        "docs declare frontend product only": "frontend/product-only" in docs,
        "normal Results/Report source has no SmartPLS equivalence claim": not any(phrase in combined_user_sources.lower() for phrase in forbidden_equivalence),
        "normal Results/Report source has no mojibake": not any(token in combined_user_sources for token in forbidden_mojibake),
    }

    failed = [name for name, passed in checks.items() if not passed]
    report_payload = {
        "milestone": "v2_1_3_results_report_mockup_alignment",
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v2113_results_report_mockup_audit.json").write_text(json.dumps(report_payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report_payload, indent=2))
        return 1
    print("v2.1.3 Results/Report mockup audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
