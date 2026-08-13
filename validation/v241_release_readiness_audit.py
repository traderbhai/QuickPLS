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


def _slice_by_id(registry: dict, slice_id: str) -> dict:
    for item in registry.get("slices", []):
        if item.get("id") == slice_id:
            return item
    return {}


def _gates_clear(slice_item: dict) -> bool:
    return bool(slice_item) and all(gate.get("status") == "passed" for gate in slice_item.get("gates", []))


def main() -> int:
    target_stage = "v2_4_1_quickpls_2_release_readiness_audit"
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    smoke_path = ROOT / "validation" / "results" / "v241_release_readiness_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    v2_docs = [
        "README.md",
        "docs/INSTALLATION.md",
        "docs/BUILD_FROM_SOURCE.md",
        "docs/QUICK_START.md",
        "docs/USER_GUIDE.md",
        "docs/FIRST_PLS_MODEL_TUTORIAL.md",
        "docs/V2_UI_VISUAL_CONTRACT.md",
        "docs/V2_4_0_PUBLIC_DOCUMENTATION_SCREENSHOT_REFRESH.md",
        "docs/V2_4_1_QUICKPLS_2_RELEASE_READINESS_AUDIT.md",
        "docs/DELIVERY_STATUS.md",
        "docs/DEVELOPMENT_LEDGER.md",
    ]
    bundle = source_bundle(v2_docs)
    current_entry_docs = source_bundle([
        "README.md",
        "docs/INSTALLATION.md",
        "docs/BUILD_FROM_SOURCE.md",
    ])
    required_slices = [
        "v2_3_2_shared_ui_verification_harness",
        "v2_4_0_public_documentation_screenshot_refresh",
        target_stage,
    ]
    stale_release_tokens = [
        "Current development release: `v2.4.0`",
        "v2.4.0 public docs and screenshots",
        "v2_4_0_public_documentation_screenshot_refresh_<timestamp>",
    ]

    checks = {
        "release readiness smoke passed": bool(smoke.get("passed")),
        **shared_v2_metadata_checks(
            version="2.4.1",
            target_stage=target_stage,
            expected_label="v2.4.1 release readiness",
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "required v2 prerequisite slices clear": all(_gates_clear(_slice_by_id(registry, slice_id)) for slice_id in required_slices),
        "package exposes v2.4.1 scripts": all(key in package["scripts"] for key in [
            "qpls:v241:release-readiness-smoke",
            "qpls:v241:release-readiness-audit",
            "qpls:v241:release-readiness",
        ]),
        "public docs use v2.4.1 current release": "Current development release: `v2.4.1`" in read_text("README.md"),
        "installation uses current v2.4.1 release": "v2.4.1" in read_text("docs/INSTALLATION.md"),
        "build docs use current v2.4.1 gate": "v2_4_1_quickpls_2_release_readiness_audit" in read_text("docs/BUILD_FROM_SOURCE.md"),
        "normal public docs have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "current entry docs have no stale v2.4.0 release wording": no_forbidden_tokens(current_entry_docs, stale_release_tokens),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v241_release_readiness_audit.py")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v241_release_readiness_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.4.1 QuickPLS 2 release readiness audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
