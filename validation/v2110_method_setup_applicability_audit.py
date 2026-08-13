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
smoke_path = RESULTS / "v2110_method_setup_applicability_smoke.json"
smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}
registry_slice = next((item for item in slices.get("slices", []) if item.get("id") == "v2_11_0_method_applicability_setup_polish"), None)

source_files = [
    "src/domain/methodApplicability.ts",
    "src/domain/methodApplicability.test.ts",
    "src/components/AnalysisCatalog.tsx",
    "src/components/TopBar.tsx",
    "src/components/DataWorkspace.tsx",
    "src/components/Explorer.tsx",
    "src/styles.css",
]
combined_source = "\n".join(read(path) for path in source_files)

checks = {
    "package_version_2_11_0": package.get("version") == "2.11.0",
    "package_lock_version_2_11_0": lock.get("version") == "2.11.0" and lock.get("packages", {}).get("", {}).get("version") == "2.11.0",
    "cargo_version_2_11_0": 'version = "2.11.0"' in cargo,
    "tauri_version_2_11_0": tauri.get("version") == "2.11.0",
    "artifact_label_v2110": "v2_11_0_method_applicability_setup_polish" in package["scripts"].get("qpls:release:artifacts", ""),
    "scripts_registered": all(name in package["scripts"] for name in [
        "qpls:v2110:method-setup-smoke",
        "qpls:v2110:method-setup-audit",
        "qpls:v2110:method-setup",
    ]),
    "current_stage_v2110": slices.get("current_stage") == "v2_11_0_method_applicability_setup_polish",
    "roadmap_current_stage_v2110": "v2_11_0_method_applicability_setup_polish" in roadmap,
    "registry_slice_exists": registry_slice is not None,
    "registry_slice_passed": registry_slice is not None and all(gate.get("status") == "passed" for gate in registry_slice.get("gates", [])),
    "docs_exist": all((ROOT / path).exists() for path in [
        "docs/V2_11_0_METHOD_APPLICABILITY_SETUP_POLISH.md",
        "docs/RELEASE_NOTES_V2_11_0.md",
    ]),
    "active_milestone_updated": "v2_11_0_method_applicability_setup_polish" in read("docs/V2_ACTIVE_MILESTONE.md"),
    "smoke_passed": smoke.get("passed") is True,
    "setup_guidance_polished": all(token in combined_source for token in [
        "Method availability",
        "data-method-applicability-polish",
        "data-method-applicability-status",
        "Why not available yet",
        "method-guidance-needed",
    ]),
    "topbar_not_full_catalog": "More methods in Setup" in read("src/components/TopBar.tsx"),
    "data_model_guidance": "What can I do with this data?" in combined_source and "What can I do with this model?" in combined_source,
    "frontend_only_guard": not any(token in combined_source for token in ["Sigma(theta)", "F_ml =", "bootstrap replicate calculation changed"]),
    "no_mojibake": "RÂ²" not in combined_source and "RÃ" not in combined_source,
}

failed = [name for name, passed in checks.items() if not passed]
report = {
    "milestone": "v2_11_0_method_applicability_setup_polish",
    "passed": not failed,
    "checks": checks,
    "failed": failed,
}

(RESULTS / "v2110_method_setup_applicability_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
if failed:
    raise SystemExit(json.dumps(report, indent=2))
print("v2.11 method setup applicability audit passed")
