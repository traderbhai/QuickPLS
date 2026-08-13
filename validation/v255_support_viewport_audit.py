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
    target_stage = "v2_5_5_support_shell_viewport_alignment"
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
    smoke_path = ROOT / "validation" / "results" / "v255_support_viewport_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
    milestone_doc = read_text("docs/V2_5_5_SUPPORT_SHELL_VIEWPORT_ALIGNMENT.md")
    bundle = source_bundle([
        "src/App.tsx",
        "src/styles.css",
        "docs/V2_5_5_SUPPORT_SHELL_VIEWPORT_ALIGNMENT.md",
        "docs/DELIVERY_STATUS.md",
        "docs/DEVELOPMENT_LEDGER.md",
    ])

    checks = {
        "support viewport smoke passed": bool(smoke.get("passed")),
        "support viewport smoke covers two desktop viewports": sorted(f"{run.get('viewport', {}).get('width')}x{run.get('viewport', {}).get('height')}" for run in smoke.get("runs", [])) == ["1280x800", "1440x900"],
        **shared_v2_metadata_checks(
            version="2.5.5",
            target_stage=target_stage,
            expected_label="v2.5.5 support shell alignment",
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "package exposes v2.5.5 scripts": all(key in package["scripts"] for key in [
            "qpls:v255:support-viewport-smoke",
            "qpls:v255:support-viewport-audit",
            "qpls:v255:support-viewport",
        ]),
        "artifact label updated": "v2_5_5_support_shell_viewport_alignment" in package["scripts"].get("qpls:release:artifacts", ""),
        "support utility frame wraps bar": "support-utility-frame" in app and "<SupportUtilityBar" in app,
        "support utility frame uses workspace gutters": ".support-utility-frame" in styles
        and "max-width: 1220px" in styles
        and "var(--q2-page-gutter-y) var(--q2-page-gutter-x)" in styles,
        "support bar no longer owns page margin": ".support-utility-bar" in styles and "margin: 0;" in styles,
        "support viewport responsive rule present": "@media (max-width: 900px)" in styles
        and ".support-utility-frame" in styles
        and ".support-utility-actions" in styles,
        "normal v2.5.5 sources have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "milestone document stays frontend scoped": frontend_boundary_check(milestone_doc),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v255_support_viewport_audit.py")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v255_support_viewport_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.5.5 support shell viewport alignment audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
