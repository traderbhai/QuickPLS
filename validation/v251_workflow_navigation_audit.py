import json

from lib.v2_ui_audit import (
    FORBIDDEN_MOJIBAKE,
    ROOT,
    frontend_boundary_check,
    no_forbidden_tokens,
    no_smartpls_equivalence,
    read_json,
    read_text,
    shared_v2_metadata_checks,
    source_bundle,
    write_result,
)


def main() -> int:
    target_stage = "v2_5_1_workflow_navigation_parity"
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    workflow = read_text("src/components/WorkflowStrip.tsx")
    nav = read_text("src/components/NavRail.tsx")
    styles = read_text("src/styles.css")
    smoke_path = ROOT / "validation" / "results" / "v251_workflow_navigation_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
    bundle = source_bundle([
        "src/components/NavRail.tsx",
        "src/components/WorkflowStrip.tsx",
        "src/domain/workflowProgress.ts",
        "src/styles.css",
        "docs/V2_5_1_WORKFLOW_NAVIGATION_PARITY.md",
        "docs/DELIVERY_STATUS.md",
        "docs/DEVELOPMENT_LEDGER.md",
    ])

    checks = {
        "workflow navigation smoke passed": bool(smoke.get("passed")),
        **shared_v2_metadata_checks(
            version="2.5.1",
            target_stage=target_stage,
            expected_label="v2.5.1 workflow navigation parity",
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "package exposes v2.5.1 scripts": all(key in package["scripts"] for key in [
            "qpls:v251:workflow-navigation-smoke",
            "qpls:v251:workflow-navigation-audit",
            "qpls:v251:workflow-navigation",
        ]),
        "workflow strip declares primary scope": 'data-workflow-scope="primary-research-workflow"' in workflow,
        "workflow strip exposes step count": "data-workflow-count={steps.length}" in workflow,
        "workflow aria separates support from primary flow": "Support destinations are available from the left navigation rail" in workflow,
        "workflow label is visible through markup": "workflow-strip-label" in workflow and ">Workflow</span>" in workflow and ".workflow-strip-label" in styles,
        "workflow remains six step calculation flow": '"data"' in read_text("src/domain/workflowProgress.ts") and '"reports"' in read_text("src/domain/workflowProgress.ts"),
        "support utilities stay in rail only": 'view: "trust"' in nav and 'view: "settings"' in nav and 'data-workflow-view="trust"' not in workflow,
        "normal navigation sources have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v251_workflow_navigation_audit.py")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v251_workflow_navigation_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.5.1 workflow navigation parity audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
