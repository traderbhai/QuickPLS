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
    target_stage = "v2_4_0_public_documentation_screenshot_refresh"
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    smoke_path = ROOT / "validation" / "results" / "v240_public_docs_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    source_files = [
        "README.md",
        "docs/INSTALLATION.md",
        "docs/BUILD_FROM_SOURCE.md",
        "docs/QUICK_START.md",
        "docs/USER_GUIDE.md",
        "docs/FIRST_PLS_MODEL_TUTORIAL.md",
        "docs/V2_4_0_PUBLIC_DOCUMENTATION_SCREENSHOT_REFRESH.md",
        "docs/DELIVERY_STATUS.md",
        "docs/DEVELOPMENT_LEDGER.md",
    ]
    bundle = source_bundle(source_files)
    stale_v1_release_tokens = [
        "Current release: `v1.8.1`",
        "Use the GitHub Release for `v1.8.1`",
        "Run Current v1.8.1 Gate",
        "RELEASE_CHECKSUMS_V1_8_1.txt",
        "GITHUB_RELEASE_V1_8_1.md",
    ]
    docs_dir = ROOT / "docs" / "screenshots" / "v2"

    checks = {
        "public docs smoke passed": bool(smoke.get("passed")),
        **shared_v2_metadata_checks(
            version="2.4.0",
            target_stage=target_stage,
            expected_label="v2.4.0 public docs and screenshots",
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "package exposes v2.4.0 scripts": all(key in package["scripts"] for key in [
            "qpls:v240:public-docs-smoke",
            "qpls:v240:public-docs-audit",
            "qpls:v240:public-docs",
        ]),
        "docs screenshot directory contains v2 assets": docs_dir.exists() and len(list(docs_dir.glob("*.png"))) >= 9,
        "normal public docs have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "normal public docs have no stale v1 release instructions": no_forbidden_tokens(bundle, stale_v1_release_tokens),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "README documents proprietary source availability": "proprietary license" in read_text("README.md").lower(),
        "installation documents unsigned installer": "unsigned" in read_text("docs/INSTALLATION.md").lower(),
        "build docs document versioned artifacts": "target/release/artifacts/" in read_text("docs/BUILD_FROM_SOURCE.md"),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v240_public_docs_audit.py")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v240_public_docs_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.4.0 public documentation screenshot refresh audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
