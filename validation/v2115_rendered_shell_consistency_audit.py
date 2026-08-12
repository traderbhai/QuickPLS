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
    topbar = read_text("src/components/TopBar.tsx")
    app = read_text("src/App.tsx")
    docs = read_text("docs/V2_1_5_RENDERED_SHELL_CONSISTENCY_AUDIT.md")
    smoke_path = RESULTS / "v2115_rendered_shell_consistency_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_1_5_rendered_shell_consistency_audit"
    frontend_sources = "\n".join(
        read_text(path)
        for path in [
            "src/App.tsx",
            "src/components/TopBar.tsx",
            "src/components/NavRail.tsx",
            "src/components/WorkflowStrip.tsx",
            "src/components/DataWorkspace.tsx",
            "src/components/ModelCanvas.tsx",
            "src/components/AnalysisCatalog.tsx",
            "src/components/RunWorkspace.tsx",
            "src/components/RunHistory.tsx",
            "src/components/ReportsWorkspace.tsx",
            "src/components/TrustCenterWorkspace.tsx",
            "src/components/SettingsWorkspace.tsx",
        ]
    )
    forbidden_mojibake = ["RÃ", "RÂ", "Ãƒ", "Â²"]
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]
    expected_views = {"welcome", "data", "models", "analyses", "run", "runs", "reports", "trust", "settings"}
    captured_views = {item.get("view") for item in smoke.get("captures", [])}
    captured_viewports = {item.get("viewport") for item in smoke.get("captures", [])}

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "all primary shell views captured": expected_views.issubset(captured_views),
        "desktop viewports captured": {"1440x900", "1280x800"}.issubset(captured_viewports),
        "current stage is v2.1.5": registry.get("current_stage") == target_stage,
        "registry contains v2.1.5 slice": any(s.get("id") == target_stage for s in registry.get("slices", [])),
        "package version is 2.1.5": package.get("version") == "2.1.5",
        "package lock root version is 2.1.5": package_lock.get("version") == "2.1.5" and package_lock.get("packages", {}).get("", {}).get("version") == "2.1.5",
        "cargo workspace version is 2.1.5": 'version = "2.1.5"' in cargo,
        "quickpls lock versions are 2.1.5": all(f'name = "{name}"\nversion = "2.1.5"' in cargo_lock for name in [
            "qpls-assessment",
            "qpls-cli",
            "qpls-core",
            "qpls-data",
            "qpls-estimation",
            "qpls-project",
            "qpls-resampling",
            "qpls-runner",
            "quickpls-desktop",
        ]),
        "tauri version is 2.1.5": tauri.get("version") == "2.1.5",
        "roadmap expects v2.1.5": target_stage in roadmap,
        "topbar shows v2.1.5": "v2.1.5 rendered shell consistency" in topbar,
        "artifact label is v2.1.5": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.1.5 scripts": all(key in package["scripts"] for key in [
            "qpls:v2115:shell-smoke",
            "qpls:v2115:shell-audit",
            "qpls:v2115:shell",
        ]),
        "smoke api supports all audited views": all(view in app for view in expected_views),
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No estimator, formula, method-validation, result-schema, project-archive, or numerical-fingerprint changes." in docs,
        "normal target source has no SmartPLS equivalence claim": not any(phrase in frontend_sources.lower() for phrase in forbidden_equivalence),
        "normal target source has no mojibake": not any(token in frontend_sources for token in forbidden_mojibake),
        "smoke and audit artifacts are referenced": all(token in docs for token in [
            "validation/v2115_rendered_shell_consistency_smoke.mjs",
            "validation/v2115_rendered_shell_consistency_audit.py",
        ]),
    }

    failed = [name for name, passed in checks.items() if not passed]
    report_payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v2115_rendered_shell_consistency_audit.json").write_text(json.dumps(report_payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report_payload, indent=2))
        return 1
    print("v2.1.5 rendered shell consistency audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
