import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
RESULTS.mkdir(parents=True, exist_ok=True)


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8-sig")


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
    app = read_text("src/App.tsx")
    store = read_text("src/store.ts")
    component = read_text("src/components/WorkspaceCoach.tsx")
    styles = read_text("src/styles.css")
    topbar = read_text("src/components/TopBar.tsx")
    docs = read_text("docs/V2_2_8_WORKFLOW_FEEDBACK_LIFECYCLE.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v228_feedback_lifecycle_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_2_8_workflow_feedback_lifecycle"
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
    source_bundle = "\n".join([app, store, component, styles, topbar])
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]
    forbidden_mojibake = ["RÃƒÆ’", "RÃƒâ€šÃ‚Â²", "ÃƒÆ’Ã‚Â¢", "Ãƒâ€š "]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.2.8": registry.get("current_stage") == target_stage,
        "registry contains v2.2.8 slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        "package version is 2.2.8": package.get("version") == "2.2.8",
        "package lock root version is 2.2.8": package_lock.get("version") == "2.2.8" and package_lock.get("packages", {}).get("", {}).get("version") == "2.2.8",
        "cargo workspace version is 2.2.8": 'version = "2.2.8"' in cargo,
        "quickpls lock versions are 2.2.8": all(f'name = "{name}"\nversion = "2.2.8"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.2.8": tauri.get("version") == "2.2.8",
        "roadmap expects v2.2.8": target_stage in roadmap,
        "topbar shows v2.2.8 feedback lifecycle": "v2.2.8 feedback lifecycle" in topbar,
        "artifact label is v2.2.8": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.2.8 scripts": all(key in package["scripts"] for key in [
            "qpls:v228:feedback-lifecycle-smoke",
            "qpls:v228:feedback-lifecycle-audit",
            "qpls:v228:feedback-lifecycle",
        ]),
        "store exposes clear feedback action": "clearWorkflowFeedback" in store and "workflowDestinationContext: null, workflowCommandContext: null" in store,
        "navigation clears command feedback": "const workflowCommandContext = state.view === nextView ? state.workflowCommandContext : null" in store,
        "dataset replacement clears feedback": "setDataset: (dataset) => set({ dataset, view: \"data\", workflowDestinationContext: null, workflowCommandContext: null })" in store,
        "project reset clears feedback": "resetProject: () => set({" in store and "workflowDestinationContext: null" in store and "workflowCommandContext: null" in store,
        "project load clears feedback": "loadProject: (project) => set({" in store and "workflowDestinationContext: null" in store and "workflowCommandContext: null" in store,
        "coach renders dismiss action": "workspace-coach-feedback-dismiss" in component and "onClick={clearWorkflowFeedback}" in component,
        "styles include feedback lifecycle classes": ".workspace-coach-feedback-line" in styles and ".workspace-coach-feedback-dismiss" in styles,
        "smoke api exposes contexts": "getWorkflowCommandContext" in app and "getWorkflowDestinationContext" in app and "loadEmptyProject" in app,
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No statistical engine changes." in docs and "numerical fingerprint" in docs,
        "delivery status updated": target_stage in delivery and "v2.2.8 Workflow Feedback Lifecycle" in delivery,
        "development ledger updated": target_stage in ledger and "v2.2.8 Workflow Feedback Lifecycle" in ledger,
        "target source has no SmartPLS equivalence claim": not any(phrase in source_bundle.lower() for phrase in forbidden_equivalence),
        "target source has no mojibake": not any(token in source_bundle for token in forbidden_mojibake),
    }

    failed = [name for name, passed in checks.items() if not passed]
    payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v228_feedback_lifecycle_audit.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.2.8 workflow feedback lifecycle audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
