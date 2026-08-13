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
    target_stage = "v2_5_0_navigation_hierarchy_polish"
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    nav = read_text("src/components/NavRail.tsx")
    styles = read_text("src/styles.css")
    smoke_path = ROOT / "validation" / "results" / "v250_navigation_hierarchy_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
    bundle = source_bundle([
        "src/components/NavRail.tsx",
        "src/components/WorkflowStrip.tsx",
        "src/components/WorkspaceCoach.tsx",
        "src/styles.css",
        "docs/V2_5_0_NAVIGATION_HIERARCHY_POLISH.md",
        "docs/DELIVERY_STATUS.md",
        "docs/DEVELOPMENT_LEDGER.md",
    ])

    checks = {
        "navigation hierarchy smoke passed": bool(smoke.get("passed")),
        **shared_v2_metadata_checks(
            version="2.5.0",
            target_stage=target_stage,
            expected_label="v2.5.0 navigation hierarchy",
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "package exposes v2.5.0 scripts": all(key in package["scripts"] for key in [
            "qpls:v250:navigation-smoke",
            "qpls:v250:navigation-audit",
            "qpls:v250:navigation",
        ]),
        "nav rail has primary workflow grouping": "workflowItems" in nav and "Research workflow" in nav,
        "nav rail has support utility grouping": "utilityItems" in nav and "Support" in nav,
        "trust and settings stay available as utilities": 'view: "trust"' in nav and 'view: "settings"' in nav,
        "groups route is not primary rail": 'view: "groups"' not in nav,
        "nav source exposes stable data attributes": "data-nav-section" in nav and "data-nav-view" in nav,
        "styles visually separate utility section": ".nav-section.utility" in styles and "margin-top: auto" in styles,
        "normal navigation sources have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v250_navigation_hierarchy_audit.py")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v250_navigation_hierarchy_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.5.0 navigation hierarchy audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
