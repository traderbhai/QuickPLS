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
    target_stage = "v2_7_0_visual_issue_register"
    version = "2.7.0"
    expected_label = "v2.7.0 visual issue register"

    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    active_doc = read_text("docs/V2_ACTIVE_MILESTONE.md")
    smoke_path = ROOT / "validation" / "results" / "v270_visual_issue_register_smoke.json"
    register_path = ROOT / "validation" / "results" / "v270_visual_issue_register.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
    register = json.loads(register_path.read_text(encoding="utf-8")) if register_path.exists() else {}
    bundle = source_bundle([
        "src/App.tsx",
        "src/styles.css",
        "src/components/TopBar.tsx",
        "src/components/NavRail.tsx",
        "src/components/WorkflowStrip.tsx",
        "src/components/WorkspaceCoach.tsx",
        "src/components/DataWorkspace.tsx",
        "src/components/AnalysisCatalog.tsx",
        "src/components/RunWorkspace.tsx",
        "src/components/RunHistory.tsx",
        "src/components/ReportsWorkspace.tsx",
        "src/components/TrustCenterWorkspace.tsx",
        "src/components/SettingsWorkspace.tsx",
        "docs/V2_ACTIVE_MILESTONE.md",
        "docs/V2_7_0_VISUAL_ISSUE_REGISTER.md",
    ])

    expected_scripts = [
        "qpls:v270:visual-issue-smoke",
        "qpls:v270:visual-issue-audit",
        "qpls:v270:visual-issue-register",
    ]
    smoke_viewports = sorted(
        f"{run.get('viewport', {}).get('width')}x{run.get('viewport', {}).get('height')}"
        for run in smoke.get("runs", [])
    )
    smoke_views = {
        item.get("view")
        for run in smoke.get("runs", [])
        for item in run.get("states", [])
    }

    checks = {
        **shared_v2_metadata_checks(
            version=version,
            target_stage=target_stage,
            expected_label=expected_label,
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "active tracker names v2.7.0 checkpoint": target_stage in active_doc,
        "active tracker preserves grouped milestone rules": "Work in larger grouped milestones" in active_doc,
        "package exposes v2.7.0 scripts": all(key in package["scripts"] for key in expected_scripts),
        "smoke output exists and passed": bool(smoke.get("passed")),
        "visual issue register exists": register.get("milestone") == target_stage,
        "visual issue register is currently clear": register.get("open_issues") == [],
        "smoke covers desktop viewports": smoke_viewports == ["1280x800", "1440x900"],
        "smoke covers workflow and support views": {"welcome", "data", "models", "analyses", "run", "runs", "reports", "trust", "settings"}.issubset(smoke_views),
        "normal v2.7.0 sources have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v270_visual_issue_register_audit.py")),
        "smoke file stays frontend scoped": frontend_boundary_check(read_text("validation/v270_visual_issue_register_smoke.mjs")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v270_visual_issue_register_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.7.0 visual issue register audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
