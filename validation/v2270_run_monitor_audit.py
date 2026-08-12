import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT_PATH = ROOT / "validation" / "results" / "v2270_run_monitor_audit.json"
MILESTONE = "v2_27_0_calculation_run_monitor"
VERSION = "2.27.0"


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
run_workspace = read_text("src/components/RunWorkspace.tsx")
top_bar = read_text("src/components/TopBar.tsx")
store = read_text("src/store.ts")
types = read_text("src/types.ts")
app = read_text("src/App.tsx")
styles = read_text("src/styles.css")
active_doc = read_text("docs/V2_ACTIVE_MILESTONE.md")
delivery = read_text("docs/DELIVERY_STATUS.md")
ledger = read_text("docs/DEVELOPMENT_LEDGER.md")
scripts = package.get("scripts", {})
slice_by_id = {item["id"]: item for item in registry["slices"]}
milestone_slice = slice_by_id.get(MILESTONE, {})
bad_r_squared = chr(82) + chr(194) + chr(178)
bad_r_squared_utf8 = chr(82) + chr(195) + chr(130) + chr(194) + chr(178)

checks = [
    check("Package version is 2.27.0", package.get("version") == VERSION, "package.json"),
    check("Package lock version is 2.27.0", package_lock.get("version") == VERSION and package_lock.get("packages", {}).get("", {}).get("version") == VERSION, "package-lock.json"),
    check("Tauri version is 2.27.0", tauri.get("version") == VERSION, "src-tauri/tauri.conf.json"),
    check("Cargo workspace version is 2.27.0", 'version = "2.27.0"' in cargo_toml, "Cargo.toml"),
    check("Cargo lock workspace crates use 2.27.0", 'version = "2.26.0"' not in cargo_lock and 'version = "2.27.0"' in cargo_lock, "Cargo.lock"),
    check("Roadmap current-stage test expects v2.27", MILESTONE in roadmap, "crates/qpls-core/src/roadmap.rs"),
    check("Registry current stage is v2.27", registry.get("current_stage") == MILESTONE, "validation/development_slices.json"),
    check("Registry slice is validated", milestone_slice.get("status") == "validated", "v2.27 registry slice"),
    check("Registry gates all passed", bool(milestone_slice.get("gates")) and all(gate.get("status") == "passed" for gate in milestone_slice.get("gates", [])), "v2.27 gate entries"),
    check("NPM scripts are registered", all(key in scripts for key in [
        "qpls:v2270:run-monitor-smoke",
        "qpls:v2270:run-monitor-audit",
        "qpls:v2270:run-monitor",
    ]), "package.json scripts"),
    check("Release artifact label is v2.27", MILESTONE in scripts.get("qpls:release:artifacts", ""), "package.json qpls:release:artifacts"),
    check("Run monitor workspace marker exists", 'data-v227-run-monitor="true"' in run_workspace and "run-v227-monitor-grid" in run_workspace, "src/components/RunWorkspace.tsx"),
    check("Run workspace has procedure progress settings and footer", all(token in run_workspace for token in ["Procedure", "run-v227-progress-panel", "Run settings", "run-v227-footer", "Outputs produced", "Unavailable in this run"]), "src/components/RunWorkspace.tsx"),
    check("Run workspace dispatches run and cancel events", "quickpls:run-analysis" in run_workspace and "quickpls:cancel-analysis" in run_workspace, "src/components/RunWorkspace.tsx"),
    check("TopBar listens for cancel event", "quickpls:cancel-analysis" in top_bar and "handleCancelRunRequest" in top_bar, "src/components/TopBar.tsx"),
    check("TopBar updates monitor states", all(token in top_bar for token in ['status: "queued"', 'status: "validating"', '"running"', 'status: "cancelling"', 'status: "completed"', 'status: "failed"', 'status: "cancelled"', 'status: "blocked"']), "src/components/TopBar.tsx"),
    check("Store has shared run monitor actions", all(token in store for token in ["runMonitor", "setRunMonitor", "appendRunLog", "resetRunMonitor"]), "src/store.ts"),
    check("Types cover run monitor states", all(token in types for token in ["RunMonitorStatus", "RunMonitorState", "RunMonitorLogEntry", "queued", "validating", "cancelling", "cancelled"]), "src/types.ts"),
    check("Smoke API can fixture monitor states", "setRunMonitorFixture" in app, "src/App.tsx"),
    check("v2.27 CSS exists", all(token in styles for token in [".run-v227-monitor-grid", ".run-v227-steps", ".run-v227-progress-track", ".run-v227-log", ".run-v227-setting-grid", ".run-v227-footer"]), "src/styles.css"),
    check("Milestone documentation exists", (ROOT / "docs" / "V2_27_0_CALCULATION_RUN_MONITOR.md").exists(), "docs/V2_27_0_CALCULATION_RUN_MONITOR.md"),
    check("Active tracker advances to v2.28 next", "Current completed checkpoint: `v2_27_0_calculation_run_monitor`" in active_doc and "v2_28_0_results_workbook_redesign" in active_doc, "docs/V2_ACTIVE_MILESTONE.md"),
    check("Delivery and ledger mention v2.27", "QuickPLS 2.27.0" in delivery and "v2.27.0 - Calculation Run Monitor" in ledger, "docs delivery/ledger"),
    check("Frontend-only boundary preserved", "invoke(" not in run_workspace and "invoke(" not in app, "Run workspace/App source boundary"),
    check("No run-monitor R-squared mojibake", bad_r_squared not in run_workspace and bad_r_squared_utf8 not in run_workspace, "R-squared source audit"),
]

passed = all(item["passed"] for item in checks)
RESULT_PATH.parent.mkdir(parents=True, exist_ok=True)
RESULT_PATH.write_text(json.dumps({
    "id": "v2270_run_monitor_audit",
    "milestone": MILESTONE,
    "passed": passed,
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "checks": checks,
}, indent=2), encoding="utf-8")

if not passed:
    print(json.dumps({"passed": passed, "failed": [item for item in checks if not item["passed"]]}, indent=2))
    raise SystemExit(1)

print(f"v2.27 run monitor audit passed: {RESULT_PATH}")
