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

OUTPUT = "v2410_mockup_visual_parity_audit.json"
MILESTONE = "v2_41_0_full_mockup_screen_parity_pass"
MANIFEST = Path("validation/mockups/v2410_mockup_manifest.json")
SMOKE_OUTPUT = RESULTS / "v2410_mockup_visual_parity_smoke.json"
MANIFEST_OUTPUT = RESULTS / "v2410_mockup_manifest_audit.json"


def load_result(path: Path) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def slug(value: str) -> str:
    cleaned = []
    previous_dash = False
    for char in value.lower():
        if char.isalnum():
            cleaned.append(char)
            previous_dash = False
        elif not previous_dash:
            cleaned.append("-")
            previous_dash = True
    return "".join(cleaned).strip("-")


def main() -> int:
    package = read_json("package.json")
    registry = read_json("validation/development_slices.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    manifest = read_json(str(MANIFEST))
    prototype = read_text("src/v2/NativePrototypeApp.tsx")
    adapters = read_text("src/v2/nativePrototypeAdapters.ts")
    css = read_text("src/v2/nativePrototype.css")
    doc = read_text("docs/V2_41_0_FULL_MOCKUP_SCREEN_PARITY_PASS.md")
    active = read_text("docs/V2_ACTIVE_MILESTONE.md")
    backlog = read_text("docs/V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md")
    smoke_script = read_text("validation/v2410_mockup_visual_parity_smoke.mjs")
    manifest_audit = read_text("validation/v2410_mockup_manifest_audit.py")
    this_audit = read_text("validation/v2410_mockup_visual_parity_audit.py")
    smoke = load_result(SMOKE_OUTPUT)
    manifest_result = load_result(MANIFEST_OUTPUT)
    current = next((item for item in registry["slices"] if item["id"] == MILESTONE), None)
    source = "\n".join([
        prototype,
        adapters,
        css,
        doc,
        active,
        backlog,
        smoke_script,
        manifest_audit,
        this_audit,
    ])
    user_facing_source = "\n".join([
        prototype,
        adapters,
        css,
        doc,
        active,
        backlog,
    ])

    scripts = package.get("scripts", {})
    screenshot_dir = RESULTS / "screens" / "v2410" / "mockup-parity"
    screen_entries = manifest.get("states", [])
    screenshot_checks = {
        f"screenshot_{entry.get('id')}": (screenshot_dir / f"{slug(entry.get('id', ''))}.png").exists()
        for entry in screen_entries
    }
    required_main_states = {
        "home",
        "data",
        "model",
        "setup",
        "run",
        "results",
        "report",
        "trust",
        "settings",
        "focus_diagram",
    }
    required_dialogs = {
        "new_project_dialog",
        "import_data_dialog",
        "calculation_setup_dialog",
        "method_scope_dialog",
        "export_options_dialog",
    }
    state_ids = {entry.get("id") for entry in screen_entries}

    checks = {
        "current_stage_registered": registry.get("current_stage") == MILESTONE,
        "registry_slice_validated": bool(current and current.get("status") == "validated"),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "roadmap_current_stage_updated": MILESTONE in roadmap,
        "package_scripts_registered": all(key in scripts for key in [
            "qpls:v2410:mockup-manifest-audit",
            "qpls:v2410:mockup-parity-smoke",
            "qpls:v2410:mockup-parity-audit",
            "qpls:v2410:mockup-parity",
        ]),
        "manifest_file_present": bool(screen_entries),
        "manifest_maps_required_main_states": required_main_states.issubset(state_ids),
        "manifest_maps_required_dialogs": required_dialogs.issubset(state_ids),
        "manifest_audit_passed": manifest_result.get("passed") is True,
        "smoke_passed": smoke.get("passed") is True,
        "all_state_screenshots_exist": all(screenshot_checks.values()) if screenshot_checks else False,
        "parity_route_adapter_present": "mockup_parity" in adapters and "fallbackNativePrototypeData" in adapters,
        "root_parity_marker_present": 'data-v241-mockup-parity="true"' in prototype,
        "css_parity_marker_present": "[data-v241-mockup-parity=\"true\"]" in css,
        "ribbon_contract_present": "Save" in prototype and "Add Latent" in prototype and "Connect Path" in prototype,
        "model_workbench_contract_present": all(token in prototype for token in [
            "SEM Explorer",
            "Object Inspector",
            "Model Issues",
            "Diagram Advisor",
            "Check Diagram",
        ]),
        "diagram_result_contract_present": all(token in prototype for token in [
            "np-measurement-line",
            "np-structural-line",
            "np-covariance-line",
            'R{"\\u00b2"} =',
        ]),
        "dialog_contract_present": all(token in prototype for token in [
            "New Project",
            "Import Data",
            "Calculation Setup",
            "Export Options",
            "Method Scope",
            "Help and Shortcuts",
        ]),
        "extra_feature_backlog_exists": "Deferred Or Hidden During Mockup Parity" in backlog,
        "frontend_boundary": frontend_boundary_check(source),
        "no_mojibake_source": no_forbidden_tokens(user_facing_source, FORBIDDEN_MOJIBAKE),
        "no_smartpls_equivalence_claim": no_smartpls_equivalence(user_facing_source),
        "no_legacy_dashboard_marker_in_parity_css": "large dashboard" not in css.lower(),
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
