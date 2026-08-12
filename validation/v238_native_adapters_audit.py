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

OUTPUT = "v2380_native_frontend_backend_adapters_audit.json"
MILESTONE = "v2_38_0_native_frontend_backend_adapters"


def main() -> int:
    package = read_json("package.json")
    registry = read_json("validation/development_slices.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    app = read_text("src/App.tsx")
    prototype = read_text("src/v2/NativePrototypeApp.tsx")
    adapter = read_text("src/v2/nativePrototypeAdapters.ts")
    data = read_text("src/v2/nativePrototypeData.ts")
    doc = read_text("docs/V2_38_0_NATIVE_FRONTEND_BACKEND_ADAPTERS.md")
    smoke_path = RESULTS / "v2380_native_frontend_backend_adapters_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}
    current = next((item for item in registry["slices"] if item["id"] == MILESTONE), None)
    source = "\n".join([app, prototype, adapter, data, doc])

    checks = {
        "current_stage_registered": registry.get("current_stage") == MILESTONE,
        "registry_slice_validated": bool(current and current.get("status") == "validated"),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "roadmap_current_stage_updated": MILESTONE in roadmap,
        "package_scripts_registered": all(key in package.get("scripts", {}) for key in [
            "qpls:v238:native-adapters-smoke",
            "qpls:v238:native-adapters-audit",
            "qpls:v238:native-adapters",
        ]),
        "feature_flag_still_isolated": "native_prototype" in app and "NativePrototypeApp" in app,
        "adapter_file_present": (ROOT / "src/v2/nativePrototypeAdapters.ts").exists(),
        "adapter_reads_store": "useWorkspace" in adapter and "state.dataset" in adapter and "state.nodes" in adapter and "state.runs" in adapter,
        "adapter_has_fallback_boundary": "fallbackNativePrototypeData" in adapter and "adapterSource" in data,
        "prototype_uses_adapter": "useNativePrototypeAdapter" in prototype and "data-v238-adapter" in prototype,
        "backend_untouched_by_adapter": not any(token in source for token in [
            "crates/qpls-estimation",
            "crates/qpls-assessment",
            "crates/qpls-runner",
            "result.schemas",
        ]),
        "frontend_boundary": frontend_boundary_check(source),
        "no_mojibake_source": no_forbidden_tokens(source, FORBIDDEN_MOJIBAKE),
        "no_smartpls_equivalence_claim": no_smartpls_equivalence(source),
        "smoke_passed": smoke.get("passed") is True,
        "smoke_store_adapter_active": smoke.get("checks", {}).get("store_adapter_active") is True,
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
