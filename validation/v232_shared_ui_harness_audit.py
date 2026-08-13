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
    target_stage = "v2_3_2_shared_ui_verification_harness"
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    milestone_doc = read_text("docs/V2_3_2_SHARED_UI_VERIFICATION_HARNESS.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    v231_smoke = read_text("validation/v231_ui_integrity_smoke.mjs")
    v231_audit = read_text("validation/v231_ui_integrity_audit.py")
    v232_smoke = read_text("validation/v232_shared_ui_harness_smoke.mjs")
    v232_audit = read_text("validation/v232_shared_ui_harness_audit.py")
    shared_smoke = read_text("validation/lib/v2_ui_smoke_harness.mjs")
    shared_audit = read_text("validation/lib/v2_ui_audit.py")

    smoke_path = ROOT / "validation" / "results" / "v232_shared_ui_harness_smoke.json"
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
        "docs/V2_3_1_UI_INTEGRITY_CONSOLIDATION.md",
        "docs/V2_3_2_SHARED_UI_VERIFICATION_HARNESS.md",
    ]
    bundle = source_bundle(source_files)

    checks = {
        "v2.3.2 smoke report passed": bool(smoke.get("passed")),
        **shared_v2_metadata_checks(
            version="2.3.2",
            target_stage=target_stage,
            expected_label="v2.3.2 shared UI verification harness",
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "package exposes v2.3.2 scripts": all(key in package["scripts"] for key in [
            "qpls:v232:harness-smoke",
            "qpls:v232:harness-audit",
            "qpls:v232:harness",
        ]),
        **command_bar_contract_checks(topbar),
        "v2.3.1 smoke imports shared smoke harness": "./lib/v2_ui_smoke_harness.mjs" in v231_smoke and "withPreviewPage" in v231_smoke,
        "v2.3.2 smoke imports shared smoke harness": "./lib/v2_ui_smoke_harness.mjs" in v232_smoke and "withPreviewPage" in v232_smoke,
        "v2.3.1 audit imports shared audit helpers": "from lib.v2_ui_audit import" in v231_audit and "shared_v2_metadata_checks" in v231_audit,
        "v2.3.2 audit imports shared audit helpers": "from lib.v2_ui_audit import" in v232_audit and "shared_v2_metadata_checks" in v232_audit,
        "only shared smoke harness starts vite preview": "npx vite preview" in shared_smoke and "npx vite preview" not in v231_smoke and "npx vite preview" not in v232_smoke,
        "shared audit helper owns package list": ("QUICKPLS_" + "PACKAGES =") in shared_audit and ("QUICKPLS_" + "PACKAGES =") not in v231_audit and ("QUICKPLS_" + "PACKAGES =") not in v232_audit,
        "normal v2 UI docs have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "milestone doc declares frontend product only": "frontend/product-only" in milestone_doc,
        "milestone doc declares no numerical changes": "No statistical engine changes." in milestone_doc and "No numerical fingerprint changes." in milestone_doc,
        "delivery status updated": target_stage in delivery and "v2.3.2 Shared UI Verification Harness" in delivery,
        "development ledger updated": target_stage in ledger and "v2.3.2 Shared UI Verification Harness" in ledger,
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v232_shared_ui_harness_audit.py")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v232_shared_ui_harness_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.3.2 shared UI verification harness audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
