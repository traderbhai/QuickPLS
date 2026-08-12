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
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    model = read_text("src/components/ModelCanvas.tsx")
    trust = read_text("src/components/TrustCenterWorkspace.tsx")
    settings = read_text("src/components/SettingsWorkspace.tsx")
    topbar = read_text("src/components/TopBar.tsx")
    docs = read_text("docs/V2_1_4_MODEL_TRUST_SETTINGS_SHELL_ALIGNMENT.md")
    smoke_path = RESULTS / "v2114_model_trust_settings_shell_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_1_4_model_trust_settings_shell_alignment"
    forbidden_mojibake = ["RÃƒÆ’Ã†â€™", "RÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â²", "RÃƒâ€šÃ‚Â²", "RÂ²", "ÃƒÆ’Ã¢â‚¬Å¡"]
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]
    user_sources = "\n".join([model, trust, settings, topbar])

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.1.4": registry.get("current_stage") == target_stage,
        "registry contains v2.1.4 slice": any(s.get("id") == target_stage for s in registry.get("slices", [])),
        "package version is 2.1.4": package.get("version") == "2.1.4",
        "package lock root version is 2.1.4": package_lock.get("version") == "2.1.4" and package_lock.get("packages", {}).get("", {}).get("version") == "2.1.4",
        "cargo workspace version is 2.1.4": 'version = "2.1.4"' in cargo,
        "quickpls lock versions are 2.1.4": all(f'name = "{name}"\nversion = "2.1.4"' in cargo_lock for name in [
            "qpls-core",
            "qpls-data",
            "qpls-project",
            "qpls-runner",
            "quickpls-desktop",
        ]),
        "tauri version is 2.1.4": tauri.get("version") == "2.1.4",
        "roadmap expects v2.1.4": target_stage in roadmap,
        "topbar shows v2.1.4": "v2.1.4 model/trust/settings shell alignment" in topbar,
        "artifact label is v2.1.4": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.1.4 scripts": all(key in package["scripts"] for key in [
            "qpls:v2114:shell-smoke",
            "qpls:v2114:shell-audit",
            "qpls:v2114:shell",
        ]),
        "v2.1.3 script remains pointed to v2.1.3 gate": "v2_1_3_results_report_mockup_alignment" in package["scripts"].get("qpls:v2113:results-report", ""),
        "model shell markers present": all(token in model for token in [
            "model-v214-workspace",
            "model-v214-toolbar",
            "model-v214-overlay-status",
            "ReactFlow",
        ]),
        "trust uses v2 primitives": all(token in trust for token in [
            "WorkspacePage",
            "PageHeader",
            "Panel",
            "MetricCard",
            "ResearchTable",
            "StatusBadge",
            "trust-v214-workspace",
        ]),
        "settings uses v2 shell": all(token in settings for token in [
            "WorkspacePage",
            "PageHeader",
            "Panel",
            "MetricCard",
            "settings-v214-workspace",
        ]),
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no sem designer rewrite": "No SEM designer behavior rewrite" in docs,
        "normal target source has no SmartPLS equivalence claim": not any(phrase in user_sources.lower() for phrase in forbidden_equivalence),
        "normal target source has no mojibake": not any(token in user_sources for token in forbidden_mojibake),
        "smoke and audit artifacts are referenced": all(token in docs for token in [
            "validation/v2114_model_trust_settings_shell_smoke.mjs",
            "validation/v2114_model_trust_settings_shell_audit.py",
        ]),
    }

    failed = [name for name, passed in checks.items() if not passed]
    report_payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v2114_model_trust_settings_shell_audit.json").write_text(json.dumps(report_payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report_payload, indent=2))
        return 1
    print("v2.1.4 Model/Trust/Settings shell audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
