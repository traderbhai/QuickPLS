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
    tauri = json.loads(read_text("src-tauri/tauri.conf.json"))
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    home = read_text("src/components/OnboardingWorkspace.tsx")
    data = read_text("src/components/DataWorkspace.tsx")
    styles = read_text("src/styles.css")
    smoke_path = RESULTS / "v201_home_data_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.0.1": registry.get("current_stage") == "v2_0_1_home_data_redesign",
        "registry contains v2.0.1 slice": any(s.get("id") == "v2_0_1_home_data_redesign" for s in registry.get("slices", [])),
        "package version is 2.0.1": package.get("version") == "2.0.1",
        "cargo version is 2.0.1": 'version = "2.0.1"' in cargo,
        "tauri version is 2.0.1": tauri.get("version") == "2.0.1",
        "roadmap expects v2.0.1": "v2_0_1_home_data_redesign" in roadmap,
        "release artifact label is v2.0.1": "v2_0_1_home_data_redesign" in package["scripts"].get("qpls:release:artifacts", ""),
        "Home does not use old v1-only shell as primary class": "workspace-page onboarding-workspace" not in home,
        "Data keeps native import wiring": "importNativeDataset" in data and "importNativeValidationFixture" in data,
        "Data guidance remains applicability-driven": "dataGuidance" in data and "method-guidance-panel" in data,
        "CSS includes desktop fallback breakpoint": "@media (max-width: 1180px)" in styles,
        "no stale v2.0.0 visible shell mark": "v2.0.0 design system shell" not in read_text("src/components/TopBar.tsx"),
    }
    failed = [name for name, passed in checks.items() if not passed]
    report = {
        "milestone": "v2_0_1_home_data_redesign",
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v201_home_data_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report, indent=2))
        return 1
    print("v2.0.1 Home/Data audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
