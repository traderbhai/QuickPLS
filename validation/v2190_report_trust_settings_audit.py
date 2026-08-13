from __future__ import annotations

import json
import re
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "v2190_report_trust_settings_audit.json"
MILESTONE = "v2_19_0_report_trust_settings_mockup_alignment"
VERSION = "2.19.0"


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
report = read("src/components/ReportsWorkspace.tsx")
trust = read("src/components/TrustCenterWorkspace.tsx")
settings = read("src/components/SettingsWorkspace.tsx")
styles = read("src/styles.css")
roadmap = read("crates/qpls-core/src/roadmap.rs")
active = read("docs/V2_ACTIVE_MILESTONE.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")
milestone_path = ROOT / "docs" / "V2_19_0_REPORT_TRUST_SETTINGS_MOCKUP_ALIGNMENT.md"
milestone_doc = milestone_path.read_text(encoding="utf-8") if milestone_path.exists() else ""

checks.append(check("package version", package.get("version") == VERSION, "package.json version is 2.19.0."))
checks.append(check("package lock version", package_lock.get("version") == VERSION and package_lock.get("packages", {}).get("", {}).get("version") == VERSION, "package-lock root version is 2.19.0."))
checks.append(check("tauri version", tauri.get("version") == VERSION, "Tauri app version is 2.19.0."))
checks.append(check("release artifact label", MILESTONE in package.get("scripts", {}).get("qpls:release:artifacts", ""), "Release artifact label points at v2.19."))
checks.append(check("npm scripts", all(script in package.get("scripts", {}) for script in ["qpls:v2190:report-trust-settings-smoke", "qpls:v2190:report-trust-settings-audit", "qpls:v2190:report-trust-settings"]), "v2.19 scripts are registered."))
checks.append(check("registry current stage", registry.get("current_stage") == MILESTONE, "Registry current_stage is v2.19."))

slice_entry = next((entry for entry in registry.get("slices", []) if entry.get("id") == MILESTONE), None)
checks.append(check("registry slice exists", slice_entry is not None, "v2.19 slice exists."))
checks.append(check("registry gates passed", bool(slice_entry) and all(gate.get("status") == "passed" for gate in slice_entry.get("gates", [])), "All v2.19 registry gates are passed."))
checks.append(check("roadmap expectation", MILESTONE in roadmap, "Roadmap test expects the v2.19 current stage."))
checks.append(check("topbar version mark", "v2.19 mockup alignment" in topbar, "Visible version mark is v2.19."))
checks.append(check("report marker", 'data-v219-mockup-screen="report"' in report and "report-v219-workspace" in report, "Report screen has v2.19 marker and class."))
checks.append(check("trust marker", 'data-v219-mockup-screen="trust"' in trust and "trust-v219-workspace" in trust, "Trust Center screen has v2.19 marker and class."))
checks.append(check("settings marker", 'data-v219-mockup-screen="settings"' in settings and "settings-v219-workspace" in settings, "Settings screen has v2.19 marker and class."))
checks.append(check("css hooks", all(selector in styles for selector in [".report-v219-workspace", ".trust-v219-workspace", ".settings-v219-workspace"]), "v2.19 CSS hooks exist."))
checks.append(check("docs updated", MILESTONE in active and MILESTONE in delivery and MILESTONE in ledger and MILESTONE in milestone_doc, "Active tracker, delivery status, ledger, and milestone doc reference v2.19."))
checks.append(check("smoke evidence exists", (ROOT / "validation" / "results" / "v2190_report_trust_settings_smoke.json").exists(), "Smoke result artifact exists."))
checks.append(check("no mojibake", not any(token in source for token in ["RÃ", "RÂ"] for source in [report, trust, settings, styles]), "Touched UI sources render R2 text cleanly."))
checks.append(check("frontend boundary", not any(path in " ".join([report, trust, settings, styles]) for path in ["crates/qpls-estimation", "crates/qpls-assessment"]), "Audit scope remains frontend/product UI."))
checks.append(check("no equivalence claim", not re.search(r"SmartPLS[- ]equivalent|equivalent to SmartPLS", "\n".join([report, trust, settings, styles, milestone_doc]), re.I), "No SmartPLS equivalence claim is introduced."))

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

print(f"v2.19 Report/Trust/Settings audit passed: {RESULT}")
