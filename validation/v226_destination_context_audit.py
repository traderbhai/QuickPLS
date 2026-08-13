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
    styles = read_text("src/styles.css")
    topbar = read_text("src/components/TopBar.tsx")
    docs = read_text("docs/V2_2_6_WORKFLOW_DESTINATION_CONTEXT.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v226_destination_context_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_2_6_workflow_destination_context"
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
    source_bundle = "\n".join([app, store, types, component, styles, topbar])
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]
    forbidden_mojibake = ["RÃ", "RÂ²", "Ã¢", "Â "]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.2.6": registry.get("current_stage") == target_stage,
        "registry contains v2.2.6 slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        "package version is 2.2.6": package.get("version") == "2.2.6",
        "package lock root version is 2.2.6": package_lock.get("version") == "2.2.6" and package_lock.get("packages", {}).get("", {}).get("version") == "2.2.6",
        "cargo workspace version is 2.2.6": 'version = "2.2.6"' in cargo,
        "quickpls lock versions are 2.2.6": all(f'name = "{name}"\nversion = "2.2.6"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.2.6": tauri.get("version") == "2.2.6",
        "roadmap expects v2.2.6": target_stage in roadmap,
        "topbar shows v2.2.6 destination context": "v2.2.6 destination context" in topbar,
        "artifact label is v2.2.6": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.2.6 scripts": all(key in package["scripts"] for key in [
            "qpls:v226:destination-context-smoke",
            "qpls:v226:destination-context-audit",
            "qpls:v226:destination-context",
        ]),
        "workflow destination type exists": "interface WorkflowDestinationContext" in types,
        "store tracks destination context": "workflowDestinationContext" in store and "timestamp: Date.now()" in store,
        "coach writes destination context": "actionLabel: action.label" in component and "coachId: messageId" in component,
        "coach renders destination note": "workspace-coach-destination" in component and "data-destination-action" in component,
        "smoke api exposes destination context": "getWorkflowDestinationContext" in app,
        "styles include destination note": ".workspace-coach-destination" in styles,
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No statistical engine changes." in docs and "numerical fingerprint" in docs,
        "delivery status updated": target_stage in delivery and "v2.2.6 Workflow Destination Context" in delivery,
        "development ledger updated": target_stage in ledger and "v2.2.6 Workflow Destination Context" in ledger,
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
    (RESULTS / "v226_destination_context_audit.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.2.6 workflow destination context audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
