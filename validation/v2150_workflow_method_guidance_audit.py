import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
RESULTS.mkdir(parents=True, exist_ok=True)


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def load_json(path: str):
    return json.loads(read(path))


package = load_json("package.json")
lock = load_json("package-lock.json")
registry = load_json("validation/development_slices.json")
tauri = load_json("src-tauri/tauri.conf.json")
cargo = read("Cargo.toml")
cargo_lock = read("Cargo.lock")
roadmap = read("crates/qpls-core/src/roadmap.rs")
active = read("docs/V2_ACTIVE_MILESTONE.md")
milestone_doc = read("docs/V2_15_0_WORKFLOW_METHOD_GUIDANCE_TRIAGE_PASS.md")
release_notes = read("docs/RELEASE_NOTES_V2_15_0.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")
smoke_path = RESULTS / "v2150_workflow_method_guidance_smoke.json"
smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
slice_ = next((item for item in registry.get("slices", []) if item.get("id") == "v2_15_0_workflow_method_guidance_triage_pass"), None)

source_files = [
    "src/domain/methodApplicability.ts",
    "src/components/DataWorkspace.tsx",
    "src/components/Explorer.tsx",
    "src/components/AnalysisCatalog.tsx",
    "src/components/TopBar.tsx",
    "src/styles.css",
]
combined_source = "\n".join(read(path) for path in source_files)
combined_docs = "\n".join([active, milestone_doc, release_notes, delivery, ledger])
scripts = package.get("scripts", {})
required_scripts = [
    "qpls:v2150:workflow-method-guidance-smoke",
    "qpls:v2150:workflow-method-guidance-audit",
    "qpls:v2150:workflow-method-guidance",
]
quickpls_lock_versions = [
    block
    for block in cargo_lock.split("[[package]]")
    if any(f'name = "{name}"' in block for name in [
        "qpls-assessment",
        "qpls-cli",
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-project",
        "qpls-resampling",
        "qpls-runner",
        "quickpls-desktop",
    ])
]

checks = {
    "package_version_2_15_0": package.get("version") == "2.15.0",
    "package_lock_version_2_15_0": lock.get("version") == "2.15.0" and lock.get("packages", {}).get("", {}).get("version") == "2.15.0",
    "cargo_version_2_15_0": 'version = "2.15.0"' in cargo,
    "quickpls_cargo_lock_versions_2_15_0": quickpls_lock_versions and all('version = "2.15.0"' in block for block in quickpls_lock_versions),
    "tauri_version_2_15_0": tauri.get("version") == "2.15.0",
    "artifact_label_v2150": "v2_15_0_workflow_method_guidance_triage_pass" in scripts.get("qpls:release:artifacts", ""),
    "scripts_registered": all(name in scripts for name in required_scripts),
    "current_stage_v2150": registry.get("current_stage") == "v2_15_0_workflow_method_guidance_triage_pass",
    "roadmap_current_stage_v2150": "v2_15_0_workflow_method_guidance_triage_pass" in roadmap,
    "registry_slice_exists": slice_ is not None,
    "registry_slice_passed": slice_ is not None and all(gate.get("status") == "passed" for gate in slice_.get("gates", [])),
    "docs_exist": all((ROOT / path).exists() for path in [
        "docs/V2_15_0_WORKFLOW_METHOD_GUIDANCE_TRIAGE_PASS.md",
        "docs/RELEASE_NOTES_V2_15_0.md",
    ]),
    "active_tracker_updated": (
        "Current active checkpoint: `v2_15_0_workflow_method_guidance_triage_pass`" in active
        or "Current completed checkpoint: `v2_15_0_workflow_method_guidance_triage_pass`" in active
    ),
    "active_tracker_has_lightweight_policy": "For pure UI milestones, run `npm run build`, the targeted smoke/audit, and the final gate." in active,
    "smoke_passed": smoke.get("passed") is True,
    "ui_markers_present": all(token in combined_source for token in [
        "data-workflow-method-guidance-triage",
        "data-data-guidance-next-action",
        "data-selected-method-next-action",
        "data-method-next-action",
        "data-topbar-guidance-count",
    ]),
    "next_action_copy_present": all(token in combined_source for token in [
        "Recommended next move",
        "If you expected another method",
        "Guidance is based on construct modes",
        "More methods in Setup",
    ]),
    "no_r_squared_mojibake": "RÂ²" not in combined_source and "RÃ" not in combined_source,
    "frontend_only_boundary": "No statistical engines" in milestone_doc
    and "does not change QuickPLS statistical engines" in release_notes
    and not any(token in combined_source for token in ["F_ml =", "Sigma(theta)", "optimizer reduction"]),
    "no_smartpls_equivalence_claim": not any(
        token in (combined_docs + combined_source).lower()
        for token in ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]
    ),
}

failed = [name for name, passed in checks.items() if not passed]
report = {
    "milestone": "v2_15_0_workflow_method_guidance_triage_pass",
    "passed": not failed,
    "checks": checks,
    "failed": failed,
}

(RESULTS / "v2150_workflow_method_guidance_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
if failed:
    raise SystemExit(json.dumps(report, indent=2))
print("v2.15 workflow method guidance audit passed")
