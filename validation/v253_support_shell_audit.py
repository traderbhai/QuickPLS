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
    target_stage = "v2_5_3_support_utility_shell_polish"
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    app = read_text("src/App.tsx")
    styles = read_text("src/styles.css")
    smoke_path = ROOT / "validation" / "results" / "v253_support_shell_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
    bundle = source_bundle([
        "src/App.tsx",
        "src/styles.css",
        "docs/V2_5_3_SUPPORT_UTILITY_SHELL_POLISH.md",
        "docs/DELIVERY_STATUS.md",
        "docs/DEVELOPMENT_LEDGER.md",
    ])

    support_view_decl = app.split("SUPPORT_UTILITY_VIEWS", 1)[1].split(";", 1)[0] if "SUPPORT_UTILITY_VIEWS" in app else ""
    checks = {
        "support shell smoke passed": bool(smoke.get("passed")),
        **shared_v2_metadata_checks(
            version="2.5.3",
            target_stage=target_stage,
            expected_label="v2.5.3 support utility shell",
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "package exposes v2.5.3 scripts": all(key in package["scripts"] for key in [
            "qpls:v253:support-shell-smoke",
            "qpls:v253:support-shell-audit",
            "qpls:v253:support-shell",
        ]),
        "support utility views are explicit": all(token in support_view_decl for token in ['"welcome"', '"trust"', '"settings"']),
        "support bar has stable hook": 'data-support-shell="launcher-support"' in app,
        "support bar actions have stable hooks": 'data-support-view={item.view}' in app and "aria-current" in app,
        "support bar is conditional": "showSupportUtilityBar" in app and "SUPPORT_UTILITY_VIEWS.includes(view)" in app,
        "primary workflow page list unchanged": '["data", "analyses", "run", "runs", "reports"]' in app,
        "support shell CSS present": ".support-utility-bar" in styles and ".support-utility-action.active" in styles,
        "normal support shell sources have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v253_support_shell_audit.py")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v253_support_shell_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.5.3 support utility shell audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
