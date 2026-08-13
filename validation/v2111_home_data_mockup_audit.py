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
    home = read_text("src/components/OnboardingWorkspace.tsx")
    data = read_text("src/components/DataWorkspace.tsx")
    ui = read_text("src/components/Ui.tsx")
    topbar = read_text("src/components/TopBar.tsx")
    docs = read_text("docs/V2_1_1_HOME_DATA_MOCKUP_ALIGNMENT.md")
    smoke_path = RESULTS / "v2111_home_data_mockup_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    required_home_primitives = [
        "WorkspacePage",
        "PageHeader",
        "Panel",
        "Card",
        "MetricCard",
        "InlineNotice",
    ]
    required_data_primitives = [
        "WorkspacePage",
        "PageHeader",
        "Panel",
        "MetricCard",
        "InlineNotice",
    ]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.1.1": registry.get("current_stage") == "v2_1_1_home_data_mockup_alignment",
        "registry contains v2.1.1 slice": any(s.get("id") == "v2_1_1_home_data_mockup_alignment" for s in registry.get("slices", [])),
        "package version is 2.1.1": package.get("version") == "2.1.1",
        "package lock version is 2.1.1": package_lock.get("version") == "2.1.1",
        "cargo version is 2.1.1": 'version = "2.1.1"' in cargo,
        "tauri version is 2.1.1": tauri.get("version") == "2.1.1",
        "roadmap expects v2.1.1": "v2_1_1_home_data_mockup_alignment" in roadmap,
        "topbar shows v2.1.1": "v2.1.1 home/data mockup alignment" in topbar,
        "artifact label is v2.1.1": "v2_1_1_home_data_mockup_alignment" in package["scripts"].get("qpls:release:artifacts", ""),
        "home uses all required primitives": all(token in home for token in required_home_primitives),
        "data uses all required primitives": all(token in data for token in required_data_primitives),
        "home keeps project commands wired": all(token in home for token in ["quickpls:save-project", "quickpls:open-project", "quickpls:open-demo-project"]),
        "data keeps native import wiring": all(token in data for token in ["importNativeDataset", "importNativeValidationFixture", "updateNativeColumnMetadata"]),
        "data keeps method guidance and prefix bridge": "dataGuidance" in data and "Create Constructs From Prefixes" in data,
        "ui exposes v2.1 primitives": all(token in ui for token in ["WorkspacePage", "Panel", "MetricCard", "InlineNotice"]),
        "docs declare frontend product only": "frontend/product-only" in docs,
        "normal Home/Data source has no SmartPLS equivalence claim": not any(phrase in (home + data).lower() for phrase in ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]),
        "normal Home/Data source has no mojibake": not any(token in home + data for token in ["RÃ", "Â²", "Ãƒ", "â€"]),
    }

    failed = [name for name, passed in checks.items() if not passed]
    report = {
        "milestone": "v2_1_1_home_data_mockup_alignment",
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v2111_home_data_mockup_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report, indent=2))
        return 1
    print("v2.1.1 Home/Data mockup audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
