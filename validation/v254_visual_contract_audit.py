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
    target_stage = "v2_5_4_visual_contract_support_shell_alignment"
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    app = read_text("src/App.tsx")
    contract = read_text("docs/V2_UI_VISUAL_CONTRACT.md")
    milestone_doc = read_text("docs/V2_5_4_VISUAL_CONTRACT_SUPPORT_SHELL_ALIGNMENT.md")
    bundle = source_bundle([
        "src/App.tsx",
        "src/styles.css",
        "docs/V2_UI_VISUAL_CONTRACT.md",
        "docs/V2_5_4_VISUAL_CONTRACT_SUPPORT_SHELL_ALIGNMENT.md",
        "docs/DELIVERY_STATUS.md",
        "docs/DEVELOPMENT_LEDGER.md",
    ])

    checks = {
        **shared_v2_metadata_checks(
            version="2.5.4",
            target_stage=target_stage,
            expected_label="v2.5.4 visual contract alignment",
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "package exposes v2.5.4 scripts": all(key in package["scripts"] for key in [
            "qpls:v254:visual-contract-audit",
            "qpls:v254:visual-contract",
        ]),
        "artifact label updated": "v2_5_4_visual_contract_support_shell_alignment" in package["scripts"].get("qpls:release:artifacts", ""),
        "workflow contract excludes home": "Workflow strip: Data, Model, Setup, Run, Results, Report." in contract
        and "Workflow strip: Home, Data" not in contract,
        "support utility shell is contract rule": "Support utility shell: Home, Trust Center, and Settings" in contract
        and "Home, Trust Center, and Settings are launcher/support utilities" in contract,
        "model designer exception documented": "Model may keep a dedicated SEM Designer workflow band" in contract,
        "support utility implementation still matches contract": all(token in app for token in [
            "SUPPORT_UTILITY_VIEWS",
            '"welcome"',
            '"trust"',
            '"settings"',
            "SupportUtilityBar",
            'data-support-shell="launcher-support"',
        ]),
        "primary workflow implementation still excludes support utilities": '["data", "analyses", "run", "runs", "reports"]' in app,
        "visual contract has proper r squared text": "R²/loadings/path overlays" in contract and "RÂ²" not in contract,
        "normal v2.5.4 sources have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "milestone document stays frontend scoped": frontend_boundary_check(milestone_doc),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v254_visual_contract_audit.py")),
        "no stale v2.5.3 milestone label": "v2.5.3 support utility shell" not in topbar,
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v254_visual_contract_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.5.4 visual contract support-shell alignment audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
