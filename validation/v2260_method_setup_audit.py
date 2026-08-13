import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT_PATH = ROOT / "validation" / "results" / "v2260_method_setup_audit.json"
MILESTONE = "v2_26_0_method_setup_applicability_center"
VERSION = "2.26.0"


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def read_json(relative: str):
    return json.loads(read_text(relative))


def check(name: str, passed: bool, evidence: str):
    return {"name": name, "passed": bool(passed), "evidence": evidence}


package = read_json("package.json")
package_lock = read_json("package-lock.json")
tauri = read_json("src-tauri/tauri.conf.json")
registry = read_json("validation/development_slices.json")
cargo_toml = read_text("Cargo.toml")
cargo_lock = read_text("Cargo.lock")
roadmap = read_text("crates/qpls-core/src/roadmap.rs")
catalog = read_text("src/components/AnalysisCatalog.tsx")
applicability = read_text("src/domain/methodApplicability.ts")
styles = read_text("src/styles.css")
active_doc = read_text("docs/V2_ACTIVE_MILESTONE.md")
bad_r_squared = chr(82) + chr(194) + chr(178)
bad_r_squared_utf8 = chr(82) + chr(195) + chr(130) + chr(194) + chr(178)

slice_by_id = {item["id"]: item for item in registry["slices"]}
milestone_slice = slice_by_id.get(MILESTONE, {})
scripts = package.get("scripts", {})

checks = [
    check("Package version is 2.26.0", package.get("version") == VERSION, "package.json"),
    check("Package lock version is 2.26.0", package_lock.get("version") == VERSION and package_lock.get("packages", {}).get("", {}).get("version") == VERSION, "package-lock.json"),
    check("Tauri version is 2.26.0", tauri.get("version") == VERSION, "src-tauri/tauri.conf.json"),
    check("Cargo workspace version is 2.26.0", 'version = "2.26.0"' in cargo_toml, "Cargo.toml"),
    check("Cargo lock workspace crates use 2.26.0", 'version = "2.25.0"' not in cargo_lock and 'version = "2.26.0"' in cargo_lock, "Cargo.lock"),
    check("Roadmap current-stage test expects v2.26", MILESTONE in roadmap, "crates/qpls-core/src/roadmap.rs"),
    check("Registry current stage is v2.26", registry.get("current_stage") == MILESTONE, "validation/development_slices.json"),
    check("Registry slice is validated", milestone_slice.get("status") == "validated", "v2.26 registry slice"),
    check("Registry gates all passed", bool(milestone_slice.get("gates")) and all(gate.get("status") == "passed" for gate in milestone_slice.get("gates", [])), "v2.26 gate entries"),
    check("NPM scripts are registered", all(key in scripts for key in [
        "qpls:v2260:method-setup-smoke",
        "qpls:v2260:method-setup-audit",
        "qpls:v2260:method-setup",
    ]), "package.json scripts"),
    check("Release artifact label is v2.26", MILESTONE in scripts.get("qpls:release:artifacts", ""), "package.json qpls:release:artifacts"),
    check("Setup center exposes required lanes", all(label in catalog for label in [
        "Recommended for this project",
        "Available now",
        "Available with setup",
        "Advanced diagnostics",
        "Standalone analyses",
        "Not applicable or scoped out",
    ]), "src/components/AnalysisCatalog.tsx"),
    check("Setup separates add-ons from primary methods", "setup-v226-addons" in catalog and "inferenceAddOns" in catalog, "src/components/AnalysisCatalog.tsx"),
    check("Applicability rules remain frontend-only", "invoke(" not in catalog and "invoke(" not in applicability, "Setup/applicability source boundary"),
    check("v2.26 CSS exists", ".setup-v226-workspace" in styles and ".setup-v226-category-tabs" in styles and ".setup-v226-addons" in styles, "src/styles.css"),
    check("Milestone documentation exists", (ROOT / "docs" / "V2_26_0_METHOD_SETUP_APPLICABILITY_CENTER.md").exists(), "docs/V2_26_0_METHOD_SETUP_APPLICABILITY_CENTER.md"),
    check("Active tracker advances to v2.27 next", "Current completed checkpoint: `v2_26_0_method_setup_applicability_center`" in active_doc and "v2_27_0_calculation_run_monitor" in active_doc, "docs/V2_ACTIVE_MILESTONE.md"),
    check("No Setup/applicability mojibake", bad_r_squared not in catalog and bad_r_squared not in applicability and bad_r_squared_utf8 not in catalog and bad_r_squared_utf8 not in applicability, "R-squared source audit"),
]

passed = all(item["passed"] for item in checks)
RESULT_PATH.parent.mkdir(parents=True, exist_ok=True)
RESULT_PATH.write_text(json.dumps({
    "id": "v2260_method_setup_audit",
    "milestone": MILESTONE,
    "passed": passed,
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "checks": checks,
}, indent=2), encoding="utf-8")

if not passed:
    print(json.dumps({"passed": passed, "failed": [item for item in checks if not item["passed"]]}, indent=2))
    raise SystemExit(1)

print(f"v2.26 method setup audit passed: {RESULT_PATH}")
