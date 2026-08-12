import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
MILESTONE = "v2_16_0_desktop_shell_visual_contract"
VERSION = "2.16.0"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def load_json(path: str):
    return json.loads(read(path))


checks: list[tuple[str, bool, str]] = []

package = load_json("package.json")
registry = load_json("validation/development_slices.json")
tauri = load_json("src-tauri/tauri.conf.json")
topbar = read("src/components/TopBar.tsx")
styles = read("src/styles.css")
store = read("src/store.ts")
roadmap = read("crates/qpls-core/src/roadmap.rs")
active_doc = read("docs/V2_ACTIVE_MILESTONE.md")
milestone_doc = read("docs/V2_16_0_DESKTOP_SHELL_VISUAL_CONTRACT.md")
smoke_path = RESULTS / "v2160_desktop_shell_smoke.json"
smoke = json.loads(smoke_path.read_text(encoding="utf-8")) if smoke_path.exists() else {}

checks.append(("package version", package.get("version") == VERSION, "package.json must use 2.16.0"))
checks.append(("tauri version", tauri.get("version") == VERSION, "src-tauri/tauri.conf.json must use 2.16.0"))
checks.append(("release label", MILESTONE in package["scripts"].get("qpls:release:artifacts", ""), "release artifact label must be v2.16"))
checks.append(("scripts present", all(key in package["scripts"] for key in ["qpls:v2160:desktop-shell-smoke", "qpls:v2160:desktop-shell-audit", "qpls:v2160:desktop-shell"]), "v2.16 npm scripts must exist"))
checks.append(("registry current stage", registry.get("current_stage") == MILESTONE, "registry current_stage must be v2.16"))
checks.append(("registry slice", any(item.get("id") == MILESTONE and item.get("status") == "validated" for item in registry.get("slices", [])), "registry slice must be validated"))
checks.append(("roadmap expected stage", MILESTONE in roadmap, "roadmap test must expect v2.16"))
checks.append(("active milestone doc", MILESTONE in active_doc and "v2_17_0_home_data_setup_mockup_alignment" in active_doc, "active milestone doc must record v2.16 and next v2.17"))
checks.append(("milestone doc", "frontend-only" in milestone_doc and "React-rendered desktop menu bar" in milestone_doc, "milestone doc must describe scope"))
checks.append(("smoke passed", smoke.get("passed") is True, "smoke result must pass"))
checks.append(("desktop shell markers", all(marker in topbar for marker in ["data-v216-desktop-shell=\"title-strip\"", "data-v216-desktop-shell=\"menu-bar\"", "data-v216-desktop-shell=\"command-strip\""]), "TopBar shell markers must exist"))
checks.append(("dialog visual contract", ".desktop-dialog-backdrop" in styles and ".desktop-dialog-card" in styles, "dialog CSS must exist"))
checks.append(("frontend-only state", "activeDesktopDialog" in store and "activeDesktopMenu" in store, "desktop menu/dialog state must be UI-only store state"))
checks.append(("no mojibake", "RÂ²" not in topbar + styles + milestone_doc, "R² mojibake must not appear"))
checks.append(("no SmartPLS equivalence claim", "SmartPLS-equivalence" not in topbar and "equivalent to SmartPLS" not in topbar.lower(), "shell must not claim SmartPLS equivalence"))

failures = [{"check": name, "detail": detail} for name, passed, detail in checks if not passed]
result = {
    "passed": not failures,
    "milestone": MILESTONE,
    "version": VERSION,
    "checks": {name: passed for name, passed, _ in checks},
    "failures": failures,
}

RESULTS.mkdir(parents=True, exist_ok=True)
(RESULTS / "v2160_desktop_shell_audit.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
if failures:
    print(json.dumps(result, indent=2))
    raise SystemExit(1)
print("v2.16 desktop shell audit passed")
