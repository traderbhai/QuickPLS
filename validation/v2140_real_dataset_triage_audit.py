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
roadmap = read("crates/qpls-core/src/roadmap.rs")
active = read("docs/V2_ACTIVE_MILESTONE.md")
milestone_doc = read("docs/V2_14_0_REAL_DATASET_FEEDBACK_TRIAGE.md")
release_notes = read("docs/RELEASE_NOTES_V2_14_0.md")
template = load_json("validation/templates/real_dataset_feedback_triage_template.json")
smoke_path = RESULTS / "v2140_real_dataset_triage_smoke.json"
backlog_path = RESULTS / "v2140_real_dataset_triage_backlog.json"
smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
backlog = json.loads(backlog_path.read_text(encoding="utf-8")) if backlog_path.exists() else {}
slice_ = next((item for item in registry.get("slices", []) if item.get("id") == "v2_14_0_real_dataset_feedback_triage"), None)

scripts = package.get("scripts", {})
required_scripts = [
    "qpls:v2140:real-dataset-triage-smoke",
    "qpls:v2140:real-dataset-triage-audit",
    "qpls:v2140:real-dataset-triage",
]
combined = "\n".join([
    active,
    milestone_doc,
    release_notes,
])

checks = {
    "package_version_2_14_0": package.get("version") == "2.14.0",
    "package_lock_version_2_14_0": lock.get("version") == "2.14.0" and lock.get("packages", {}).get("", {}).get("version") == "2.14.0",
    "cargo_version_2_14_0": 'version = "2.14.0"' in cargo,
    "tauri_version_2_14_0": tauri.get("version") == "2.14.0",
    "artifact_label_v2140": "v2_14_0_real_dataset_feedback_triage" in scripts.get("qpls:release:artifacts", ""),
    "scripts_registered": all(name in scripts for name in required_scripts),
    "current_stage_v2140": registry.get("current_stage") == "v2_14_0_real_dataset_feedback_triage",
    "roadmap_current_stage_v2140": "v2_14_0_real_dataset_feedback_triage" in roadmap,
    "registry_slice_exists": slice_ is not None,
    "registry_slice_passed": slice_ is not None and all(gate.get("status") == "passed" for gate in slice_.get("gates", [])),
    "docs_exist": all((ROOT / path).exists() for path in [
        "docs/V2_14_0_REAL_DATASET_FEEDBACK_TRIAGE.md",
        "docs/RELEASE_NOTES_V2_14_0.md",
        "validation/templates/real_dataset_feedback_triage_template.json",
    ]),
    "active_tracker_updated": "Current completed checkpoint: `v2_14_0_real_dataset_feedback_triage`" in active,
    "active_tracker_has_next_policy": "Next Active Milestone" in active and "next_grouped_frontend_pass_from_triage" in active,
    "template_blocks_private_data": template.get("privacy_rules", {}).get("raw_data_committed") is False
    and template.get("privacy_rules", {}).get("private_qpls_committed") is False
    and template.get("privacy_rules", {}).get("value_revealing_screenshots_committed") is False,
    "template_has_triage_categories": template.get("findings", [{}])[0].get("category") in {
        "launch_blocker",
        "workflow_friction",
        "visual_polish",
        "method_guidance",
        "reporting_export",
        "statistical_evidence_gap",
    },
    "smoke_passed": smoke.get("passed") is True,
    "backlog_generated": backlog.get("milestone") == "v2_14_0_real_dataset_feedback_triage",
    "backlog_has_triage_lanes": len(backlog.get("triage_lanes", [])) >= 4,
    "backlog_preserves_no_private_persistence": all(value is False for value in backlog.get("privacy_boundary", {}).values()),
    "frontend_product_boundary": "No statistical engines changed." in release_notes
    and "No numerical fingerprints changed." in release_notes
    and "No estimator code changes." in milestone_doc,
    "no_smartpls_equivalence_claim": not any(
        token in combined.lower()
        for token in ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]
    ),
}

failed = [name for name, passed in checks.items() if not passed]
report = {
    "milestone": "v2_14_0_real_dataset_feedback_triage",
    "passed": not failed,
    "checks": checks,
    "failed": failed,
}

(RESULTS / "v2140_real_dataset_triage_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
if failed:
    raise SystemExit(json.dumps(report, indent=2))
print("v2.14 real dataset feedback triage audit passed")
