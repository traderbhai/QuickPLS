import json

from lib.v2_ui_audit import (
    FORBIDDEN_MOJIBAKE,
    ROOT,
    frontend_boundary_check,
    no_forbidden_tokens,
    no_smartpls_equivalence,
    read_json,
    read_text,
    source_bundle,
    write_result,
)


def main() -> int:
    target_stage = "v2_6_0_launch_quality_visual_consolidation"
    package = read_json("package.json")
    active_doc = read_text("docs/V2_ACTIVE_MILESTONE.md")
    app = read_text("src/App.tsx")
    styles = read_text("src/styles.css")
    topbar = read_text("src/components/TopBar.tsx")
    smoke_path = ROOT / "validation" / "results" / "v260_launch_quality_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
    bundle = source_bundle([
        "src/App.tsx",
        "src/styles.css",
        "src/components/TopBar.tsx",
        "src/components/NavRail.tsx",
        "src/components/WorkflowStrip.tsx",
        "src/components/WorkspaceCoach.tsx",
        "src/components/DataWorkspace.tsx",
        "src/components/AnalysisCatalog.tsx",
        "src/components/RunWorkspace.tsx",
        "src/components/RunHistory.tsx",
        "src/components/ReportsWorkspace.tsx",
        "src/components/TrustCenterWorkspace.tsx",
        "src/components/SettingsWorkspace.tsx",
        "docs/V2_ACTIVE_MILESTONE.md",
    ])

    expected_scripts = [
        "qpls:v260:launch-quality-smoke",
        "qpls:v260:launch-quality-audit",
        "qpls:v260:launch-quality",
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
        "active tracker defines v2.6.0 milestone": target_stage in active_doc,
        "active tracker keeps milestone concise": "Do not repeat the full historical QuickPLS plan" in active_doc,
        "active tracker states frontend boundary": "frontend-only" in active_doc and "no estimator" in active_doc,
        "package exposes v2.6.0 scripts": all(key in package["scripts"] for key in expected_scripts),
        "topbar shows v2.6.0 launch quality label": "v2.6.0 launch quality" in topbar,
        "smoke output exists and passed": bool(smoke.get("passed")),
        "smoke covers desktop viewports": smoke_viewports == ["1280x800", "1440x900"],
        "smoke covers support and workflow views": {"welcome", "trust", "settings", "data", "models", "analyses", "run", "runs", "reports"}.issubset(smoke_views),
        "support shell implementation still present": "SUPPORT_UTILITY_VIEWS" in app and "support-utility-frame" in app,
        "workflow shell implementation still present": "WorkflowStrip" in app and "WorkspaceCoach" in app,
        "workspace gutters still centralized": ".workspace-page" in styles and "--q2-page-gutter-x" in styles,
        "topbar still exposes command run metadata": "command-run-cluster" in topbar and "data-run-disabled-reason" in topbar,
        "normal v2.6.0 sources have no mojibake": no_forbidden_tokens(bundle, FORBIDDEN_MOJIBAKE),
        "no SmartPLS equivalence claim": no_smartpls_equivalence(bundle),
        "audit file stays frontend scoped": frontend_boundary_check(read_text("validation/v260_launch_quality_audit.py")),
        "smoke file stays frontend scoped": frontend_boundary_check(read_text("validation/v260_launch_quality_smoke.mjs")),
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {"milestone": target_stage, "passed": not failed, "checks": checks, "failed": failed}
    write_result("v260_launch_quality_audit.json", payload)
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.6.0 launch quality visual consolidation audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
