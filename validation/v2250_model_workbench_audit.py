import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT_PATH = ROOT / "validation" / "results" / "v2250_model_workbench_audit.json"
MILESTONE = "v2_25_0_model_workbench_integration"
VERSION = "2.25.0"


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
styles = read_text("src/styles.css")
app = read_text("src/App.tsx")
issues_pane = read_text("src/components/ModelIssuesPane.tsx")

slice_by_id = {item["id"]: item for item in registry["slices"]}
milestone_slice = slice_by_id.get(MILESTONE, {})

checks = [
    check("Package version is 2.25.0", package.get("version") == VERSION, "package.json"),
    check("Package lock version is 2.25.0", package_lock.get("version") == VERSION and package_lock.get("packages", {}).get("", {}).get("version") == VERSION, "package-lock.json"),
    check("Tauri version is 2.25.0", tauri.get("version") == VERSION, "src-tauri/tauri.conf.json"),
    check("Cargo workspace version is 2.25.0", 'version = "2.25.0"' in cargo_toml, "Cargo.toml"),
    check("Cargo lock workspace crates use 2.25.0", 'version = "2.24.0"' not in cargo_lock and 'version = "2.25.0"' in cargo_lock, "Cargo.lock"),
    check("Roadmap current-stage test expects v2.25", MILESTONE in roadmap, "crates/qpls-core/src/roadmap.rs"),
    check("Registry current stage is v2.25", registry.get("current_stage") == MILESTONE, "validation/development_slices.json"),
    check("Registry slice is validated", milestone_slice.get("status") == "validated", "v2.25 registry slice"),
    check("Registry gates all passed", bool(milestone_slice.get("gates")) and all(gate.get("status") == "passed" for gate in milestone_slice.get("gates", [])), "v2.25 gate entries"),
    check("NPM scripts are registered", all(key in package.get("scripts", {}) for key in [
        "qpls:v2250:model-workbench-smoke",
        "qpls:v2250:model-workbench-audit",
        "qpls:v2250:model-workbench",
    ]), "package.json scripts"),
    check("Release artifact label is v2.25", MILESTONE in package.get("scripts", {}).get("qpls:release:artifacts", ""), "package.json qpls:release:artifacts"),
    check("Bottom pane is rendered in model route", "<ModelIssuesPane />" in app, "src/App.tsx"),
    check("Bottom pane stays frontend-only", "useWorkspace" in issues_pane and "analysisReadiness" in issues_pane and "invoke(" not in issues_pane, "src/components/ModelIssuesPane.tsx"),
    check("Model workbench CSS exists", ".model-v225-bottom-pane" in styles and ".model-v225-inspector" in styles and ".model-v225-canvas" in styles, "src/styles.css"),
    check("Focus Diagram CSS covers v2.25 panes", ".focus-diagram-mode .model-v225-bottom-pane" in styles and ".focus-diagram-mode .model-v225-canvas" in styles, "src/styles.css"),
    check("Milestone documentation exists", (ROOT / "docs" / "V2_25_0_MODEL_WORKBENCH_INTEGRATION.md").exists(), "docs/V2_25_0_MODEL_WORKBENCH_INTEGRATION.md"),
    check("No touched source contains R mojibake", "RÂ²" not in "\n".join([styles, app, issues_pane, read_text("src/components/ModelCanvas.tsx"), read_text("src/components/Inspector.tsx"), read_text("src/components/Explorer.tsx")]), "R² source audit"),
]

passed = all(item["passed"] for item in checks)
RESULT_PATH.parent.mkdir(parents=True, exist_ok=True)
RESULT_PATH.write_text(json.dumps({
    "id": "v2250_model_workbench_audit",
    "milestone": MILESTONE,
    "passed": passed,
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "checks": checks,
}, indent=2), encoding="utf-8")

if not passed:
    print(json.dumps({"passed": passed, "failed": [item for item in checks if not item["passed"]]}, indent=2))
    raise SystemExit(1)

print(f"v2.25 model workbench audit passed: {RESULT_PATH}")
