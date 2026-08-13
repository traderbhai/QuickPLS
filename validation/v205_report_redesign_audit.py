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
    report = read_text("src/components/ReportsWorkspace.tsx")
    styles = read_text("src/styles.css")
    docs = read_text("docs/V2_0_5_REPORT_EXPORT_FLOW_REDESIGN.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v205_report_redesign_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    slice_id = "v2_0_5_report_export_flow_redesign"
    required_slice = next((s for s in registry.get("slices", []) if s.get("id") == slice_id), None)
    boundary_text = "\n".join([docs, delivery, ledger])
    quickpls_packages = {
        "quickpls-desktop",
        "qpls-assessment",
        "qpls-cli",
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-project",
        "qpls-resampling",
        "qpls-runner",
    }
    package_versions: dict[str, str] = {}
    current_name: str | None = None
    for line in cargo_lock.splitlines():
        stripped = line.strip()
        if stripped == "[[package]]":
            current_name = None
        elif stripped.startswith("name = "):
            current_name = stripped.split('"', 2)[1]
        elif current_name in quickpls_packages and stripped.startswith("version = "):
            package_versions[current_name] = stripped.split('"', 2)[1]
    quickpls_lock_versions_current = (
        set(package_versions) == quickpls_packages
        and all(version == "2.0.5" for version in package_versions.values())
    )

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.0.5": registry.get("current_stage") == slice_id,
        "registry contains v2.0.5 slice": required_slice is not None,
        "v2.0.5 gates are passed": bool(required_slice) and all(g.get("status") == "passed" for g in required_slice.get("gates", [])),
        "package version is 2.0.5": package.get("version") == "2.0.5",
        "package lock root version is 2.0.5": '"version": "2.0.5"' in package_lock,
        "cargo workspace version is 2.0.5": 'version = "2.0.5"' in cargo,
        "cargo lock QuickPLS package versions are updated": quickpls_lock_versions_current,
        "tauri version is 2.0.5": tauri.get("version") == "2.0.5",
        "roadmap expects v2.0.5": slice_id in roadmap,
        "release artifact label is v2.0.5": slice_id in package["scripts"].get("qpls:release:artifacts", ""),
        "report uses v2 workspace": "qpls2-workspace" in report and "report-v2-workspace" in report,
        "report hero and command center exist": "report-v2-hero" in report and "report-v2-command-center" in report,
        "report preserves export outputs": all(token in report for token in ["CSV tables", "HTML report", "XLSX workbook", "Print / PDF", "Model diagram SVG"]),
        "styles cover v2 report classes": all(token in styles for token in [".report-v2-hero", ".report-v2-command-center", ".report-v2-preview-shell", ".report-v2-export-actions"]),
        "docs record frontend-only boundary": "No statistical engines" in boundary_text or "no estimator" in boundary_text.lower(),
        "delivery and ledger mention v2.0.5": "v2.0.5 Report Export Flow Redesign" in delivery and "v2.0.5 Report Export Flow Redesign" in ledger,
        "no mojibake in v2.0.5 sources": "RÃ" not in "\n".join([report, styles, docs]),
    }
    failed = [name for name, passed in checks.items() if not passed]
    output = {
        "milestone": slice_id,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v205_report_redesign_audit.json").write_text(json.dumps(output, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(output, indent=2))
        return 1
    print("v2.0.5 Report redesign audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
