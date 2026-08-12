import json
from datetime import UTC, datetime

from lib.v2_ui_audit import (
    FORBIDDEN_MOJIBAKE,
    RESULTS,
    frontend_boundary_check,
    no_forbidden_tokens,
    no_smartpls_equivalence,
    read_json,
    read_text,
    write_result,
)

OUTPUT = "v2390_native_frontend_screen_replacement_audit.json"
MILESTONE = "v2_39_0_native_frontend_screen_replacement_plan"


def main() -> int:
    package = read_json("package.json")
    registry = read_json("validation/development_slices.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    app = read_text("src/App.tsx")
    prototype = read_text("src/v2/NativePrototypeApp.tsx")
    doc = read_text("docs/V2_39_0_NATIVE_FRONTEND_SCREEN_REPLACEMENT_PLAN.md")
    active = read_text("docs/V2_ACTIVE_MILESTONE.md")
    smoke_path = RESULTS / "v2390_native_frontend_screen_replacement_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}
    current = next((item for item in registry["slices"] if item["id"] == MILESTONE), None)
    source = "\n".join([app, prototype, doc, active])

    checks = {
        "current_stage_registered": registry.get("current_stage") == MILESTONE,
        "registry_slice_validated": bool(current and current.get("status") == "validated"),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "roadmap_current_stage_updated": MILESTONE in roadmap,
        "package_scripts_registered": all(key in package.get("scripts", {}) for key in [
            "qpls:v239:screen-replacement-smoke",
            "qpls:v239:screen-replacement-audit",
            "qpls:v239:screen-replacement",
        ]),
        "candidate_flag_added": "native_shell" in app and "NativeShellCandidateApp" in app,
        "prototype_flag_still_available": "native_prototype" in app,
        "route_mapping_explicit": all(token in app for token in [
            "mapNativeViewToWorkspace",
            "mapWorkspaceViewToNative",
            "\"welcome\"",
            "\"models\"",
            "\"analyses\"",
            "\"runs\"",
            "\"reports\"",
        ]),
        "candidate_mode_marked": "data-v239-shell-mode" in prototype and "production-candidate" in prototype,
        "frontend_boundary": frontend_boundary_check(source),
        "no_mojibake_source": no_forbidden_tokens(source, FORBIDDEN_MOJIBAKE),
        "no_smartpls_equivalence_claim": no_smartpls_equivalence(source),
        "smoke_passed": smoke.get("passed") is True,
        "smoke_routes_synced": smoke.get("checks", {}).get("workspace_routes_synced") is True,
    }
    issues = [
        {"id": key, "severity": "high", "detail": f"Failed check: {key}"}
        for key, passed in checks.items()
        if not passed
    ]
    payload = {
        "passed": not issues,
        "milestone": MILESTONE,
        "generatedAt": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "checks": checks,
        "issues": issues,
    }
    write_result(OUTPUT, payload)
    print(json.dumps(payload, indent=2))
    return 0 if payload["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
