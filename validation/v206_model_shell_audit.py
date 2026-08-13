import datetime
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "v206_model_shell_audit.json"
VERSION = "2.0.6"
MILESTONE = "v2_0_6_model_shell_sem_designer_surround"
QUICKPLS_PACKAGES = {
    "quickpls-desktop",
    "qpls-assessment",
    "qpls-cli",
    "qpls-core",
    "qpls-data",
    "qpls-estimation",
    "qpls-project",
    "qpls-resampling",
    "qpls-runner",
}


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def json_read(path: str):
    return json.loads(read(path))


def cargo_lock_versions() -> dict[str, str]:
    text = read("Cargo.lock")
    versions: dict[str, str] = {}
    for block in text.split("[[package]]"):
        name_match = re.search(r'^name = "([^"]+)"', block, flags=re.MULTILINE)
        version_match = re.search(r'^version = "([^"]+)"', block, flags=re.MULTILINE)
        if not name_match or not version_match:
            continue
        name = name_match.group(1)
        if name in QUICKPLS_PACKAGES:
            versions[name] = version_match.group(1)
    return versions


package = json_read("package.json")
package_lock = json_read("package-lock.json")
registry = json_read("validation/development_slices.json")
smoke = json_read("validation/results/v206_model_shell_smoke.json")
cargo = read("Cargo.toml")
tauri = json_read("src-tauri/tauri.conf.json")
roadmap = read("crates/qpls-core/src/roadmap.rs")
explorer = read("src/components/Explorer.tsx")
canvas = read("src/components/ModelCanvas.tsx")
inspector = read("src/components/Inspector.tsx")
styles = read("src/styles.css")
contract = read("docs/V2_UI_VISUAL_CONTRACT.md")
doc = read("docs/V2_0_6_MODEL_SHELL_SEM_DESIGNER_SURROUND.md")
delivery = read("docs/DELIVERY_STATUS.md")
ledger = read("docs/DEVELOPMENT_LEDGER.md")

slice_row = next((item for item in registry["slices"] if item["id"] == MILESTONE), None)
lock_versions = cargo_lock_versions()
combined_surface = "\n".join([explorer, canvas, inspector, styles, contract, doc, delivery, ledger])

checks = [
    ("smoke report passed", smoke.get("passed") is True),
    ("package version is 2.0.6", package.get("version") == VERSION),
    ("release artifact label is v2.0.6", MILESTONE in package["scripts"]["qpls:release:artifacts"]),
    ("package-lock root version is 2.0.6", package_lock.get("version") == VERSION and package_lock.get("packages", {}).get("", {}).get("version") == VERSION),
    ("Cargo workspace version is 2.0.6", f'version = "{VERSION}"' in cargo),
    ("Cargo.lock QuickPLS packages are 2.0.6", lock_versions.keys() == QUICKPLS_PACKAGES and all(version == VERSION for version in lock_versions.values())),
    ("Tauri version is 2.0.6", tauri.get("version") == VERSION),
    ("roadmap current stage expects v2.0.6", MILESTONE in roadmap),
    ("registry current stage is v2.0.6", registry.get("current_stage") == MILESTONE),
    ("registry v2.0.6 slice exists", slice_row is not None),
    ("registry v2.0.6 gates are all passed", slice_row is not None and all(gate.get("status") == "passed" for gate in slice_row.get("gates", []))),
    ("Explorer has v2 shell hooks", all(token in explorer for token in ["model-v2-explorer", "model-v2-status-card", "model-v2-guidance-card", "model-v2-tabs"])),
    ("Model canvas has v2 shell hooks", all(token in canvas for token in ["model-v2-canvas", "model-v2-toolbar"])),
    ("Inspector has v2 shell hook", "model-v2-inspector" in inspector),
    ("Styles define v2 model shell", all(token in styles for token in [".model-v2-explorer", ".model-v2-canvas", ".model-v2-toolbar", ".model-v2-inspector", ".workspace-shell:has(.model-v2-canvas)"])),
    ("Model surfaces render R² without mojibake", "R²" in canvas and "R²" in inspector and not re.search(r"RÂ|RÃ|R�|Ã‚|ï¿½", combined_surface)),
    ("Documentation records frontend-only boundary", all(phrase in doc for phrase in ["No statistical engines", "numerical fingerprints", "No SmartPLS equivalence claim"])),
    ("Delivery and ledger mention v2.0.6", MILESTONE in delivery and MILESTONE in ledger),
]

failures = [name for name, passed in checks if not passed]
result = {
    "passed": not failures,
    "milestone": MILESTONE,
    "checked_at": datetime.datetime.now(datetime.UTC).isoformat().replace("+00:00", "Z"),
    "checks": [{"name": name, "passed": bool(passed)} for name, passed in checks],
    "failures": failures,
    "quickpls_cargo_lock_versions": lock_versions,
}

RESULT.parent.mkdir(parents=True, exist_ok=True)
RESULT.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")

if failures:
    print(json.dumps(result, indent=2))
    raise SystemExit(1)

print(json.dumps(result, indent=2))
