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
smoke_path = RESULTS / "v181_method_applicability_smoke.json"
smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {"passed": False}

registry_slice = next((item for item in slices.get("slices", []) if item.get("id") == "v1_8_1_method_applicability_guided_setup"), None)
source_files = [
    "src/domain/methodApplicability.ts",
    "src/domain/analysisReadiness.ts",
    "src/components/AnalysisCatalog.tsx",
    "src/components/TopBar.tsx",
    "src/components/DataWorkspace.tsx",
    "src/components/Explorer.tsx",
]
combined_source = "\n".join(read(path) for path in source_files)

checks = {
    "package_version_1_8_1": package.get("version") == "1.8.1",
    "package_lock_version_1_8_1": lock.get("version") == "1.8.1" and lock.get("packages", {}).get("", {}).get("version") == "1.8.1",
    "cargo_version_1_8_1": 'version = "1.8.1"' in cargo,
    "tauri_version_1_8_1": tauri.get("version") == "1.8.1",
    "artifact_label_v181": "v1_8_1_method_applicability_guided_setup" in package["scripts"].get("qpls:release:artifacts", ""),
    "scripts_registered": all(name in package["scripts"] for name in [
        "qpls:v181:method-applicability-smoke",
        "qpls:v181:method-applicability-audit",
        "qpls:v181:method-applicability",
    ]),
    "current_stage_v181": slices.get("current_stage") == "v1_8_1_method_applicability_guided_setup",
    "registry_slice_exists": registry_slice is not None,
    "registry_slice_passed": registry_slice is not None and all(gate.get("status") == "passed" for gate in registry_slice.get("gates", [])),
    "doc_exists": (ROOT / "docs" / "V1_8_1_METHOD_APPLICABILITY_GUIDED_SETUP.md").exists(),
    "smoke_passed": smoke.get("passed") is True,
    "topbar_not_full_catalog": "More methods in Setup" in read("src/components/TopBar.tsx"),
    "setup_has_show_all": "Show all methods" in read("src/components/AnalysisCatalog.tsx"),
    "data_model_guidance": "What can I do with this data?" in combined_source and "What can I do with this model?" in combined_source,
    "no_mojibake": "RÂ²" not in combined_source and "RÃ‚Â²" not in combined_source,
}

failed = [name for name, passed in checks.items() if not passed]
report = {
    "milestone": "v1_8_1_method_applicability_guided_setup",
    "passed": not failed,
    "checks": checks,
    "failed": failed,
}

(RESULTS / "v181_method_applicability_audit.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
if failed:
    raise SystemExit(json.dumps(report, indent=2))
print("v1.8.1 method applicability audit passed")
