import datetime
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
SMOKE = RESULTS / "v211_mockup_pixel_alignment_smoke.json"
OUTPUT = RESULTS / "v211_mockup_pixel_alignment_audit.json"
MILESTONE = "v2_0_11_mockup_pixel_alignment"
VERSION = "2.0.11"
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


def cargo_lock_workspace_versions_are_current() -> bool:
    text = read("Cargo.lock")
    found = {}
    for block in text.split("[[package]]"):
        name = re.search(r'name = "([^"]+)"', block)
        version = re.search(r'version = "([^"]+)"', block)
        if name and version and name.group(1) in WORKSPACE_PACKAGES:
            found[name.group(1)] = version.group(1)
    return set(found) == WORKSPACE_PACKAGES and all(value == VERSION for value in found.values())


def add(checks: list[dict[str, object]], name: str, passed: bool, detail: str):
    checks.append({"name": name, "passed": bool(passed), "detail": detail})


def normal_source_without_mojibake() -> bool:
    checked = [
        "src/components/AnalysisCatalog.tsx",
        "src/components/RunHistory.tsx",
        "src/components/ReportsWorkspace.tsx",
        "src/components/ModelCanvas.tsx",
        "src/components/TopBar.tsx",
        "src/styles.css",
    ]
    combined = "\n".join(read(path) for path in checked)
    return re.search(r"RÃ|RÂ²|Ãƒ|Â", combined) is None


checks: list[dict[str, object]] = []
pkg = read_json("package.json")
package_lock = read_json("package-lock.json")
tauri = read_json("src-tauri/tauri.conf.json")
registry = read_json("validation/development_slices.json")
roadmap = read("crates/qpls-core/src/roadmap.rs")
doc = read("docs/V2_0_11_MOCKUP_PIXEL_ALIGNMENT.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")
contract = read("docs/V2_UI_VISUAL_CONTRACT.md")
smoke = json.loads(SMOKE.read_text(encoding="utf-8")) if SMOKE.exists() else {}
scripts = pkg.get("scripts", {})

add(checks, "pixel smoke exists and passed", smoke.get("passed") is True, str(SMOKE.relative_to(ROOT)))
add(checks, "all primary workspaces captured", smoke.get("checklist", {}).get("all_primary_workspaces_captured") is True, "Home/Data/Model/Setup/Run/Results/Report/Trust/Settings")
add(checks, "1280 desktop subset captured", smoke.get("checklist", {}).get("desktop_1280_subset_captured") is True, "1280x800 subset")
add(checks, "no visual gaps remain", smoke.get("checklist", {}).get("no_visual_gaps") is True, f"{len(smoke.get('issues', []))} issues")
add(checks, "Results command center present", smoke.get("checklist", {}).get("results_command_center_present") is True, "empty and desktop Results state")
add(checks, "trust entries visible", smoke.get("checklist", {}).get("evidence_workspaces_have_trust_entries") is True, "Setup/Results/Report/Trust")
add(checks, "package version is current", pkg.get("version") == VERSION, pkg.get("version", "missing"))
add(checks, "package-lock version is current", package_lock.get("version") == VERSION and package_lock.get("packages", {}).get("", {}).get("version") == VERSION, "package-lock root")
add(checks, "Cargo.toml version is current", f'version = "{VERSION}"' in read("Cargo.toml"), "Cargo.toml")
add(checks, "Cargo.lock workspace versions are current", cargo_lock_workspace_versions_are_current(), "QuickPLS workspace package versions")
add(checks, "Tauri version is current", tauri.get("version") == VERSION, tauri.get("version", "missing"))
add(checks, "release artifact label is current", MILESTONE in scripts.get("qpls:release:artifacts", ""), scripts.get("qpls:release:artifacts", "missing"))
add(checks, "current stage is v2.0.11", registry.get("current_stage") == MILESTONE, registry.get("current_stage", "missing"))

slices = [item for item in registry.get("slices", []) if item.get("id") == MILESTONE]
add(checks, "registry slice exists and all gates passed", len(slices) == 1 and all(gate.get("status") == "passed" for gate in slices[0].get("gates", [])), MILESTONE)
add(checks, "roadmap expects v2.0.11", MILESTONE in roadmap, "roadmap current-stage test")
add(checks, "milestone doc states frontend boundary", "frontend/product-only" in doc and "No estimator changes" in doc, "V2_0_11 doc")
add(checks, "visual contract remains present", "QuickPLS 2.0" in contract and "visual contract" in contract.lower(), "V2_UI_VISUAL_CONTRACT.md")
add(checks, "delivery status updated", "v2.0.11 Mockup Pixel Alignment" in delivery or MILESTONE in delivery, "DELIVERY_STATUS")
add(checks, "development ledger updated", "v2.0.11 Mockup Pixel Alignment" in ledger or MILESTONE in ledger, "DEVELOPMENT_LEDGER")
add(checks, "npm scripts exist", all(name in scripts for name in ["qpls:v211:pixel-smoke", "qpls:v211:pixel-audit", "qpls:v211:pixel-alignment"]), "package.json scripts")
add(checks, "normal UI source has no mojibake", normal_source_without_mojibake(), "selected UI source files")

combined_docs = "\n".join([doc, delivery, ledger, contract])
add(checks, "docs have no mojibake", re.search(r"RÃ|RÂ²|Ãƒ|Â", combined_docs) is None, "documentation encoding")
add(checks, "docs avoid SmartPLS equivalence claims", re.search(r"identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS", combined_docs, re.I) is None, "claim boundary")

passed = all(check["passed"] for check in checks)
result = {
    "schema_version": 1,
    "target": "QuickPLS v2.0.11 mockup pixel alignment audit",
    "milestone": MILESTONE,
    "passed": passed,
    "generated_at": datetime.datetime.now(datetime.UTC).isoformat(),
    "checks": checks,
    "issues": smoke.get("issues", []),
}

RESULTS.mkdir(parents=True, exist_ok=True)
OUTPUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
if not passed:
    print(json.dumps(result, indent=2))
    raise SystemExit(1)

print("v2.0.11 mockup pixel alignment audit passed")
