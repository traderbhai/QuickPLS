import json

from lib.v2_ui_audit import (
    FORBIDDEN_MOJIBAKE,
    ROOT,
    no_forbidden_tokens,
    no_smartpls_equivalence,
    read_json,
    read_text,
    shared_v2_metadata_checks,
    source_bundle,
    write_result,
)


def contains_any_marker(relative_root: str, markers: list[str]) -> bool:
    root = ROOT / relative_root
    if not root.exists():
        return False
    for path in root.rglob("*"):
        if path.is_file() and path.suffix in {".rs", ".toml", ".json"}:
            text = path.read_text(encoding="utf-8", errors="ignore")
            if any(marker in text for marker in markers):
                return True
    return False


def main() -> int:
    target_stage = "v2_10_0_results_report_research_table_pass"
    version = "2.10.0"
    expected_label = "v2.10.0 results/report tables"

    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    run_history = read_text("src/components/RunHistory.tsx")
    reports = read_text("src/components/ReportsWorkspace.tsx")
    styles = read_text("src/styles.css")
    active = read_text("docs/V2_ACTIVE_MILESTONE.md")
    milestone_doc = read_text("docs/V2_10_0_RESULTS_REPORT_RESEARCH_TABLE_PASS.md")
    notes = read_text("docs/RELEASE_NOTES_V2_10_0.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = ROOT / "validation" / "results" / "v2100_results_report_tables_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False, "runs": []}
    bundle = source_bundle([
        "docs/V2_ACTIVE_MILESTONE.md",
        "docs/V2_10_0_RESULTS_REPORT_RESEARCH_TABLE_PASS.md",
        "docs/RELEASE_NOTES_V2_10_0.md",
        "docs/DELIVERY_STATUS.md",
        "docs/DEVELOPMENT_LEDGER.md",
        "validation/v2100_results_report_tables_smoke.mjs",
        "validation/v2100_results_report_tables_audit.py",
        "src/components/RunHistory.tsx",
        "src/components/ReportsWorkspace.tsx",
        "src/styles.css",
        "src/components/TopBar.tsx",
    ])
    expected_scripts = [
        "qpls:v2100:results-report-tables-smoke",
        "qpls:v2100:results-report-tables-audit",
        "qpls:v2100:results-report-tables",
    ]
    smoke_viewports = sorted(
        f"{run.get('viewport', {}).get('width')}x{run.get('viewport', {}).get('height')}"
        for run in smoke.get("runs", [])
    )
    result_counts = [run.get("results", {}).get("state", {}).get("researchTableCount", 0) for run in smoke.get("runs", [])]
    report_counts = [run.get("report", {}).get("state", {}).get("reportPreviewCount", 0) for run in smoke.get("runs", [])]

    checks = {
        **shared_v2_metadata_checks(
            version=version,
            target_stage=target_stage,
            expected_label=expected_label,
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "package exposes v2.10.0 scripts": all(key in package["scripts"] for key in expected_scripts),
        "run history uses v2.10 research table marker": 'data-results-research-table-pass="v2.10"' in run_history,
        "run history exposes table captions and scan affordance": "v2100-table-affordance" in run_history and "<caption>" in run_history,
        "run history adds per-table export": "Export table" in run_history and "exportCurrentTable" in run_history,
        "reports workspace uses v2.10 export flow marker": 'data-report-export-flow="v2.10"' in reports,
        "reports workspace has table preview helper": "function ReportTablePreview" in reports and 'data-report-table-preview="v2.10"' in reports,
        "reports workspace adds per-preview export": "Export table" in reports and "exportReportTable" in reports,
        "v2.10 styles exist": all(token in styles for token in [
            ".v2100-research-table",
            ".v2100-table-affordance",
            ".v2100-report-table-preview",
            ".report-v2100-table-scroll",
        ]),
        "smoke output exists and passed": bool(smoke.get("passed")),
        "smoke covers desktop viewports": smoke_viewports == ["1280x800", "1440x900"],
        "smoke covers results research tables": result_counts and min(result_counts) >= 3,
        "smoke covers report table previews": report_counts and min(report_counts) >= 1,
        "active tracker names v2.10 current milestone": target_stage in active and "Results and Report" in active,
        "milestone doc records frontend boundary": "frontend/product-only" in milestone_doc and "No estimator" in milestone_doc,
        "release notes describe v2.10 table pass": "QuickPLS 2.10.0" in notes and "Results/Report research table" in notes,
        "delivery status includes v2.10": "v2.10.0 Results/Report Research Table Pass" in delivery,
        "development ledger includes v2.10": "v2.10.0 Results/Report Research Table Pass" in ledger,
        "normal v2.10.0 sources have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "no estimator/result-schema marker changes": not any(
            contains_any_marker(path, [target_stage, "v2100-results", "v2.10"])
            for path in [
                "crates/qpls-estimation/src",
                "crates/qpls-assessment/src",
                "crates/qpls-resampling/src",
                "crates/qpls-runner/src",
                "crates/qpls-project/src",
            ]
        ),
    }

    failed = [name for name, passed in checks.items() if not passed]
    payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    write_result("v2100_results_report_tables_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.10.0 Results/Report research table audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
