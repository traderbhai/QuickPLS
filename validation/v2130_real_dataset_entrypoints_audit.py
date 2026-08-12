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
trust = read("src/components/TrustCenterWorkspace.tsx")
settings = read("src/components/SettingsWorkspace.tsx")
home = read("src/components/OnboardingWorkspace.tsx")
release_notes = read("docs/RELEASE_NOTES_V2_13_0.md")
smoke_path = RESULTS / "v2130_real_dataset_entrypoints_smoke.json"
smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
registry_slice = next((item for item in slices.get("slices", []) if item.get("id") == "v2_13_0_real_dataset_protocol_entrypoints"), None)

required_scripts = [
    "qpls:v2130:real-dataset-entrypoints-smoke",
    "qpls:v2130:real-dataset-entrypoints-audit",
    "qpls:v2130:real-dataset-entrypoints",
]

combined_source = "\n".join([trust, settings, home])
checks = {
    "package_version_2_13_0": package.get("version") == "2.13.0",
    "package_lock_version_2_13_0": lock.get("version") == "2.13.0" and lock.get("packages", {}).get("", {}).get("version") == "2.13.0",
    "cargo_version_2_13_0": 'version = "2.13.0"' in cargo,
    "tauri_version_2_13_0": tauri.get("version") == "2.13.0",
    "artifact_label_v2130": "v2_13_0_real_dataset_protocol_entrypoints" in package["scripts"].get("qpls:release:artifacts", ""),
    "scripts_registered": all(name in package["scripts"] for name in required_scripts),
    "current_stage_v2130": slices.get("current_stage") == "v2_13_0_real_dataset_protocol_entrypoints",
    "roadmap_current_stage_v2130": "v2_13_0_real_dataset_protocol_entrypoints" in roadmap,
    "registry_slice_exists": registry_slice is not None,
    "registry_slice_passed": registry_slice is not None and all(gate.get("status") == "passed" for gate in registry_slice.get("gates", [])),
    "docs_exist": all((ROOT / path).exists() for path in [
        "docs/V2_13_0_REAL_DATASET_PROTOCOL_ENTRYPOINTS.md",
        "docs/RELEASE_NOTES_V2_13_0.md",
        "docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md",
        "validation/templates/real_dataset_issue_register_template.json",
    ]),
    "active_milestone_updated": "Current completed checkpoint: `v2_13_0_real_dataset_protocol_entrypoints`" in active,
    "smoke_passed": smoke.get("passed") is True,
    "trust_center_entrypoint": "data-real-dataset-protocol-entrypoint=\"trust-center\"" in trust,
    "settings_entrypoint": "data-real-dataset-protocol-entrypoint=\"settings\"" in settings,
    "home_entrypoint": "Reviewing a private dataset?" in home and 'start("trust")' in home,
    "protocol_artifacts_referenced": "docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md" in trust and "validation/templates/real_dataset_issue_register_template.json" in trust,
    "privacy_copy_present": all(token in combined_source for token in [
        "raw files",
        "private projects",
        "value-revealing",
        "repository",
    ]),
    "frontend_product_boundary": "No statistical engines changed." in release_notes and "No numerical fingerprints changed." in release_notes,
    "no_backend_formula_terms": not any(token in combined_source for token in ["Sigma(theta)", "F_ml =", "bootstrap replicate calculation changed"]),
}

failed = [name for name, passed in checks.items() if not passed]
report = {
    "milestone": "v2_13_0_real_dataset_protocol_entrypoints",
    "passed": not failed,
    "checks": checks,
    "failed": failed,
}

(RESULTS / "v2130_real_dataset_entrypoints_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
if failed:
    raise SystemExit(json.dumps(report, indent=2))
print("v2.13 real dataset protocol entrypoints audit passed")
