import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
RESULTS.mkdir(parents=True, exist_ok=True)


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def read_json(relative: str):
    return json.loads(read_text(relative))


def main() -> int:
    package = read_json("package.json")
    package_lock = read_json("package-lock.json")
    registry = read_json("validation/development_slices.json")
    cargo = read_text("Cargo.toml")
    cargo_lock = read_text("Cargo.lock")
    tauri = read_json("src-tauri/tauri.conf.json")
    roadmap = read_text("crates/qpls-core/src/roadmap.rs")
    topbar = read_text("src/components/TopBar.tsx")
    progress = read_text("src/domain/workflowProgress.ts")
    strip = read_text("src/components/WorkflowStrip.tsx")
    styles = read_text("src/styles.css")
    docs = read_text("docs/V2_2_2_WORKFLOW_STEP_CLARITY.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v222_workflow_step_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_2_2_workflow_step_clarity"
    quickpls_packages = [
        "qpls-assessment",
        "qpls-cli",
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-project",
        "qpls-resampling",
        "qpls-runner",
        "quickpls-desktop",
    ]
    state_tokens = ["complete", "current", "next", "blocked", "ready", "pending"]
    source_bundle = "\n".join([topbar, progress, strip, styles])
    forbidden_mojibake = ["RÃƒÆ’", "RÃƒâ€š", "Ãƒâ€šÃ‚Â²"]
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.2.2": registry.get("current_stage") == target_stage,
        "registry contains v2.2.2 slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        "package version is 2.2.2": package.get("version") == "2.2.2",
        "package lock root version is 2.2.2": package_lock.get("version") == "2.2.2" and package_lock.get("packages", {}).get("", {}).get("version") == "2.2.2",
        "cargo workspace version is 2.2.2": 'version = "2.2.2"' in cargo,
        "quickpls lock versions are 2.2.2": all(f'name = "{name}"\nversion = "2.2.2"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.2.2": tauri.get("version") == "2.2.2",
        "roadmap expects v2.2.2": target_stage in roadmap,
        "topbar shows v2.2.2 workflow steps": "v2.2.2 workflow steps" in topbar,
        "artifact label is v2.2.2": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.2.2 scripts": all(key in package["scripts"] for key in [
            "qpls:v222:workflow-step-smoke",
            "qpls:v222:workflow-step-audit",
            "qpls:v222:workflow-step",
        ]),
        "workflow progress domain exists": "export function workflowProgress" in progress and "WorkflowStepState" in progress,
        "workflow progress covers all states": all(f'"{token}"' in progress for token in state_tokens),
        "workflow progress uses readiness": "analysisReadiness" in progress and "readiness.canRun" in progress,
        "strip uses workflow progress": "workflowProgress" in strip and "workflowStepStatusSummary" in strip,
        "strip renders stable state marker": "data-workflow-state" in strip and "step.state" in strip,
        "strip renders accessible reasons": "aria-label" in strip and "title={step.detail}" in strip,
        "strip keeps buttons navigable": "disabled" not in strip,
        "css defines workflow state classes": all(f".workflow-step.{token}" in styles for token in state_tokens),
        "css uses six-step desktop grid": "grid-template-columns: repeat(6" in styles and "overflow-x: auto" not in styles[styles.find(".workflow-strip"):styles.find(".workspace-page")],
        "docs declare frontend product only": "frontend-only" in docs,
        "docs declare no numerical changes": "No statistical engine changes." in docs and "numerical fingerprint" in docs,
        "delivery status updated": target_stage in delivery and "v2.2.2 Workflow Step Clarity" in delivery,
        "development ledger updated": target_stage in ledger and "v2.2.2 Workflow Step Clarity" in ledger,
        "target source has no SmartPLS equivalence claim": not any(phrase in source_bundle.lower() for phrase in forbidden_equivalence),
        "target source has no mojibake": not any(token in source_bundle for token in forbidden_mojibake),
    }

    failed = [name for name, passed in checks.items() if not passed]
    report_payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v222_workflow_step_audit.json").write_text(json.dumps(report_payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report_payload, indent=2))
        return 1
    print("v2.2.2 workflow step clarity audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
