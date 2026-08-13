import json

from lib.v2_ui_audit import (
    FORBIDDEN_MOJIBAKE,
    ROOT,
    command_bar_contract_checks,
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
    target_stage = "v2_3_1_ui_integrity_consolidation"
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    visual_contract = read_text("docs/V2_UI_VISUAL_CONTRACT.md")
    milestone_doc = read_text("docs/V2_3_1_UI_INTEGRITY_CONSOLIDATION.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = ROOT / "validation" / "results" / "v231_ui_integrity_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    source_files = [
        "src/components/TopBar.tsx",
        "src/components/WorkflowStrip.tsx",
        "src/components/NavRail.tsx",
        "src/components/RunWorkspace.tsx",
        "src/components/ReportsWorkspace.tsx",
        "src/components/RunHistory.tsx",
        "src/components/ModelCanvas.tsx",
        "src/components/TrustCenterWorkspace.tsx",
        "src/components/SettingsWorkspace.tsx",
        "src/domain/methodApplicability.ts",
        "docs/V2_UI_VISUAL_CONTRACT.md",
        "docs/V2_3_0_GLOBAL_COMMAND_BAR_READINESS.md",
        "docs/V2_3_1_UI_INTEGRITY_CONSOLIDATION.md",
    ]
    bundle = source_bundle(source_files)
    stale_visible_labels = ["v2.3.0 command bar readiness", "v1.5.3 layout, copy, and readiness polish"]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        **shared_v2_metadata_checks(
            version="2.3.1",
            target_stage=target_stage,
            expected_label="v2.3.1 UI integrity consolidation",
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "package exposes v2.3.1 scripts": all(key in package["scripts"] for key in [
            "qpls:v231:ui-integrity-smoke",
            "qpls:v231:ui-integrity-audit",
            "qpls:v231:ui-integrity",
        ]),
        **command_bar_contract_checks(topbar),
        "visual contract keeps R squared readable": "R²/loadings/path overlays" in visual_contract,
        "normal v2 UI docs have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "normal v2 UI docs have no stale visible labels": no_forbidden_tokens(bundle, stale_visible_labels),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "milestone doc declares frontend product only": "frontend/product-only" in milestone_doc,
        "milestone doc declares no numerical changes": "No statistical engine changes." in milestone_doc and "No numerical fingerprint changes." in milestone_doc,
        "delivery status updated": target_stage in delivery and "v2.3.1 UI Integrity Consolidation" in delivery,
        "development ledger updated": target_stage in ledger and "v2.3.1 UI Integrity Consolidation" in ledger,
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v231_ui_integrity_audit.py")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v231_ui_integrity_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.3.1 UI integrity consolidation audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
