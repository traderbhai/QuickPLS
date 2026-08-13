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
    component = read_text("src/components/WorkspaceCoach.tsx")
    coach = read_text("src/domain/workflowCoach.ts")
    commands = read_text("src/domain/workspaceCommands.ts")
    styles = read_text("src/styles.css")
    docs = read_text("docs/V2_2_4_WORKFLOW_COACH_ACTION_CLARITY.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v224_coach_actions_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_2_4_workflow_coach_action_clarity"
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
    source_bundle = "\n".join([topbar, component, coach, commands, styles])
    forbidden_mojibake = [
        "RÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢",
        "RÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡",
        "ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â²",
    ]
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.2.4": registry.get("current_stage") == target_stage,
        "registry contains v2.2.4 slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        "package version is 2.2.4": package.get("version") == "2.2.4",
        "package lock root version is 2.2.4": package_lock.get("version") == "2.2.4" and package_lock.get("packages", {}).get("", {}).get("version") == "2.2.4",
        "cargo workspace version is 2.2.4": 'version = "2.2.4"' in cargo,
        "quickpls lock versions are 2.2.4": all(f'name = "{name}"\nversion = "2.2.4"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.2.4": tauri.get("version") == "2.2.4",
        "roadmap expects v2.2.4": target_stage in roadmap,
        "topbar shows v2.2.4 coach actions": "v2.2.4 coach actions" in topbar,
        "artifact label is v2.2.4": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.2.4 scripts": all(key in package["scripts"] for key in [
            "qpls:v224:coach-actions-smoke",
            "qpls:v224:coach-actions-audit",
            "qpls:v224:coach-actions",
        ]),
        "coach actions expose stable metadata": "data-action-label" in component and "data-action-disabled" in component,
        "disabled actions expose aria and visible reason": "aria-describedby" in component and "workspace-coach-action-reason" in component,
        "duplicate actions are suppressed": "actionIdentity" in component and "!== actionIdentity(message.primary)" in component,
        "command dispatch stays event based": "dispatchWorkspaceCommand" in component and "setTimeout" in component,
        "model incomplete secondary action is not duplicate": 'secondary: { label: "Open Data", view: "data" }' in coach,
        "coach uses existing command names": "quickpls:import-data" in commands and "quickpls:open-project" in commands,
        "no removed placeholder commands returned": "quickpls:validate-diagram" not in coach and "quickpls:load-sample" not in coach,
        "css styles action blocks and reasons": ".workspace-coach-action-block" in styles and ".workspace-coach-action-reason" in styles,
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No statistical engine changes." in docs and "numerical fingerprint" in docs,
        "delivery status updated": target_stage in delivery and "v2.2.4 Workflow Coach Action Clarity" in delivery,
        "development ledger updated": target_stage in ledger and "v2.2.4 Workflow Coach Action Clarity" in ledger,
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
    (RESULTS / "v224_coach_actions_audit.json").write_text(json.dumps(report_payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report_payload, indent=2))
        return 1
    print("v2.2.4 workflow coach action clarity audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
