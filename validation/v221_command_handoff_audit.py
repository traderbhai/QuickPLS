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
    commands = read_text("src/domain/workspaceCommands.ts")
    workflow = read_text("src/domain/workflowCoach.ts")
    coach = read_text("src/components/WorkspaceCoach.tsx")
    docs = read_text("docs/V2_2_1_COMMAND_HANDOFF_CONSISTENCY.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v221_command_handoff_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_2_1_command_handoff_consistency"
    previous_stage = "v2_2_0_workflow_continuity_command_clarity"
    command_events = [
        "quickpls:run-analysis",
        "quickpls:save-project",
        "quickpls:open-project",
        "quickpls:open-demo-project",
        "quickpls:import-data",
    ]
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
    source_bundle = "\n".join([topbar, commands, workflow, coach])
    forbidden_mojibake = ["RÃƒ", "RÃ‚", "Ã‚Â²"]
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.2.1": registry.get("current_stage") == target_stage,
        "registry contains v2.2.1 slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        "package version is 2.2.1": package.get("version") == "2.2.1",
        "package lock root version is 2.2.1": package_lock.get("version") == "2.2.1" and package_lock.get("packages", {}).get("", {}).get("version") == "2.2.1",
        "cargo workspace version is 2.2.1": 'version = "2.2.1"' in cargo,
        "quickpls lock versions are 2.2.1": all(f'name = "{name}"\nversion = "2.2.1"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.2.1": tauri.get("version") == "2.2.1",
        "roadmap expects v2.2.1": target_stage in roadmap,
        "topbar shows v2.2.1 command handoff": "v2.2.1 command handoff" in topbar,
        "artifact label is v2.2.1": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.2.1 scripts": all(key in package["scripts"] for key in [
            "qpls:v221:commands-smoke",
            "qpls:v221:commands-audit",
            "qpls:v221:commands",
        ]),
        "v2.2.0 script still points to v2.2.0 gate": previous_stage in package["scripts"].get("qpls:v220:workflow", ""),
        "shared command module has all events": all(event in commands for event in command_events) and "dispatchWorkspaceCommand" in commands,
        "workflow uses typed command events": "WorkspaceCommandEvent" in workflow and "quickpls:import-data" in workflow,
        "coach dispatches shared command helper": "dispatchWorkspaceCommand" in coach and "new CustomEvent" not in coach,
        "topbar listens to all command events": all(f'addEventListener("{event}"' in topbar and f'removeEventListener("{event}"' in topbar for event in command_events),
        "topbar import command is handled": "handleImportData" in topbar and "importDataCommand" in topbar,
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No estimator, formula, method-validation, result-schema, project-archive, or numerical-fingerprint changes." in docs,
        "delivery status updated": target_stage in delivery and "v2.2.1 Command Handoff Consistency" in delivery,
        "development ledger updated": target_stage in ledger and "v2.2.1 Command Handoff Consistency" in ledger,
        "target source has no SmartPLS equivalence claim": not any(phrase in source_bundle.lower() for phrase in forbidden_equivalence),
        "target source has no mojibake": not any(token in source_bundle for token in forbidden_mojibake),
    }

    failed = [name for name, passed in checks.items() if not passed]
    report_payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v221_command_handoff_audit.json").write_text(json.dumps(report_payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report_payload, indent=2))
        return 1
    print("v2.2.1 command handoff audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
