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

OUTPUT = "v2400_mockup_fidelity_audit.json"
MILESTONE = "v2_40_0_mockup_fidelity_native_shell_alignment"


def main() -> int:
    package = read_json("package.json")
    registry = read_json("validation/development_slices.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    prototype = read_text("src/v2/NativePrototypeApp.tsx")
    css = read_text("src/v2/nativePrototype.css")
    doc = read_text("docs/V2_40_0_MOCKUP_FIDELITY_NATIVE_SHELL_ALIGNMENT.md")
    active = read_text("docs/V2_ACTIVE_MILESTONE.md")
    backlog = read_text("docs/V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md")
    smoke_path = RESULTS / "v2400_mockup_fidelity_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}
    current = next((item for item in registry["slices"] if item["id"] == MILESTONE), None)
    source = "\n".join([prototype, css, doc, active, backlog])

    checks = {
        "current_stage_registered": registry.get("current_stage") == MILESTONE,
        "registry_slice_validated": bool(current and current.get("status") == "validated"),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "roadmap_current_stage_updated": MILESTONE in roadmap,
        "package_scripts_registered": all(key in package.get("scripts", {}) for key in [
            "qpls:v2400:mockup-fidelity-smoke",
            "qpls:v2400:mockup-fidelity-audit",
            "qpls:v2400:mockup-fidelity",
        ]),
        "fidelity_marker_present": "data-v240-mockup-fidelity" in prototype,
        "ribbon_contract_present": "data-v240-ribbon" in prototype and ".np-ribbon" in css,
        "model_workbench_contract_present": all(token in prototype for token in [
            "data-v240-explorer-tree",
            "data-v240-inspector-tabs",
            "data-v240-bottom-tabs",
            "Object Inspector",
            "Diagram Advisor",
        ]),
        "extra_feature_backlog_exists": "Deferred Or Hidden During Mockup Parity" in backlog,
        "frontend_boundary": frontend_boundary_check(source),
        "no_mojibake_source": no_forbidden_tokens(source, FORBIDDEN_MOJIBAKE),
        "no_smartpls_equivalence_claim": no_smartpls_equivalence(source),
        "smoke_passed": smoke.get("passed") is True,
        "smoke_ribbon_passed": smoke.get("checks", {}).get("ribbon_core_commands_present") is True,
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
