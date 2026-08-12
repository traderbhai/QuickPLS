import datetime
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
MILESTONE = "v2_0_7_run_execution_surface_redesign"
VERSION = "2.0.7"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def package_version_in_lock() -> bool:
    lock = json.loads(read("package-lock.json"))
    return lock.get("version") == VERSION and lock.get("packages", {}).get("", {}).get("version") == VERSION


def cargo_lock_workspace_versions() -> bool:
    text = read("Cargo.lock")
    workspace = {
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
    blocks = text.split("[[package]]")
    found = {}
    for block in blocks:
        name_match = re.search(r'name = "([^"]+)"', block)
        version_match = re.search(r'version = "([^"]+)"', block)
        if name_match and version_match and name_match.group(1) in workspace:
            found[name_match.group(1)] = version_match.group(1)
    return workspace == set(found) and all(version == VERSION for version in found.values())


checks = []


def add(name: str, passed: bool, detail: str):
    checks.append({"name": name, "passed": bool(passed), "detail": detail})


pkg = json.loads(read("package.json"))
registry = json.loads(read("validation/development_slices.json"))
run = read("src/components/RunWorkspace.tsx")
styles = read("src/styles.css")
topbar = read("src/components/TopBar.tsx")
roadmap = read("crates/qpls-core/src/roadmap.rs")
doc = read("docs/V2_0_7_RUN_EXECUTION_SURFACE_REDESIGN.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")
tauri = json.loads(read("src-tauri/tauri.conf.json"))
smoke_path = RESULTS / "v207_run_surface_smoke.json"
smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}

add("smoke result exists and passed", smoke.get("passed") is True, str(smoke_path))
add("package version is current", pkg.get("version") == VERSION, pkg.get("version", "missing"))
add("package-lock version is current", package_version_in_lock(), "package-lock root version")
add("Cargo.toml version is current", 'version = "2.0.7"' in read("Cargo.toml"), "Cargo.toml")
add("Cargo.lock workspace versions are current", cargo_lock_workspace_versions(), "QuickPLS workspace packages")
add("Tauri version is current", tauri.get("version") == VERSION, tauri.get("version", "missing"))
add("release artifact label is current", MILESTONE in pkg["scripts"].get("qpls:release:artifacts", ""), pkg["scripts"].get("qpls:release:artifacts", "missing"))
add("current stage points to v2.0.7", registry.get("current_stage") == MILESTONE, registry.get("current_stage", "missing"))

slice_rows = [row for row in registry.get("slices", []) if row.get("id") == MILESTONE]
slice_ok = len(slice_rows) == 1 and all(gate.get("status") == "passed" for gate in slice_rows[0].get("gates", []))
add("registry slice exists with passed gates", slice_ok, MILESTONE)
add("roadmap test expects v2.0.7", MILESTONE in roadmap, "roadmap current stage")
add("top bar label is current", "v2.0.7 run surface redesign" in topbar, "TopBar alpha mark")

for token in [
    "run-v2-workspace",
    "run-v2-hero",
    "run-v2-readiness-panel",
    "run-v2-output-preview",
    "run-v2-execution-plan",
    "run-v2-handoff-grid",
    "Run disabled:",
    "quickpls:run-analysis",
]:
    add(f"Run workspace includes {token}", token in run, token)
    if token.startswith("run-v2"):
        add(f"Styles include {token}", f".{token}" in styles, token)

add("docs mention frontend-only boundary", "frontend/product-only" in doc and "No estimator behavior changes" in doc, "v2.0.7 doc")
add("delivery status updated", MILESTONE in delivery or "v2.0.7 Run Execution Surface Redesign" in delivery, "DELIVERY_STATUS")
add("development ledger updated", MILESTONE in ledger or "v2.0.7 Run Execution Surface Redesign" in ledger, "DEVELOPMENT_LEDGER")
add("Run source has no mojibake", re.search(r"[ÃÂ�ï¿½]", run) is None, "RunWorkspace encoding")

passed = all(check["passed"] for check in checks)
result = {
    "passed": passed,
    "milestone": MILESTONE,
    "generated_at": datetime.datetime.now(datetime.UTC).isoformat(),
    "checks": checks,
}

RESULTS.mkdir(parents=True, exist_ok=True)
(RESULTS / "v207_run_surface_audit.json").write_text(json.dumps(result, indent=2), encoding="utf-8")

if not passed:
    print(json.dumps(result, indent=2))
    raise SystemExit(1)

print("v2.0.7 Run surface audit passed")
