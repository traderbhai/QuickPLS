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
    topbar = read_text("src/components/TopBar.tsx")
    component = read_text("src/components/WorkspaceCoach.tsx")
    coach = read_text("src/domain/workflowCoach.ts")
    docs = read_text("docs/V2_2_5_WORKFLOW_COACH_ACTION_EXECUTION.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v225_coach_execution_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_2_5_workflow_coach_action_execution"
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
    source_bundle = "\n".join([app, topbar, component, coach])
    forbidden_mojibake = [
        "RÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢",
        "RÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡",
        "ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â²",
    ]
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.2.5": registry.get("current_stage") == target_stage,
        "registry contains v2.2.5 slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        "package version is 2.2.5": package.get("version") == "2.2.5",
        "package lock root version is 2.2.5": package_lock.get("version") == "2.2.5" and package_lock.get("packages", {}).get("", {}).get("version") == "2.2.5",
        "cargo workspace version is 2.2.5": 'version = "2.2.5"' in cargo,
        "quickpls lock versions are 2.2.5": all(f'name = "{name}"\nversion = "2.2.5"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.2.5": tauri.get("version") == "2.2.5",
        "roadmap expects v2.2.5": target_stage in roadmap,
        "topbar shows v2.2.5 coach execution": "v2.2.5 coach execution" in topbar,
        "artifact label is v2.2.5": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.2.5 scripts": all(key in package["scripts"] for key in [
            "qpls:v225:coach-execution-smoke",
            "qpls:v225:coach-execution-audit",
            "qpls:v225:coach-execution",
        ]),
        "coach actions expose target metadata": "data-action-view" in component and "data-action-event" in component,
        "coach action metadata from v2.2.4 retained": "data-action-label" in component and "data-action-disabled" in component and "workspace-coach-action-reason" in component,
        "smoke api exposes getView": "getView: () => string" in app and "getView: () => useWorkspace.getState().view" in app,
        "label normalization applied": "function coachLabel" in coach and '"Open model": "Open Model"' in coach and '"Import Data"' in coach and '"Run Now"' in coach and '"Run Method"' in coach and '"Review Model"' in coach,
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No statistical engine changes." in docs and "numerical fingerprint" in docs,
        "delivery status updated": target_stage in delivery and "v2.2.5 Workflow Coach Action Execution" in delivery,
        "development ledger updated": target_stage in ledger and "v2.2.5 Workflow Coach Action Execution" in ledger,
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
    (RESULTS / "v225_coach_execution_audit.json").write_text(json.dumps(report_payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report_payload, indent=2))
        return 1
    print("v2.2.5 workflow coach action execution audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
