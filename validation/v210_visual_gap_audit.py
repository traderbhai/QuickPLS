import datetime
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
SMOKE = RESULTS / "v210_visual_gap_smoke.json"
OUTPUT = RESULTS / "v210_visual_gap_audit.json"
MILESTONE = "v2_0_10_visual_gap_audit"
VERSION = "2.0.10"
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


checks: list[dict[str, object]] = []
pkg = read_json("package.json")
package_lock = read_json("package-lock.json")
tauri = read_json("src-tauri/tauri.conf.json")
registry = read_json("validation/development_slices.json")
roadmap = read("crates/qpls-core/src/roadmap.rs")
doc = read("docs/V2_0_10_VISUAL_GAP_AUDIT.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")
contract = read("docs/V2_UI_VISUAL_CONTRACT.md")
smoke = json.loads(SMOKE.read_text(encoding="utf-8")) if SMOKE.exists() else {}

add(checks, "visual gap smoke exists and passed", smoke.get("passed") is True, str(SMOKE.relative_to(ROOT)))
add(checks, "smoke captured all primary workspaces", smoke.get("checklist", {}).get("all_primary_workspaces_captured") is True, "Home/Data/Model/Setup/Run/Results/Report/Trust/Settings")
add(checks, "smoke captured 1280 desktop subset", smoke.get("checklist", {}).get("desktop_1280_subset_captured") is True, "1280x800 subset")
add(checks, "smoke found no high-severity gaps", smoke.get("checklist", {}).get("no_high_severity_visual_gaps") is True, f"{len(smoke.get('issues', []))} issues")
add(checks, "package version is current", pkg.get("version") == VERSION, pkg.get("version", "missing"))
add(checks, "package-lock version is current", package_lock.get("version") == VERSION and package_lock.get("packages", {}).get("", {}).get("version") == VERSION, "package-lock root")
add(checks, "Cargo.toml version is current", f'version = "{VERSION}"' in read("Cargo.toml"), "Cargo.toml")
add(checks, "Cargo.lock workspace versions are current", cargo_lock_workspace_versions_are_current(), "QuickPLS workspace package versions")
add(checks, "Tauri version is current", tauri.get("version") == VERSION, tauri.get("version", "missing"))
add(checks, "release artifact label is current", MILESTONE in pkg.get("scripts", {}).get("qpls:release:artifacts", ""), pkg.get("scripts", {}).get("qpls:release:artifacts", "missing"))
add(checks, "current stage is v2.0.10", registry.get("current_stage") == MILESTONE, registry.get("current_stage", "missing"))

slices = [item for item in registry.get("slices", []) if item.get("id") == MILESTONE]
add(checks, "registry slice exists and all gates passed", len(slices) == 1 and all(gate.get("status") == "passed" for gate in slices[0].get("gates", [])), MILESTONE)
add(checks, "roadmap expects v2.0.10", MILESTONE in roadmap, "roadmap current-stage test")
add(checks, "milestone doc states frontend boundary", "frontend/product-only" in doc and "No estimator changes" in doc, "V2_0_10 doc")
add(checks, "visual contract names visual gap audit", "visual gap audit" in contract.lower(), "V2_UI_VISUAL_CONTRACT.md")
add(checks, "delivery status updated", "v2.0.10 Visual Gap Audit" in delivery or MILESTONE in delivery, "DELIVERY_STATUS")
add(checks, "development ledger updated", "v2.0.10 Visual Gap Audit" in ledger or MILESTONE in ledger, "DEVELOPMENT_LEDGER")

scripts = pkg.get("scripts", {})
add(checks, "npm scripts exist", all(name in scripts for name in ["qpls:v210:visual-gap-smoke", "qpls:v210:visual-gap-audit", "qpls:v210:visual-gap"]), "package.json scripts")
add(checks, "artifact preservation script remains versioned", "--label" in scripts.get("qpls:release:artifacts", "") and "qpls:release:artifacts" in scripts.get("qpls:desktop:build-versioned", ""), "release artifact scripts")

combined_docs = "\n".join([doc, delivery, ledger, contract])
add(checks, "docs have no mojibake", re.search(r"RÃ|RÂ²|Ãƒ|Â", combined_docs) is None, "documentation encoding")
add(checks, "docs avoid SmartPLS equivalence claims", re.search(r"identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS", combined_docs, re.I) is None, "claim boundary")

passed = all(check["passed"] for check in checks)
result = {
    "schema_version": 1,
    "target": "QuickPLS v2.0.10 visual gap audit",
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

print("v2.0.10 visual gap audit passed")
