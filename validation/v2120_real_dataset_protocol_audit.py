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
slices = load_json("validation/development_slices.json")
tauri = load_json("src-tauri/tauri.conf.json")
cargo = read("Cargo.toml")
roadmap = read("crates/qpls-core/src/roadmap.rs")
active = read("docs/V2_ACTIVE_MILESTONE.md")
protocol = read("docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md")
template = load_json("validation/templates/real_dataset_issue_register_template.json")
smoke_path = RESULTS / "v2120_real_dataset_protocol_smoke.json"
smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
registry_slice = next((item for item in slices.get("slices", []) if item.get("id") == "v2_12_0_real_dataset_review_protocol"), None)

required_scripts = [
    "qpls:v2120:real-dataset-protocol-smoke",
    "qpls:v2120:real-dataset-protocol-audit",
    "qpls:v2120:real-dataset-protocol",
]

template_issue_type = template.get("issues", [{}])[0].get("type", "")
checks = {
    "package_version_2_12_0": package.get("version") == "2.12.0",
    "package_lock_version_2_12_0": lock.get("version") == "2.12.0" and lock.get("packages", {}).get("", {}).get("version") == "2.12.0",
    "cargo_version_2_12_0": 'version = "2.12.0"' in cargo,
    "tauri_version_2_12_0": tauri.get("version") == "2.12.0",
    "artifact_label_v2120": "v2_12_0_real_dataset_review_protocol" in package["scripts"].get("qpls:release:artifacts", ""),
    "scripts_registered": all(name in package["scripts"] for name in required_scripts),
    "current_stage_v2120": slices.get("current_stage") == "v2_12_0_real_dataset_review_protocol",
    "roadmap_current_stage_v2120": "v2_12_0_real_dataset_review_protocol" in roadmap,
    "registry_slice_exists": registry_slice is not None,
    "registry_slice_passed": registry_slice is not None and all(gate.get("status") == "passed" for gate in registry_slice.get("gates", [])),
    "docs_exist": all((ROOT / path).exists() for path in [
        "docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md",
        "docs/RELEASE_NOTES_V2_12_0.md",
        "validation/templates/real_dataset_issue_register_template.json",
    ]),
    "active_milestone_updated": "Current completed checkpoint: `v2_12_0_real_dataset_review_protocol`" in active and "pending_real_dataset_feedback_or_next_grouped_frontend_pass" in active,
    "smoke_passed": smoke.get("passed") is True,
    "no_private_data_rule": all(token in protocol for token in [
        "raw private datasets",
        "private `.qpls` projects",
        "value-revealing screenshots",
        "Permitted artifacts",
    ]),
    "manual_review_covers_workspaces": all(token in protocol for token in [
        "Data Workspace",
        "Setup Workspace",
        "Results Workspace",
        "Report Workspace",
    ]),
    "statistical_gap_separation": "Statistical evidence gaps" in protocol and "must not be used to promote method scope" in protocol,
    "template_privacy_defaults_safe": template.get("privacy", {}).get("raw_data_committed") is False
        and template.get("privacy", {}).get("private_project_committed") is False
        and template.get("privacy", {}).get("value_revealing_screenshots_committed") is False,
    "template_issue_taxonomy": all(token in template_issue_type for token in [
        "ui_issue",
        "workflow_gap",
        "method_guidance_gap",
        "export_gap",
        "statistical_evidence_gap",
        "cannot_reproduce_without_private_data",
    ]),
    "no_private_file_patterns_in_protocol": not any(token in protocol.lower() for token in [
        "c:\\users\\",
        "d:\\users\\",
        ".sav attached",
        "actual respondent",
    ]),
    "frontend_product_boundary": "No statistical engines changed." in read("docs/RELEASE_NOTES_V2_12_0.md"),
}

failed = [name for name, passed in checks.items() if not passed]
report = {
    "milestone": "v2_12_0_real_dataset_review_protocol",
    "passed": not failed,
    "checks": checks,
    "failed": failed,
}

(RESULTS / "v2120_real_dataset_protocol_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
if failed:
    raise SystemExit(json.dumps(report, indent=2))
print("v2.12 real dataset protocol audit passed")
