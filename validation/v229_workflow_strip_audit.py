import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
RESULTS.mkdir(parents=True, exist_ok=True)


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8-sig")


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
    workflow_strip = read_text("src/components/WorkflowStrip.tsx")
    store = read_text("src/store.ts")
    topbar = read_text("src/components/TopBar.tsx")
    docs = read_text("docs/V2_2_9_WORKFLOW_STRIP_CONTEXT_ALIGNMENT.md")
    delivery = read_text("docs/DELIVERY_STATUS.md")
    ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
    smoke_path = RESULTS / "v229_workflow_strip_smoke.json"
    smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

    target_stage = "v2_2_9_workflow_strip_context_alignment"
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
    source_bundle = "\n".join([workflow_strip, store, topbar])
    forbidden_equivalence = ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]
    forbidden_mojibake = ["RÃƒÆ’", "RÃƒâ€šÃ‚Â²", "ÃƒÆ’Ã‚Â¢", "Ãƒâ€š "]

    checks = {
        "smoke report passed": bool(smoke.get("passed")),
        "current stage is v2.2.9": registry.get("current_stage") == target_stage,
        "registry contains v2.2.9 slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        "package version is 2.2.9": package.get("version") == "2.2.9",
        "package lock root version is 2.2.9": package_lock.get("version") == "2.2.9" and package_lock.get("packages", {}).get("", {}).get("version") == "2.2.9",
        "cargo workspace version is 2.2.9": 'version = "2.2.9"' in cargo,
        "quickpls lock versions are 2.2.9": all(f'name = "{name}"\nversion = "2.2.9"' in cargo_lock for name in quickpls_packages),
        "tauri version is 2.2.9": tauri.get("version") == "2.2.9",
        "roadmap expects v2.2.9": target_stage in roadmap,
        "topbar shows v2.2.9 workflow strip alignment": "v2.2.9 workflow strip alignment" in topbar,
        "artifact label is v2.2.9": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
        "package exposes v2.2.9 scripts": all(key in package["scripts"] for key in [
            "qpls:v229:workflow-strip-smoke",
            "qpls:v229:workflow-strip-audit",
            "qpls:v229:workflow-strip",
        ]),
        "workflow strip exposes metadata": all(token in workflow_strip for token in [
            "data-workflow-view",
            "data-workflow-label",
            "data-workflow-action",
            "data-workflow-detail",
            "data-workflow-state",
        ]),
        "workflow strip routes with context": "setView(targetView, {" in workflow_strip and "coachId: \"workflow-strip\"" in workflow_strip and "actionLabel: `Workflow: ${label}`" in workflow_strip,
        "current step avoids context": "if (targetView === view)" in workflow_strip and "setView(targetView);" in workflow_strip,
        "store destination context contract remains available": "workflowDestinationContext" in store and "timestamp: Date.now()" in store,
        "docs declare frontend product only": "frontend/product-only" in docs,
        "docs declare no numerical changes": "No statistical engine changes." in docs and "numerical fingerprint" in docs,
        "delivery status updated": target_stage in delivery and "v2.2.9 Workflow Strip Context Alignment" in delivery,
        "development ledger updated": target_stage in ledger and "v2.2.9 Workflow Strip Context Alignment" in ledger,
        "target source has no SmartPLS equivalence claim": not any(phrase in source_bundle.lower() for phrase in forbidden_equivalence),
        "target source has no mojibake": not any(token in source_bundle for token in forbidden_mojibake),
    }

    failed = [name for name, passed in checks.items() if not passed]
    payload = {
        "milestone": target_stage,
        "passed": not failed,
        "checks": checks,
        "failed": failed,
    }
    (RESULTS / "v229_workflow_strip_audit.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
    if failed:
        print(json.dumps(payload, indent=2))
        return 1
    print("v2.2.9 workflow strip context alignment audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
