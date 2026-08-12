import datetime
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
MILESTONE = "v2_0_9_mockup_fidelity_system"
VERSION = "2.0.9"
WORKSPACE_PACKAGES = {
    "qpls-assessment",
    "qpls-cli",
    "qpls-core",
    "qpls-data",
    "qpls-estimation",
    "qpls-project",
    "qpls-resampling",
    "qpls-runner",
    "quickpls-desktop",
}


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def read_json(path: str):
    return json.loads(read(path))


checks = []


def add(name: str, passed: bool, detail: str):
    checks.append({"name": name, "passed": bool(passed), "detail": detail})


def package_lock_is_current() -> bool:
    lock = read_json("package-lock.json")
    return lock.get("version") == VERSION and lock.get("packages", {}).get("", {}).get("version") == VERSION


def cargo_lock_workspace_versions_are_current() -> bool:
    text = read("Cargo.lock")
    found = {}
    for block in text.split("[[package]]"):
        name = re.search(r'name = "([^"]+)"', block)
        version = re.search(r'version = "([^"]+)"', block)
        if name and version and name.group(1) in WORKSPACE_PACKAGES:
            found[name.group(1)] = version.group(1)
    return set(found) == WORKSPACE_PACKAGES and all(value == VERSION for value in found.values())


pkg = read_json("package.json")
tauri = read_json("src-tauri/tauri.conf.json")
registry = read_json("validation/development_slices.json")
roadmap = read("crates/qpls-core/src/roadmap.rs")
contract = read("docs/V2_UI_VISUAL_CONTRACT.md")
doc = read("docs/V2_0_9_MOCKUP_FIDELITY_SYSTEM.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")
smoke_path = RESULTS / "v209_mockup_fidelity_smoke.json"
smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}

add("smoke result exists and passed", smoke.get("passed") is True, str(smoke_path))
add("package version is current", pkg.get("version") == VERSION, pkg.get("version", "missing"))
add("package-lock version is current", package_lock_is_current(), "package-lock root version")
add("Cargo.toml version is current", f'version = "{VERSION}"' in read("Cargo.toml"), "Cargo.toml")
add("Cargo.lock workspace versions are current", cargo_lock_workspace_versions_are_current(), "QuickPLS workspace packages")
add("Tauri version is current", tauri.get("version") == VERSION, tauri.get("version", "missing"))
add("release artifact label is current", MILESTONE in pkg["scripts"].get("qpls:release:artifacts", ""), pkg["scripts"].get("qpls:release:artifacts", "missing"))
add("current stage points to v2.0.9", registry.get("current_stage") == MILESTONE, registry.get("current_stage", "missing"))

slice_rows = [row for row in registry.get("slices", []) if row.get("id") == MILESTONE]
slice_ok = len(slice_rows) == 1 and all(gate.get("status") == "passed" for gate in slice_rows[0].get("gates", []))
add("registry slice exists with passed gates", slice_ok, MILESTONE)
add("roadmap test expects v2.0.9", MILESTONE in roadmap, "roadmap current stage")
add("contract has mockup matching checklist", "Mockup-Matching Rules" in contract and "Screen Completion Checklist" in contract, "V2_UI_VISUAL_CONTRACT.md")
add("v2.0.9 doc describes frontend-only boundary", "frontend/product-only" in doc and "No estimator changes" in doc, "V2_0_9 doc")
add("delivery status updated", MILESTONE in delivery or "v2.0.9 Mockup Fidelity" in delivery, "DELIVERY_STATUS")
add("development ledger updated", MILESTONE in ledger or "v2.0.9 Mockup Fidelity" in ledger, "DEVELOPMENT_LEDGER")

for previous_doc in [
    "docs/V2_0_0_DESIGN_SYSTEM_AND_SHELL.md",
    "docs/V2_0_1_HOME_DATA_REDESIGN.md",
    "docs/V2_0_2_SETUP_METHOD_GUIDANCE_REDESIGN.md",
    "docs/V2_0_3_VISUAL_FIDELITY_FOUNDATION.md",
    "docs/V2_0_4_RESULTS_TABLE_INTERPRETATION_REDESIGN.md",
    "docs/V2_0_5_REPORT_EXPORT_FLOW_REDESIGN.md",
    "docs/V2_0_6_MODEL_SHELL_SEM_DESIGNER_SURROUND.md",
    "docs/V2_0_7_RUN_EXECUTION_SURFACE_REDESIGN.md",
    "docs/V2_0_8_TRUST_CENTER_SCOPE_TRANSPARENCY.md",
]:
    add(f"previous v2 doc exists {previous_doc}", (ROOT / previous_doc).exists(), previous_doc)

artifact_script = pkg["scripts"].get("qpls:release:artifacts", "")
add("versioned artifact script uses label argument", "--label" in artifact_script and MILESTONE in artifact_script, artifact_script)
add("desktop build script packages then copies artifacts", "npm run tauri -- build" in pkg["scripts"].get("qpls:desktop:build-versioned", "") and "qpls:release:artifacts" in pkg["scripts"].get("qpls:desktop:build-versioned", ""), pkg["scripts"].get("qpls:desktop:build-versioned", "missing"))

combined = "\n".join([contract, doc, delivery, ledger])
add("docs have no mojibake", re.search(r"[ÃƒÆ’Ãƒâ€šÃ¯Â¿Â½ÃƒÂ¯Ã‚Â¿Ã‚Â½]|RÃ‚Â²", combined) is None, "documentation encoding")
add("docs avoid SmartPLS equivalence claims", re.search(r"identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS", combined, re.I) is None, "claim boundary")

passed = all(check["passed"] for check in checks)
result = {
    "passed": passed,
    "milestone": MILESTONE,
    "generated_at": datetime.datetime.now(datetime.UTC).isoformat(),
    "checks": checks,
}

RESULTS.mkdir(parents=True, exist_ok=True)
(RESULTS / "v209_mockup_fidelity_audit.json").write_text(json.dumps(result, indent=2), encoding="utf-8")

if not passed:
    print(json.dumps(result, indent=2))
    raise SystemExit(1)

print("v2.0.9 mockup fidelity audit passed")
