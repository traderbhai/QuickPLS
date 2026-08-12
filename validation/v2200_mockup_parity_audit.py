from __future__ import annotations

import json
import re
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "v2200_mockup_parity_audit.json"
MILESTONE = "v2_20_0_quickpls_2_mockup_parity_release_audit"
VERSION = "2.20.0"
REQUIRED_PRIOR = [
    "v2_16_0_desktop_shell_visual_contract",
    "v2_17_0_home_data_setup_mockup_alignment",
    "v2_18_0_model_run_results_mockup_alignment",
    "v2_19_0_report_trust_settings_mockup_alignment",
]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def check(name: str, passed: bool, detail: str) -> dict[str, object]:
    return {"name": name, "passed": bool(passed), "detail": detail}


package = json.loads(read("package.json"))
package_lock = json.loads(read("package-lock.json"))
tauri = json.loads(read("src-tauri/tauri.conf.json"))
registry = json.loads(read("validation/development_slices.json"))
topbar = read("src/components/TopBar.tsx")
active = read("docs/V2_ACTIVE_MILESTONE.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")
roadmap = read("crates/qpls-core/src/roadmap.rs")
milestone_path = ROOT / "docs" / "V2_20_0_QUICKPLS_2_MOCKUP_PARITY_RELEASE_AUDIT.md"
milestone_doc = milestone_path.read_text(encoding="utf-8") if milestone_path.exists() else ""

all_ui_sources = "\n".join(
    read(path)
    for path in [
        "src/components/TopBar.tsx",
        "src/components/OnboardingWorkspace.tsx",
        "src/components/DataWorkspace.tsx",
        "src/components/AnalysisCatalog.tsx",
        "src/components/ModelCanvas.tsx",
        "src/components/RunWorkspace.tsx",
        "src/components/RunHistory.tsx",
        "src/components/ReportsWorkspace.tsx",
        "src/components/TrustCenterWorkspace.tsx",
        "src/components/SettingsWorkspace.tsx",
        "src/store.ts",
        "src/types.ts",
        "src/styles.css",
    ]
)

checks: list[dict[str, object]] = []
checks.append(check("package version", package.get("version") == VERSION, "package.json version is 2.20.0."))
checks.append(check("package lock version", package_lock.get("version") == VERSION and package_lock.get("packages", {}).get("", {}).get("version") == VERSION, "package-lock root version is 2.20.0."))
checks.append(check("tauri version", tauri.get("version") == VERSION, "Tauri app version is 2.20.0."))
checks.append(check("release artifact label", MILESTONE in package.get("scripts", {}).get("qpls:release:artifacts", ""), "Release artifact label points at v2.20 for any future artifact build."))
checks.append(check("npm scripts", all(script in package.get("scripts", {}) for script in ["qpls:v2200:mockup-parity-smoke", "qpls:v2200:mockup-parity-audit", "qpls:v2200:mockup-parity"]), "v2.20 scripts are registered."))
checks.append(check("registry current stage", registry.get("current_stage") == MILESTONE, "Registry current_stage is v2.20."))

slice_entry = next((entry for entry in registry.get("slices", []) if entry.get("id") == MILESTONE), None)
checks.append(check("registry slice exists", slice_entry is not None, "v2.20 slice exists."))
checks.append(check("registry gates passed", bool(slice_entry) and all(gate.get("status") == "passed" for gate in slice_entry.get("gates", [])), "All v2.20 registry gates are passed."))
slice_ids = {entry.get("id"): entry for entry in registry.get("slices", [])}
checks.append(check("prior gates remain clear", all(slice_ids.get(mid, {}).get("status") == "validated" and all(g.get("status") == "passed" for g in slice_ids.get(mid, {}).get("gates", [])) for mid in REQUIRED_PRIOR), "v2.16 through v2.19 remain validated."))
checks.append(check("roadmap expectation", MILESTONE in roadmap, "Roadmap test expects the v2.20 current stage."))
checks.append(check("topbar version mark", "v2.20 mockup parity" in topbar, "Visible version mark is v2.20."))

markers = [
    'data-v216-desktop-shell="menu-bar"',
    'data-v217-mockup-screen="home"',
    'data-v217-mockup-screen="data"',
    'data-v217-mockup-screen="setup"',
    'data-v218-mockup-screen="model"',
    'data-v218-mockup-screen="run"',
    'data-v218-mockup-screen="results"',
    'data-v219-mockup-screen="report"',
    'data-v219-mockup-screen="trust"',
    'data-v219-mockup-screen="settings"',
]
checks.append(check("screen coverage markers", all(marker in all_ui_sources for marker in markers), "All mockup target screens expose milestone markers."))
checks.append(check("desktop dialog coverage", all(dialog_id in topbar for dialog_id in ["new_project", "import_data", "calculation_setup", "export_options", "settings", "help_shortcuts"]), "Required desktop-style dialogs are present."))
checks.append(check("docs updated", MILESTONE in active and MILESTONE in delivery and MILESTONE in ledger and MILESTONE in milestone_doc, "Active tracker, delivery status, ledger, and milestone doc reference v2.20."))
checks.append(check("smoke evidence exists", (ROOT / "validation" / "results" / "v2200_mockup_parity_smoke.json").exists(), "Smoke result artifact exists."))
checks.append(check("no mojibake or stale copy", not any(token in all_ui_sources for token in ["RÃ", "RÂ", "Validation fixture"]), "Mockup-aligned UI sources avoid known text corruption and stale fixture copy."))
checks.append(check("no equivalence claim", not re.search(r"SmartPLS[- ]equivalent|equivalent to SmartPLS", all_ui_sources + "\n" + milestone_doc, re.I), "No SmartPLS equivalence claim is introduced."))
checks.append(check("frontend boundary", "F_ml =" not in all_ui_sources and "crates/qpls-estimation" not in all_ui_sources, "Parity pass remains frontend/product-only."))
checks.append(check("artifact policy", "No versioned desktop artifacts were created" in milestone_doc, "Intermediate artifact policy is documented."))

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

print(f"v2.20 mockup parity audit passed: {RESULT}")
