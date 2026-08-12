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
    setup = read_text("src/components/AnalysisCatalog.tsx")
    styles = read_text("src/styles.css")
    topbar = read_text("src/components/TopBar.tsx")
    docs = read_text("docs/V2_0_2_SETUP_METHOD_GUIDANCE_REDESIGN.md")
    smoke_path = RESULTS / "v202_setup_guidance_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.0.2": registry.get("current_stage") == "v2_0_2_setup_method_guidance_redesign",
        "registry contains v2.0.2 slice": any(s.get("id") == "v2_0_2_setup_method_guidance_redesign" for s in registry.get("slices", [])),
        "package version is 2.0.2": package.get("version") == "2.0.2",
        "package lock version is 2.0.2": '"version": "2.0.2"' in read_text("package-lock.json"),
        "cargo version is 2.0.2": 'version = "2.0.2"' in cargo,
        "tauri version is 2.0.2": tauri.get("version") == "2.0.2",
        "roadmap expects v2.0.2": "v2_0_2_setup_method_guidance_redesign" in roadmap,
        "release artifact label is v2.0.2": "v2_0_2_setup_method_guidance_redesign" in package["scripts"].get("qpls:release:artifacts", ""),
        "top bar label is current": "v2.0.2 setup guidance redesign" in topbar,
        "Setup uses selected method sidecar": "setup-v2-sidecar" in setup and "Requirement checks" in setup,
        "Setup avoids duplicated flat settings strip": setup.count("analysis-settings guided-settings") == 1,
        "Setup has exact disabled run reason": "Run disabled:" in setup and "selectedFailure?.detail" in setup,
        "Setup preserves MethodScopeDrawer": "MethodScopeDrawer" in setup and "Why trust this method?" in setup,
        "CSS includes v2 setup sidecar and responsive rules": ".setup-v2-sidecar" in styles and ".setup-v2-main { grid-template-columns: 1fr; }" in styles,
        "docs mention no backend changes": "does not change statistical engines" in docs,
        "no R mojibake in Setup/applicability/topbar": "RÂ²" not in (setup + read_text("src/domain/methodApplicability.ts") + topbar),
    }
    failed = [name for name, passed in checks.items() if not passed]
    report = {
        "milestone": "v2_0_2_setup_method_guidance_redesign",
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v202_setup_guidance_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report, indent=2))
        return 1
    print("v2.0.2 Setup guidance audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
