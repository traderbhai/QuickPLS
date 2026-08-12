import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
RESULTS.mkdir(parents=True, exist_ok=True)


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def main() -> int:
    package = json.loads(read_text("package.json"))
    registry = json.loads(read_text("validation/development_slices.json"))
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    package_lock = read_text("package-lock.json")
    tauri = json.loads(read_text("src-tauri/tauri.conf.json"))
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    run_history = read_text("src/components/RunHistory.tsx")
    styles = read_text("src/styles.css")
    docs = read_text("docs/V2_0_4_RESULTS_TABLE_INTERPRETATION_REDESIGN.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v204_results_redesign_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    slice_id = "v2_0_4_results_table_interpretation_redesign"
    required_slice = next((s for s in registry.get("slices", []) if s.get("id") == slice_id), None)
    boundary_text = "\n".join([docs, delivery, ledger])
    quickpls_lock_versions = [
        line for line in cargo_lock.splitlines()
        if line.strip() == 'version = "2.0.3"'
    ]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.0.4": registry.get("current_stage") == slice_id,
        "registry contains v2.0.4 slice": required_slice is not None,
        "v2.0.4 gates are passed": bool(required_slice) and all(g.get("status") == "passed" for g in required_slice.get("gates", [])),
        "package version is 2.0.4": package.get("version") == "2.0.4",
        "package lock root version is 2.0.4": '"version": "2.0.4"' in package_lock,
        "cargo workspace version is 2.0.4": 'version = "2.0.4"' in cargo,
        "cargo lock QuickPLS package versions are updated": len(quickpls_lock_versions) == 0,
        "tauri version is 2.0.4": tauri.get("version") == "2.0.4",
        "roadmap expects v2.0.4": slice_id in roadmap,
        "release artifact label is v2.0.4": slice_id in package["scripts"].get("qpls:release:artifacts", ""),
        "results lens exists": "ResultsV2LensPanel" in run_history and "resultsTabSummary" in run_history,
        "selected run context exists": "Selected run" in run_history and "Active results view" in run_history,
        "research table metadata exists": "results-v2-table-meta" in run_history and "Wide table: first column stays pinned while scrolling" in run_history,
        "styles cover v2 results lens/table": all(token in styles for token in [".results-v2-lens-panel", ".results-v2-table-header", ".results-v2-nav-header"]),
        "docs record frontend-only boundary": "No statistical engines" in boundary_text or "no estimator" in boundary_text.lower(),
        "delivery and ledger mention v2.0.4": "v2.0.4 Results Table And Interpretation Redesign" in delivery and "v2.0.4 Results Table And Interpretation Redesign" in ledger,
        "no mojibake in v2.0.4 sources": "RÃ" not in "\n".join([run_history, styles, docs]),
    }
    failed = [name for name, passed in checks.items() if not passed]
    report = {
        "milestone": slice_id,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v204_results_redesign_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report, indent=2))
        return 1
    print("v2.0.4 Results redesign audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
