import json
from datetime import UTC, datetime

from lib.v2_ui_audit import (
    FORBIDDEN_MOJIBAKE,
    ROOT,
    RESULTS,
    frontend_boundary_check,
    no_forbidden_tokens,
    no_smartpls_equivalence,
    read_json,
    read_text,
    write_result,
)

OUTPUT = "v2370_native_frontend_prototype_audit.json"
MILESTONE = "v2_37_0_native_frontend_prototype_shell"


def main() -> int:
    package = read_json("package.json")
    registry = read_json("validation/development_slices.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    app = read_text("src/App.tsx")
    prototype = read_text("src/v2/NativePrototypeApp.tsx")
    data = read_text("src/v2/nativePrototypeData.ts")
    css = read_text("src/v2/nativePrototype.css")
    doc = read_text("docs/V2_37_0_NATIVE_FRONTEND_PROTOTYPE_SHELL.md")
    smoke_path = RESULTS / "v2370_native_frontend_prototype_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}
    source = "\n".join([app, prototype, data, css, doc])
    current = next((item for item in registry["slices"] if item["id"] == MILESTONE), None)

    checks = {
        "current_stage_registered": registry.get("current_stage") == MILESTONE,
        "registry_slice_validated": bool(current and current.get("status") == "validated"),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "roadmap_current_stage_updated": MILESTONE in roadmap,
        "package_scripts_registered": all(key in package.get("scripts", {}) for key in [
            "qpls:v237:native-prototype-smoke",
            "qpls:v237:native-prototype-audit",
            "qpls:v237:native-prototype",
        ]),
        "feature_flag_isolated": "native_prototype" in app and "NativePrototypeApp" in app,
        "prototype_files_present": all((ROOT / path).exists() for path in [
            "src/v2/NativePrototypeApp.tsx",
            "src/v2/nativePrototypeData.ts",
            "src/v2/nativePrototype.css",
        ]),
        "all_screens_present": all(token in prototype for token in [
            'data-v237-screen="home"',
            'data-v237-screen="data"',
            'data-v237-screen="model"',
            'data-v237-screen="setup"',
            'data-v237-screen="run"',
            'data-v237-screen="results"',
            'data-v237-screen="report"',
            'data-v237-screen="trust"',
            'data-v237-screen="settings"',
        ]),
        "all_dialogs_present": all(token in prototype for token in [
            "new_project",
            "sample_gallery",
            "import_data",
            "calculation_setup",
            "method_scope",
            "export_options",
            "help_shortcuts",
        ]),
        "menu_and_rail_contract_present": all(token in prototype for token in [
            "File", "Edit", "Data", "Model", "Calculate", "Results", "Report", "View", "Tools", "Window", "Help",
            "Trust Center", "Settings",
        ]),
        "dummy_data_boundary_present": "dummy data" in doc.lower() and "backend wiring" in doc.lower(),
        "sem_designer_boundary_present": "SEM designer core" in doc,
        "frontend_boundary": frontend_boundary_check(source),
        "no_mojibake_source": no_forbidden_tokens(source, FORBIDDEN_MOJIBAKE),
        "no_smartpls_equivalence_claim": no_smartpls_equivalence(source),
        "smoke_passed": smoke.get("passed") is True,
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
