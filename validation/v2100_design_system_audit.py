import datetime
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
SMOKE = RESULTS / "v2100_design_system_smoke.json"
OUTPUT = RESULTS / "v2100_design_system_audit.json"
MILESTONE = "v2_1_0_design_system_foundation"
VERSION = "2.1.0"
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


def add(checks: list[dict[str, object]], name: str, passed: bool, detail: str):
    checks.append({"name": name, "passed": bool(passed), "detail": detail})


def cargo_lock_workspace_versions_are_current() -> bool:
    text = read("Cargo.lock")
    found = {}
    for block in text.split("[[package]]"):
        name = re.search(r'name = "([^"]+)"', block)
        version = re.search(r'version = "([^"]+)"', block)
        if name and version and name.group(1) in WORKSPACE_PACKAGES:
            found[name.group(1)] = version.group(1)
    return set(found) == WORKSPACE_PACKAGES and all(value == VERSION for value in found.values())


def source_has_required_primitives() -> bool:
    ui = read("src/components/Ui.tsx")
    styles = read("src/styles.css")
    required_exports = [
        "WorkspacePage",
        "Panel",
        "MetricCard",
        "CommandGroup",
        "ToolbarButton",
        "InlineNotice",
    ]
    required_classes = [
        ".qpls2-page-shell",
        ".qpls2-design-panel",
        ".qpls2-metric-card",
        ".qpls2-command-group",
        ".qpls2-toolbar-button",
        ".qpls2-inline-notice",
    ]
    return all(f"function {name}" in ui for name in required_exports) and all(name in styles for name in required_classes)


def source_without_mojibake() -> bool:
    checked = [
        "src/components/Ui.tsx",
        "src/components/SettingsWorkspace.tsx",
        "src/components/TopBar.tsx",
        "src/styles.css",
    ]
    combined = "\n".join(read(path) for path in checked)
    return re.search(r"RÃƒ|RÃ‚Â²|ÃƒÆ’|Ã‚", combined) is None


checks: list[dict[str, object]] = []
pkg = read_json("package.json")
package_lock = read_json("package-lock.json")
tauri = read_json("src-tauri/tauri.conf.json")
registry = read_json("validation/development_slices.json")
roadmap = read("crates/qpls-core/src/roadmap.rs")
doc = read("docs/V2_1_0_DESIGN_SYSTEM_FOUNDATION.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")
contract = read("docs/V2_UI_VISUAL_CONTRACT.md")
smoke = json.loads(SMOKE.read_text(encoding="utf-8")) if SMOKE.exists() else {}
scripts = pkg.get("scripts", {})

add(checks, "design smoke exists and passed", smoke.get("passed") is True, str(SMOKE.relative_to(ROOT)))
add(checks, "all primary workspaces captured", smoke.get("checklist", {}).get("all_primary_workspaces_captured") is True, "Home/Data/Model/Setup/Run/Results/Report/Trust/Settings")
add(checks, "1280 desktop subset captured", smoke.get("checklist", {}).get("desktop_1280_subset_captured") is True, "1280x800 subset")
add(checks, "settings primitive preview complete", smoke.get("checklist", {}).get("design_system_preview_complete") is True, "Settings design-system preview")
add(checks, "no visual gaps remain", smoke.get("checklist", {}).get("no_visual_gaps") is True, f"{len(smoke.get('issues', []))} issues")
add(checks, "package version is current", pkg.get("version") == VERSION, pkg.get("version", "missing"))
add(checks, "package-lock version is current", package_lock.get("version") == VERSION and package_lock.get("packages", {}).get("", {}).get("version") == VERSION, "package-lock root")
add(checks, "Cargo.toml version is current", f'version = "{VERSION}"' in read("Cargo.toml"), "Cargo.toml")
add(checks, "Cargo.lock workspace versions are current", cargo_lock_workspace_versions_are_current(), "QuickPLS workspace package versions")
add(checks, "Tauri version is current", tauri.get("version") == VERSION, tauri.get("version", "missing"))
add(checks, "release artifact label is current", MILESTONE in scripts.get("qpls:release:artifacts", ""), scripts.get("qpls:release:artifacts", "missing"))
add(checks, "current stage is v2.1.0", registry.get("current_stage") == MILESTONE, registry.get("current_stage", "missing"))

slices = [item for item in registry.get("slices", []) if item.get("id") == MILESTONE]
add(checks, "registry slice exists and all gates passed", len(slices) == 1 and all(gate.get("status") == "passed" for gate in slices[0].get("gates", [])), MILESTONE)
add(checks, "roadmap expects v2.1.0", MILESTONE in roadmap, "roadmap current-stage test")
add(checks, "milestone doc states frontend boundary", "frontend/product-only" in doc and "No estimator changes" in doc, "V2_1_0 doc")
add(checks, "visual contract remains present", "QuickPLS 2.0" in contract and "visual contract" in contract.lower(), "V2_UI_VISUAL_CONTRACT.md")
add(checks, "delivery status updated", "v2.1.0 Design System Foundation" in delivery or MILESTONE in delivery, "DELIVERY_STATUS")
add(checks, "development ledger updated", "v2.1.0 Design System Foundation" in ledger or MILESTONE in ledger, "DEVELOPMENT_LEDGER")
add(checks, "npm scripts exist", all(name in scripts for name in ["qpls:v2100:design-system-smoke", "qpls:v2100:design-system-audit", "qpls:v2100:design-system"]), "package.json scripts")
add(checks, "required primitives exist", source_has_required_primitives(), "Ui.tsx and styles.css")
add(checks, "normal UI source has no mojibake", source_without_mojibake(), "selected UI source files")

combined_docs = "\n".join([doc, delivery, ledger, contract])
add(checks, "docs have no mojibake", re.search(r"RÃƒ|RÃ‚Â²|ÃƒÆ’|Ã‚", combined_docs) is None, "documentation encoding")
add(checks, "docs avoid SmartPLS equivalence claims", re.search(r"identical to SmartPLS|SmartPLS equivalent|equivalent to SmartPLS", combined_docs, re.I) is None, "claim boundary")

passed = all(check["passed"] for check in checks)
result = {
    "schema_version": 1,
    "target": "QuickPLS v2.1.0 design system foundation audit",
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

print("v2.1.0 design system foundation audit passed")
