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
    app = read_text("src/App.tsx")
    coach = read_text("src/domain/workflowCoach.ts")
    styles = read_text("src/styles.css")
    topbar = read_text("src/components/TopBar.tsx")
    docs = read_text("docs/V2_2_3_MODEL_WORKFLOW_CONTEXT.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v223_model_workflow_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_2_3_model_workflow_context"
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
    source_bundle = "\n".join([app, coach, styles, topbar])
    forbidden_mojibake = ["RÃƒÆ’Ã†â€™", "RÃƒÆ’Ã¢â‚¬Å¡", "ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â²"]
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.2.3": registry.get("current_stage") == target_stage,
        "registry contains v2.2.3 slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        "package version is 2.2.3": package.get("version") == "2.2.3",
        "package lock root version is 2.2.3": package_lock.get("version") == "2.2.3" and package_lock.get("packages", {}).get("", {}).get("version") == "2.2.3",
        "cargo workspace version is 2.2.3": 'version = "2.2.3"' in cargo,
        "quickpls lock versions are 2.2.3": all(f'name = "{name}"\nversion = "2.2.3"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.2.3": tauri.get("version") == "2.2.3",
        "roadmap expects v2.2.3": target_stage in roadmap,
        "topbar shows v2.2.3 model workflow": "v2.2.3 model workflow" in topbar,
        "artifact label is v2.2.3": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.2.3 scripts": all(key in package["scripts"] for key in [
            "qpls:v223:model-workflow-smoke",
            "qpls:v223:model-workflow-audit",
            "qpls:v223:model-workflow",
        ]),
        "model shell class is scoped": 'view === "models" ? " model-workflow-shell"' in app and 'className="model-workflow-band"' in app,
        "model branch preserves designer components": "<Explorer /><ModelCanvas /><Inspector />" in app,
        "model branch renders workflow context": "<WorkflowStrip /><WorkspaceCoach />" in app,
        "model coach branch exists": 'if (view === "models")' in coach and "model-needs-data" in coach and "model-ready-run" in coach,
        "model coach uses existing commands only": "quickpls:validate-diagram" not in coach and "quickpls:load-sample" not in coach,
        "css defines model workflow shell rows": ".workspace-shell.model-workflow-shell" in styles and ".model-workflow-band" in styles,
        "css keeps designer below band": ".model-workflow-shell .explorer" in styles and ".model-workflow-shell .model-canvas" in styles and ".model-workflow-shell .inspector" in styles,
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No statistical engine changes." in docs and "numerical fingerprint" in docs,
        "delivery status updated": target_stage in delivery and "v2.2.3 Model Workflow Context" in delivery,
        "development ledger updated": target_stage in ledger and "v2.2.3 Model Workflow Context" in ledger,
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
    (RESULTS / "v223_model_workflow_audit.json").write_text(json.dumps(report_payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(report_payload, indent=2))
        return 1
    print("v2.2.3 model workflow context audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
