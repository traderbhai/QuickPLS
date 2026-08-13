import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
MILESTONE = "v2_17_0_home_data_setup_mockup_alignment"
VERSION = "2.17.0"


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
active_doc = read("docs/V2_ACTIVE_MILESTONE.md")
milestone_doc = read("docs/V2_17_0_HOME_DATA_SETUP_MOCKUP_ALIGNMENT.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")
home = read("src/components/OnboardingWorkspace.tsx")
data = read("src/components/DataWorkspace.tsx")
setup = read("src/components/AnalysisCatalog.tsx")
styles = read("src/styles.css")
smoke_path = RESULTS / "v2170_home_data_setup_smoke.json"
smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}
slice_ = next((item for item in registry.get("slices", []) if item.get("id") == MILESTONE), None)
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
source = "\n".join([home, data, setup, styles])
docs = "\n".join([active_doc, milestone_doc, delivery, ledger])
scripts = package.get("scripts", {})

checks = {
    "package_version": package.get("version") == VERSION,
    "package_lock_version": lock.get("version") == VERSION and lock.get("packages", {}).get("", {}).get("version") == VERSION,
    "cargo_version": f'version = "{VERSION}"' in cargo,
    "quickpls_cargo_lock_versions": quickpls_lock_versions and all(f'version = "{VERSION}"' in block for block in quickpls_lock_versions),
    "tauri_version": tauri.get("version") == VERSION,
    "release_label": MILESTONE in scripts.get("qpls:release:artifacts", ""),
    "scripts_registered": all(name in scripts for name in [
        "qpls:v2170:home-data-setup-smoke",
        "qpls:v2170:home-data-setup-audit",
        "qpls:v2170:home-data-setup",
    ]),
    "registry_current_stage": registry.get("current_stage") == MILESTONE,
    "registry_slice_passed": slice_ is not None and slice_.get("status") == "validated" and all(gate.get("status") == "passed" for gate in slice_.get("gates", [])),
    "roadmap_current_stage": MILESTONE in roadmap,
    "active_tracker_updated": MILESTONE in active_doc and "v2_18_0_model_run_results_mockup_alignment" in active_doc,
    "delivery_and_ledger_updated": MILESTONE in delivery and MILESTONE in ledger,
    "milestone_doc_scope": "frontend-only" in milestone_doc and "No statistical engines" in milestone_doc,
    "smoke_passed": smoke.get("passed") is True,
    "screen_markers": all(marker in source for marker in [
        'data-v217-mockup-screen="home"',
        'data-v217-mockup-screen="data"',
        'data-v217-mockup-screen="setup"',
    ]),
    "mockup_density_css": all(token in styles for token in [
        ".home-v217-workspace",
        ".data-v217-workspace",
        ".setup-v217-workspace",
        ".data-v217-preview .data-workbench",
        ".setup-v217-workspace .method-guidance-grid",
    ]),
    "frontend_only_boundary": not any(token in source for token in ["F_ml =", "Sigma(theta)", "optimizer reduction", "AnalysisResult {"]),
    "no_mojibake": "RÃ‚Â²" not in source + docs and "RÃƒ" not in source + docs,
    "no_smartpls_equivalence_claim": not any(
        token in (source + docs).lower()
        for token in ["identical to smartpls", "smartpls equivalent", "equivalent to smartpls"]
    ),
}

failures = [{"check": name} for name, passed in checks.items() if not passed]
result = {
    "passed": not failures,
    "milestone": MILESTONE,
    "version": VERSION,
    "checks": checks,
    "failures": failures,
}

RESULTS.mkdir(parents=True, exist_ok=True)
(RESULTS / "v2170_home_data_setup_audit.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
if failures:
    print(json.dumps(result, indent=2))
    raise SystemExit(1)
print("v2.17 Home/Data/Setup audit passed")
