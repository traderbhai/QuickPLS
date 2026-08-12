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
    target_stage = "v2_9_0_acceptance_backlog_and_next_pass"
    version = "2.9.0"
    expected_label = "v2.9.0 acceptance backlog"

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
    milestone_doc = read_text("docs/V2_9_0_ACCEPTANCE_BACKLOG_AND_NEXT_PASS.md")
    notes = read_text("docs/RELEASE_NOTES_V2_9_0.md")
    smoke_path = ROOT / "validation" / "results" / "v290_acceptance_backlog_smoke.json"
    backlog_path = ROOT / "validation" / "results" / "v290_acceptance_backlog.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
    backlog = json.loads(backlog_path.read_text(encoding="utf-8")) if backlog_path.exists() else {}
    bundle = source_bundle([
        "docs/V2_ACTIVE_MILESTONE.md",
        "docs/V2_9_0_ACCEPTANCE_BACKLOG_AND_NEXT_PASS.md",
        "docs/DELIVERY_STATUS.md",
        "docs/DEVELOPMENT_LEDGER.md",
        "README.md",
        "docs/INSTALLATION.md",
        "docs/BUILD_FROM_SOURCE.md",
        "docs/RELEASE_NOTES_V2_9_0.md",
        "validation/v290_acceptance_backlog_smoke.mjs",
        "validation/v290_acceptance_backlog_audit.py",
        "src/components/TopBar.tsx",
    ])

    workstreams = backlog.get("workstreams", [])
    decisions = {item.get("decision") for item in workstreams}
    expected_scripts = [
        "qpls:v290:acceptance-backlog-smoke",
        "qpls:v290:acceptance-backlog-audit",
        "qpls:v290:acceptance-backlog",
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
        "package exposes v2.9.0 scripts": all(key in package["scripts"] for key in expected_scripts),
        "README names current v2.9.0 release": "Current development release: `v2.9.0`." in readme,
        "README verify commands point to v2.9.0": "npm run qpls:v290:acceptance-backlog" in readme and target_stage in readme,
        "README links release notes v2.9.0": "docs/RELEASE_NOTES_V2_9_0.md" in readme,
        "installation names current v2.9.0 release": "Current development release: `v2.9.0`." in install,
        "build docs use current v2.9.0 gate": "npm run qpls:v290:acceptance-backlog" in build and target_stage in build,
        "release notes describe acceptance backlog scope": "QuickPLS 2.9.0" in notes and "acceptance-backlog" in notes,
        "active tracker names next milestone": target_stage in active and "Next Active Milestone" in active,
        "active tracker preserves grouped milestone rules": "Work in larger grouped milestones" in active,
        "milestone doc records frontend boundary": "frontend/product-only" in milestone_doc and "No estimator" in milestone_doc,
        "smoke output exists and passed": bool(smoke.get("passed")),
        "backlog output exists": backlog.get("milestone") == target_stage,
        "backlog has grouped decisions": {"do_next", "defer", "do_not_do"}.issubset(decisions),
        "backlog has at least three do-next streams": sum(1 for item in workstreams if item.get("decision") == "do_next") >= 3,
        "each workstream has acceptance and evidence": all(
            len(item.get("acceptance", [])) >= 2 and len(item.get("evidence_needed", [])) >= 2
            for item in workstreams
        ),
        "smoke covers desktop viewports": smoke_viewports == ["1280x800", "1440x900"],
        "smoke covers workflow and support views": {"welcome", "data", "models", "analyses", "run", "runs", "reports", "trust", "settings"}.issubset(smoke_views),
        "normal v2.9.0 sources have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v290_acceptance_backlog_audit.py")),
        "smoke file stays frontend scoped": frontend_boundary_check(read_text("validation/v290_acceptance_backlog_smoke.mjs")),
    }

    failed = [name for name, passed in checks.items() if not passed]
    payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    write_result("v290_acceptance_backlog_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.9.0 acceptance backlog audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
