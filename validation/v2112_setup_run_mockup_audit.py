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
    setup = read_text("src/components/AnalysisCatalog.tsx")
    run = read_text("src/components/RunWorkspace.tsx")
    ui = read_text("src/components/Ui.tsx")
    topbar = read_text("src/components/TopBar.tsx")
    docs = read_text("docs/V2_1_2_SETUP_RUN_MOCKUP_ALIGNMENT.md")
    smoke_path = RESULTS / "v2112_setup_run_mockup_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    required_setup_primitives = [
        "WorkspacePage",
        "PageHeader",
        "Panel",
        "Card",
        "StatusBadge",
        "TabStrip",
    ]
    required_run_primitives = [
        "WorkspacePage",
        "PageHeader",
        "Panel",
        "MetricCard",
        "StatusBadge",
    ]
    combined_user_sources = setup + "\n" + run + "\n" + topbar
    forbidden_mojibake = ["RÃƒ", "RÃ‚Â²", "RÂ²", "ÃƒÆ’", "Ã‚", "Ã¢â‚¬"]
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.1.2": registry.get("current_stage") == "v2_1_2_setup_run_mockup_alignment",
        "registry contains v2.1.2 slice": any(s.get("id") == "v2_1_2_setup_run_mockup_alignment" for s in registry.get("slices", [])),
        "package version is 2.1.2": package.get("version") == "2.1.2",
        "package lock version is 2.1.2": package_lock.get("version") == "2.1.2",
        "cargo version is 2.1.2": 'version = "2.1.2"' in cargo,
        "tauri version is 2.1.2": tauri.get("version") == "2.1.2",
        "roadmap expects v2.1.2": "v2_1_2_setup_run_mockup_alignment" in roadmap,
        "topbar shows v2.1.2": "v2.1.2 setup/run mockup alignment" in topbar,
        "artifact label is v2.1.2": "v2_1_2_setup_run_mockup_alignment" in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.1.2 scripts": all(key in package["scripts"] for key in [
            "qpls:v2112:setup-run-smoke",
            "qpls:v2112:setup-run-audit",
            "qpls:v2112:setup-run",
        ]),
        "setup uses all required primitives": all(token in setup for token in required_setup_primitives),
        "run uses all required primitives": all(token in run for token in required_run_primitives),
        "setup keeps method applicability guidance": all(token in setup for token in [
            "evaluateMethodApplicability",
            "MethodSection",
            "setup-v2-requirements",
            "selectedExpectedOutputs",
        ]),
        "setup keeps run command event wired": "quickpls:run-analysis" in setup,
        "run keeps run command event wired": "quickpls:run-analysis" in run,
        "run keeps results and report handoff": all(token in run for token in ["Open results", "Prepare report"]),
        "run keeps output preview": "outputPreview" in run and "run-v2-output-preview" in run,
        "ui exposes v2.1 primitives": all(token in ui for token in ["WorkspacePage", "Panel", "MetricCard", "StatusBadge"]),
        "docs declare frontend product only": "frontend/product-only" in docs,
        "normal Setup/Run source has no SmartPLS equivalence claim": not any(phrase in combined_user_sources.lower() for phrase in forbidden_equivalence),
        "normal Setup/Run source has no mojibake": not any(token in combined_user_sources for token in forbidden_mojibake),
    }

    failed = [name for name, passed in checks.items() if not passed]
    report = {
        "milestone": "v2_1_2_setup_run_mockup_alignment",
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v2112_setup_run_mockup_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report, indent=2))
        return 1
    print("v2.1.2 Setup/Run mockup audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
