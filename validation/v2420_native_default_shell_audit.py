import json
from datetime import UTC, datetime
from pathlib import Path

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

MILESTONE = "v2_42_0_make_native_mockup_shell_default"
OUTPUT = "v2420_native_default_shell_audit.json"
SMOKE_OUTPUT = RESULTS / "v2420_native_default_shell_smoke.json"


def load_result(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    package = read_json("package.json")
    registry = read_json("validation/development_slices.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    app = read_text("src/App.tsx")
    prototype = read_text("src/v2/NativePrototypeApp.tsx")
    smoke_script = read_text("validation/v2420_native_default_shell_smoke.mjs")
    this_audit = read_text("validation/v2420_native_default_shell_audit.py")
    doc = read_text("docs/V2_42_0_MAKE_NATIVE_MOCKUP_SHELL_DEFAULT.md")
    active = read_text("docs/V2_ACTIVE_MILESTONE.md")
    smoke = load_result(SMOKE_OUTPUT)
    scripts = package.get("scripts", {})
    current = next((item for item in registry["slices"] if item["id"] == MILESTONE), None)
    source = "\n".join([app, prototype, smoke_script, this_audit, doc, active])
    user_facing_source = "\n".join([prototype, doc, active])
    screenshot_dir = RESULTS / "screens" / "v2420" / "native-default"
    expected_screenshots = [
        "home.png",
        "data.png",
        "model.png",
        "setup.png",
        "run.png",
        "results.png",
        "report.png",
        "trust.png",
        "settings.png",
        "legacy-shell-fallback.png",
    ]
    screenshot_checks = {
        f"screenshot_{name}": (screenshot_dir / name).exists()
        for name in expected_screenshots
    }

    checks = {
        "current_stage_registered": registry.get("current_stage") == MILESTONE,
        "registry_slice_validated": bool(current and current.get("status") == "validated"),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "roadmap_current_stage_updated": MILESTONE in roadmap,
        "package_scripts_registered": all(key in scripts for key in [
            "qpls:v2420:native-default-smoke",
            "qpls:v2420:native-default-audit",
            "qpls:v2420:native-default",
        ]),
        "default_route_returns_native_shell": "return <NativeShellCandidateApp />;" in app and "if (params.has(\"legacy_shell\"))" in app,
        "legacy_shell_fallback_present": "return <LegacyApp />;" in app and "legacy_shell" in app,
        "native_prototype_route_preserved": "native_prototype" in app and 'mode="prototype"' in app,
        "native_shell_route_preserved": "native_shell" in app,
        "native_shell_mode_production_candidate": 'mode="production-candidate"' in app,
        "native_root_markers_preserved": all(token in prototype for token in [
            'data-v237-native-prototype="true"',
            'data-v241-mockup-parity="true"',
            "data-v239-shell-mode={mode}",
        ]),
        "smoke_passed": smoke.get("passed") is True,
        "smoke_validated_default_route": bool(smoke.get("checks", {}).get("default_route_uses_native_shell")),
        "smoke_validated_legacy_fallback": bool(smoke.get("checks", {}).get("legacy_fallback_available")),
        "all_default_screenshots_exist": all(screenshot_checks.values()),
        "frontend_boundary": frontend_boundary_check(source),
        "no_mojibake_source": no_forbidden_tokens(user_facing_source, [*FORBIDDEN_MOJIBAKE, "â€¢", "Î”"]),
        "no_smartpls_equivalence_claim": no_smartpls_equivalence(user_facing_source),
    }
    checks.update(screenshot_checks)
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
