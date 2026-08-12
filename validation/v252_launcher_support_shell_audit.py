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
    target_stage = "v2_5_2_launcher_support_shell_separation"
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    app = read_text("src/App.tsx")
    workflow = read_text("src/components/WorkflowStrip.tsx")
    nav = read_text("src/components/NavRail.tsx")
    smoke_path = ROOT / "validation" / "results" / "v252_launcher_support_shell_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
    bundle = source_bundle([
        "src/App.tsx",
        "src/components/NavRail.tsx",
        "src/components/WorkflowStrip.tsx",
        "src/styles.css",
        "docs/V2_5_2_LAUNCHER_SUPPORT_SHELL_SEPARATION.md",
        "docs/DELIVERY_STATUS.md",
        "docs/DEVELOPMENT_LEDGER.md",
    ])

    checks = {
        "launcher support shell smoke passed": bool(smoke.get("passed")),
        **shared_v2_metadata_checks(
            version="2.5.2",
            target_stage=target_stage,
            expected_label="v2.5.2 launcher/support shell",
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "package exposes v2.5.2 scripts": all(key in package["scripts"] for key in [
            "qpls:v252:shell-separation-smoke",
            "qpls:v252:shell-separation-audit",
            "qpls:v252:shell-separation",
        ]),
        "app defines primary workflow page views": 'PRIMARY_WORKFLOW_PAGE_VIEWS' in app and '["data", "analyses", "run", "runs", "reports"]' in app,
        "app conditionally renders workflow band": "showPrimaryWorkflowBand" in app and 'has-workflow-band' in app and 'support-shell' in app,
        "home trust settings excluded from primary page views": '"welcome"' not in app.split("PRIMARY_WORKFLOW_PAGE_VIEWS", 1)[1].split(";", 1)[0] and '"trust"' not in app.split("PRIMARY_WORKFLOW_PAGE_VIEWS", 1)[1].split(";", 1)[0] and '"settings"' not in app.split("PRIMARY_WORKFLOW_PAGE_VIEWS", 1)[1].split(";", 1)[0],
        "model keeps dedicated workflow band": 'model-workflow-band"><WorkflowStrip /><WorkspaceCoach /></div>' in app,
        "workflow strip remains primary scoped": 'data-workflow-scope="primary-research-workflow"' in workflow and "data-workflow-count={steps.length}" in workflow,
        "support utilities stay in left rail": 'view: "trust"' in nav and 'view: "settings"' in nav,
        "normal shell sources have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v252_launcher_support_shell_audit.py")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v252_launcher_support_shell_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.5.2 launcher/support shell separation audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
