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
    styles = read_text("src/styles.css")
    workflow = read_text("src/domain/workflowCoach.ts")
    coach = read_text("src/components/WorkspaceCoach.tsx")
    docs = read_text("docs/V2_2_0_WORKFLOW_CONTINUITY_COMMAND_CLARITY.md")
    smoke_path = RESULTS / "v220_workflow_continuity_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_2_0_workflow_continuity_command_clarity"
    expected_views = {"welcome", "data", "analyses", "run", "runs", "reports", "trust", "settings"}
    captured_views = {item.get("view") for item in smoke.get("captures", [])}
    captured_viewports = {item.get("viewport") for item in smoke.get("captures", [])}
    quickpls_packages = [
        "qpls-assessment",
        "qpls-cli",
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-project",
        "qpls-resampling",
        "qpls-runner",
        "quickpls-desktop",
    ]
    source_bundle = "\n".join([app, topbar, workflow, coach, styles])
    forbidden_mojibake = ["RÃ", "RÂ", "Â²"]
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "all workflow coach views captured": expected_views.issubset(captured_views),
        "desktop viewports captured": {"1440x900", "1280x800"}.issubset(captured_viewports),
        "current stage is v2.2.0": registry.get("current_stage") == target_stage,
        "registry contains v2.2.0 slice": any(s.get("id") == target_stage for s in registry.get("slices", [])),
        "package version is 2.2.0": package.get("version") == "2.2.0",
        "package lock root version is 2.2.0": package_lock.get("version") == "2.2.0" and package_lock.get("packages", {}).get("", {}).get("version") == "2.2.0",
        "cargo workspace version is 2.2.0": 'version = "2.2.0"' in cargo,
        "quickpls lock versions are 2.2.0": all(f'name = "{name}"\nversion = "2.2.0"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.2.0": tauri.get("version") == "2.2.0",
        "roadmap expects v2.2.0": target_stage in roadmap,
        "topbar shows v2.2.0 workflow clarity": "v2.2.0 workflow clarity" in topbar,
        "artifact label is v2.2.0": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.2.0 scripts": all(key in package["scripts"] for key in [
            "qpls:v220:workflow-smoke",
            "qpls:v220:workflow-audit",
            "qpls:v220:workflow",
        ]),
        "old v2.1.5 script still points to v2.1.5 gate": "v2_1_5_rendered_shell_consistency_audit" in package["scripts"].get("qpls:v2115:shell", ""),
        "app renders workflow coach outside model branch": "WorkspaceCoach" in app and "<WorkflowStrip />" in app,
        "workflow coach has accessible marker": 'aria-label="Workflow coach"' in coach and "data-coach-id" in coach,
        "workflow logic covers expected views": all(f'view === "{view}"' in workflow for view in expected_views),
        "workflow coach styles present": ".workspace-coach" in styles and ".workspace-coach-actions" in styles,
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No estimator, formula, method-validation, result-schema, project-archive, or numerical-fingerprint changes." in docs,
        "target source has no SmartPLS equivalence claim": not any(phrase in source_bundle.lower() for phrase in forbidden_equivalence),
        "target source has no mojibake": not any(token in source_bundle for token in forbidden_mojibake),
        "smoke and audit artifacts are referenced": all(token in docs for token in [
            "validation/v220_workflow_continuity_smoke.mjs",
            "validation/v220_workflow_continuity_audit.py",
        ]),
    }

    failed = [name for name, passed in checks.items() if not passed]
    report_payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v220_workflow_continuity_audit.json").write_text(json.dumps(report_payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report_payload, indent=2))
        return 1
    print("v2.2.0 workflow continuity audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
