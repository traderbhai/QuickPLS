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
    types = read_text("src/types.ts")
    component = read_text("src/components/WorkspaceCoach.tsx")
    commands = read_text("src/domain/workspaceCommands.ts")
    styles = read_text("src/styles.css")
    topbar = read_text("src/components/TopBar.tsx")
    docs = read_text("docs/V2_2_7_WORKFLOW_COMMAND_FEEDBACK.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v227_command_feedback_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_2_7_workflow_command_feedback"
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
    source_bundle = "\n".join([app, store, types, component, commands, styles, topbar])
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]
    forbidden_mojibake = ["RÃƒ", "RÃ‚Â²", "ÃƒÂ¢", "Ã‚ "]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.2.7": registry.get("current_stage") == target_stage,
        "registry contains v2.2.7 slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        "package version is 2.2.7": package.get("version") == "2.2.7",
        "package lock root version is 2.2.7": package_lock.get("version") == "2.2.7" and package_lock.get("packages", {}).get("", {}).get("version") == "2.2.7",
        "cargo workspace version is 2.2.7": 'version = "2.2.7"' in cargo,
        "quickpls lock versions are 2.2.7": all(f'name = "{name}"\nversion = "2.2.7"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.2.7": tauri.get("version") == "2.2.7",
        "roadmap expects v2.2.7": target_stage in roadmap,
        "topbar shows v2.2.7 command feedback": "v2.2.7 command feedback" in topbar,
        "artifact label is v2.2.7": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.2.7 scripts": all(key in package["scripts"] for key in [
            "qpls:v227:command-feedback-smoke",
            "qpls:v227:command-feedback-audit",
            "qpls:v227:command-feedback",
        ]),
        "workflow command type exists": "interface WorkflowCommandContext" in types,
        "store tracks command context": "workflowCommandContext" in store and "setWorkflowCommandContext" in store and "timestamp: Date.now()" in store,
        "coach writes command context": "setWorkflowCommandContext({ from: currentView" in component and "event: action.event" in component and "coachId: messageId" in component,
        "coach renders command note": "workspace-coach-command" in component and "data-command-event" in component and "Requested {commandLabel(commandContext.event)}" in component,
        "command dispatch contract preserved": "dispatchWorkspaceCommand(action.event!)" in component and "quickpls:run-analysis" in commands,
        "smoke api exposes command context": "getWorkflowCommandContext" in app,
        "styles include command note": ".workspace-coach-command" in styles,
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No statistical engine changes." in docs and "numerical fingerprint" in docs,
        "delivery status updated": target_stage in delivery and "v2.2.7 Workflow Command Feedback" in delivery,
        "development ledger updated": target_stage in ledger and "v2.2.7 Workflow Command Feedback" in ledger,
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
    (RESULTS / "v227_command_feedback_audit.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.2.7 workflow command feedback audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
