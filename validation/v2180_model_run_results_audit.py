from __future__ import annotations

import json
import re
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "v2180_model_run_results_audit.json"
MILESTONE = "v2_18_0_model_run_results_mockup_alignment"
VERSION = "2.18.0"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def check(name: str, passed: bool, detail: str) -> dict[str, object]:
    return {"name": name, "passed": bool(passed), "detail": detail}


checks: list[dict[str, object]] = []

package = json.loads(read("package.json"))
package_lock = json.loads(read("package-lock.json"))
tauri = json.loads(read("src-tauri/tauri.conf.json"))
registry = json.loads(read("validation/development_slices.json"))
topbar = read("src/components/TopBar.tsx")
model = read("src/components/ModelCanvas.tsx")
run = read("src/components/RunWorkspace.tsx")
results = read("src/components/RunHistory.tsx")
styles = read("src/styles.css")
roadmap = read("crates/qpls-core/src/roadmap.rs")
active = read("docs/V2_ACTIVE_MILESTONE.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")
milestone_doc = read("docs/V2_18_0_MODEL_RUN_RESULTS_MOCKUP_ALIGNMENT.md") if (ROOT / "docs" / "V2_18_0_MODEL_RUN_RESULTS_MOCKUP_ALIGNMENT.md").exists() else ""

checks.append(check("package version", package.get("version") == VERSION, "package.json version is 2.18.0."))
checks.append(check("package lock version", package_lock.get("version") == VERSION and package_lock.get("packages", {}).get("", {}).get("version") == VERSION, "package-lock root version is 2.18.0."))
checks.append(check("tauri version", tauri.get("version") == VERSION, "Tauri app version is 2.18.0."))
checks.append(check("release artifact label", MILESTONE in package.get("scripts", {}).get("qpls:release:artifacts", ""), "Release artifact label points at v2.18."))
checks.append(check("npm scripts", all(script in package.get("scripts", {}) for script in ["qpls:v2180:model-run-results-smoke", "qpls:v2180:model-run-results-audit", "qpls:v2180:model-run-results"]), "v2.18 scripts are registered."))
checks.append(check("registry current stage", registry.get("current_stage") == MILESTONE, "Registry current_stage is v2.18."))

slice_entry = next((entry for entry in registry.get("slices", []) if entry.get("id") == MILESTONE), None)
checks.append(check("registry slice exists", slice_entry is not None, "v2.18 slice exists."))
checks.append(check("registry gates passed", bool(slice_entry) and all(gate.get("status") == "passed" for gate in slice_entry.get("gates", [])), "All v2.18 registry gates are passed."))
checks.append(check("roadmap expectation", MILESTONE in roadmap, "Roadmap test expects the v2.18 current stage."))
checks.append(check("topbar version mark", "v2.18 mockup alignment" in topbar, "Visible version mark is v2.18."))
checks.append(check("model marker", 'data-v218-mockup-screen="model"' in model and "model-v218-canvas-shell" in model, "Model screen has v2.18 marker and class."))
checks.append(check("run marker", 'data-v218-mockup-screen="run"' in run and "run-v218-workspace" in run, "Run screen has v2.18 marker and class."))
checks.append(check("results marker", 'data-v218-mockup-screen="results"' in results and "results-v218-workspace" in results, "Results screen has v2.18 marker and class."))
checks.append(check("css hooks", all(selector in styles for selector in [".model-v218-canvas-shell", ".run-v218-workspace", ".results-v218-workspace"]), "v2.18 CSS hooks exist."))
checks.append(check("docs updated", MILESTONE in active and MILESTONE in delivery and MILESTONE in ledger and MILESTONE in milestone_doc, "Active tracker, delivery status, ledger, and milestone doc reference v2.18."))
checks.append(check("smoke evidence exists", (ROOT / "validation" / "results" / "v2180_model_run_results_smoke.json").exists(), "Smoke result artifact exists."))
checks.append(check("no mojibake", not any(token in source for token in ["RÂ²", "RÃ"] for source in [model, run, results, styles]), "Touched UI sources render R² cleanly."))
checks.append(check("frontend boundary", not any(path in " ".join([model, run, results, styles]) for path in ["crates/qpls-estimation", "crates/qpls-assessment"]), "Audit scope remains frontend/product UI."))
checks.append(check("no equivalence claim", not re.search(r"SmartPLS equivalen", "\n".join([model, run, results, styles, milestone_doc]), re.I), "No SmartPLS equivalence claim is introduced."))

passed = all(entry["passed"] for entry in checks)
RESULT.parent.mkdir(parents=True, exist_ok=True)
RESULT.write_text(
    json.dumps(
        {
            "passed": passed,
            "milestone": MILESTONE,
            "version": VERSION,
            "generatedAt": datetime.now(timezone.utc).isoformat(),
            "checks": checks,
        },
        indent=2,
    ),
    encoding="utf-8",
)

if not passed:
    print(json.dumps({"passed": passed, "failed": [entry for entry in checks if not entry["passed"]]}, indent=2))
    raise SystemExit(1)

print(f"v2.18 Model/Run/Results audit passed: {RESULT}")
