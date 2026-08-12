import json
import re

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


def contains_all(text: str, needles: list[str]) -> bool:
    return all(needle in text for needle in needles)


def main() -> int:
    target_stage = "v2_8_0_release_handoff_consistency"
    version = "2.8.0"
    expected_label = "v2.8.0 release handoff"

    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    readme = read_text("README.md")
    install = read_text("docs/INSTALLATION.md")
    build = read_text("docs/BUILD_FROM_SOURCE.md")
    active = read_text("docs/V2_ACTIVE_MILESTONE.md")
    notes = read_text("docs/RELEASE_NOTES_V2_8_0.md")
    bundle = source_bundle([
        "README.md",
        "docs/INSTALLATION.md",
        "docs/BUILD_FROM_SOURCE.md",
        "docs/V2_ACTIVE_MILESTONE.md",
        "docs/V2_8_0_RELEASE_HANDOFF_CONSISTENCY.md",
        "docs/RELEASE_NOTES_V2_8_0.md",
        "src/components/TopBar.tsx",
    ])

    expected_scripts = [
        "qpls:v280:release-handoff-audit",
        "qpls:v280:release-handoff",
        "qpls:desktop:build-versioned",
    ]
    release_artifacts = ROOT / "validation" / "results" / "release_artifacts.json"
    screenshot_dir = ROOT / "docs" / "screenshots" / "v2"
    screenshot_names = [
        "home.png",
        "data-workspace.png",
        "sem-designer.png",
        "setup-guided-methods.png",
        "run-workspace.png",
        "results-workspace.png",
        "report-workspace.png",
        "trust-center.png",
    ]

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
        "package exposes v2.8.0 scripts": all(key in package["scripts"] for key in expected_scripts),
        "README names current v2.8.0 release": "Current development release: `v2.8.0`." in readme,
        "README verify commands point to v2.8.0": contains_all(readme, [
            "npm run qpls:v280:release-handoff",
            "cargo run -p qpls-cli -- gate v2_8_0_release_handoff_consistency",
        ]),
        "README links release notes v2.8.0": "docs/RELEASE_NOTES_V2_8_0.md" in readme,
        "installation names current v2.8.0 release": "Current development release: `v2.8.0`." in install,
        "installation explains installer portable checksums": contains_all(install, [
            "_x64_setup.exe",
            "_x64_portable.exe",
            "_x64_checksums.txt",
            "Windows SmartScreen",
        ]),
        "build docs use current v2 gate": contains_all(build, [
            "npm run qpls:v280:release-handoff",
            "cargo run -p qpls-cli -- gate v2_8_0_release_handoff_consistency",
            "npm run qpls:desktop:build-versioned",
        ]),
        "active tracker names v2.8.0 milestone": target_stage in active,
        "active tracker keeps artifact rule": "installer, portable exe, and checksums" in active,
        "release notes describe handoff scope": contains_all(notes, [
            "QuickPLS 2.8.0",
            "release handoff",
            "No statistical engines",
        ]),
        "release artifact manifest exists": release_artifacts.exists(),
        "README screenshot files exist": all((screenshot_dir / name).exists() for name in screenshot_names),
        "README does not contain stale v2.4.1 release status": "Current development release: `v2.4.1`." not in readme,
        "installation does not contain stale v2.4.1 release status": "Current development release: `v2.4.1`." not in install,
        "build docs do not use stale v2.4 gate as current": "Run Current v2 Documentation Gate" not in build and "qpls:v240:public-docs" not in build,
        "normal v2.8.0 docs have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v280_release_handoff_audit.py")),
    }

    stale_versions = sorted(set(re.findall(r"v2\.(?:4\.1|6\.0|7\.0)", readme + install + build)))
    checks["public docs avoid stale current-version markers"] = not stale_versions

    failed = [name for name, passed in checks.items() if not passed]
    payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
        "stale_versions": stale_versions,
    }
    write_result("v280_release_handoff_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.8.0 release handoff audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
