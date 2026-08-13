import json
from datetime import UTC, datetime
from pathlib import Path

from lib.v2_ui_audit import (
    FORBIDDEN_MOJIBAKE,
    ROOT,
    RESULTS,
    frontend_boundary_check,
    no_forbidden_tokens,
    no_smartpls_equivalence,
    read_json,
    read_text,
    shared_v2_metadata_checks,
    write_result,
)

OUTPUT = "v2300_reportability_assistant_audit.json"
MILESTONE = "v2_30_0_interpretation_reportability_assistant"
VERSION = "2.30.0"
LABEL = "v2.30 reportability assistant"


def main() -> int:
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    run_history = read_text("src/components/RunHistory.tsx")
    ui = read_text("src/components/Ui.tsx")
    css = read_text("src/styles.css")
    interpretation = read_text("src/domain/resultInterpretation.ts")
    smoke_path = RESULTS / "v2300_reportability_assistant_smoke.json"
    smoke_payload = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}
    current = next((item for item in registry["slices"] if item["id"] == MILESTONE), None)
    source = "\n".join([run_history, ui, css, interpretation, topbar])

    checks = {
        **shared_v2_metadata_checks(
            version=VERSION,
            target_stage=MILESTONE,
            expected_label=LABEL,
            package=package,
            package_lock=package_lock,
            registry=registry,
            cargo=cargo,
            cargo_lock=cargo_lock,
            tauri=tauri,
            roadmap=roadmap,
            topbar=topbar,
        ),
        "scripts_registered": all(key in package.get("scripts", {}) for key in [
            "qpls:v2300:reportability-assistant-smoke",
            "qpls:v2300:reportability-assistant-audit",
            "qpls:v2300:reportability-assistant",
        ]),
        "registry_gates_passed": bool(current and all(gate.get("status") == "passed" for gate in current.get("gates", []))),
        "reportability_assistant_component_present": "function ReportabilityAssistantPanel" in run_history and 'data-v230-reportability-assistant="true"' in run_history,
        "reportability_item_detail_present": "What the value says" in run_history and "Why it matters" in run_history and "What to inspect next" in run_history,
        "copy_report_snippets_present": "Copy report snippets" in run_history and "reportSnippets" in run_history,
        "canonical_checklist_items_present": all(token in run_history for token in [
            "indicator_reliability",
            "internal_consistency",
            "convergent_validity",
            "discriminant_validity",
            "collinearity",
            "structural_paths",
            "r_squared",
            "f_squared",
            "prediction",
            "conditional_effects",
            "inference",
            "warnings",
        ]),
        "threshold_guidance_caveat_present": "Threshold colors are guidance, not universal pass/fail rules" in run_history and "Threshold colors are methodological guidance" in ui,
        "checklist_marker_present": 'data-v230-reportability-checklist="true"' in ui,
        "status_lanes_present": all(token in run_history for token in ["Must address", "Review before reporting", "Ready evidence", "Unavailable / not applicable"]),
        "report_wording_uses_run_values": "interpretation.reportParagraphs" in run_history and "reportSentence" in run_history,
        "css_contract_present": all(token in css for token in [
            ".reportability-assistant",
            ".reportability-assistant-grid",
            ".reportability-lane.issue",
            ".reportability-assistant-item",
            ".reportability-report-snippets",
        ]),
        "interpretation_engine_remains_deterministic": "buildResultInterpretation" in interpretation and "navigator" not in interpretation and "fetch(" not in interpretation,
        "no_tauri_or_backend_invocation_added": "invoke(" not in run_history and "invoke(" not in interpretation,
        "no_mojibake_source": no_forbidden_tokens(source, FORBIDDEN_MOJIBAKE),
        "no_smartpls_equivalence_claim": no_smartpls_equivalence(source),
        "frontend_boundary": frontend_boundary_check(source),
        "docs_present": (ROOT / "docs" / "V2_30_0_INTERPRETATION_REPORTABILITY_ASSISTANT.md").exists(),
        "delivery_docs_updated": MILESTONE in read_text("docs/DELIVERY_STATUS.md") and "v2.30.0 - Interpretation And Reportability Assistant" in read_text("docs/DEVELOPMENT_LEDGER.md"),
        "smoke_passed": smoke_payload.get("passed") is True,
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
